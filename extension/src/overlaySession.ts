import * as vscode from "vscode";
import WebSocket from "ws";
import {
  LightningGitClient,
  type Comment,
  type MergeConflict,
  type ProjectMember,
} from "./client";
import { ConflictPanel } from "./conflictPanel";
import { conflictsEqual } from "./conflictsEqual";
import type { WsMessage } from "./parseWsMessage";
import { colorForUser, initialsFor, withAlpha } from "./palette";

type TeammateChange = {
  content: string;
  lines: [number, number];
  lastUpdate: number;
};

export class OverlaySession {
  private ws: WebSocket | undefined;
  private readonly disposables: vscode.Disposable[] = [];
  private debounceTimer: ReturnType<typeof setTimeout> | undefined;
  private peekHideTimer: ReturnType<typeof setTimeout> | undefined;
  private conflictPeekHideTimer: ReturnType<typeof setTimeout> | undefined;

  private activeFile: string | undefined;
  private activeBranch: string | undefined;
  private pendingPath: string | undefined;
  private readonly teammateChanges = new Map<string, TeammateChange>();
  // current conflict set, pushed whole by the server on each "conflicts"
  // message over the overlay channel. the backend now owns conflict
  // detection (committed branches + live overlays); the client just renders.
  private conflicts: MergeConflict[] = [];
  // last set actually painted to status bar / gutter / panel. used as the
  // repaint guard so an identical "conflicts" message doesnt re-issue
  // setDecorations + webview html and saturate the IPC channel.
  private lastRendered: MergeConflict[] = [];
  private comments: Comment[] = [];
  // user_id -> display_name, refreshed when the session starts and again
  // whenever an author we have no name for shows up. fallback to a short uuid
  // prefix for users still not in the cache.
  private members = new Map<string, string>();
  // guards for the lazy name resolution in maybeResolveUnknownAuthor.
  private pendingMemberRefresh = false;
  private readonly resolveAttempted = new Set<string>();

  private readonly statusBarItem: vscode.StatusBarItem;
  private readonly conflictStatusBarItem: vscode.StatusBarItem;

  // Per-teammate line tints, created lazily and keyed by colour so the same
  // teammate keeps a stable colour across repaints. The display-name label
  // rides on the first changed line as an instance decoration; the raw content
  // never gets dumped into the editor, it lives in the hover and the full diff.
  private readonly teammateDecorations = new Map<string, vscode.TextEditorDecorationType>();

  private readonly conflictDecoration = vscode.window.createTextEditorDecorationType({
    isWholeLine: true,
    backgroundColor: "rgba(255, 140, 0, 0.08)",
    borderWidth: "0 0 0 3px",
    borderStyle: "solid",
    borderColor: "#ff8c00",
    overviewRulerColor: "#ff8c00",
    overviewRulerLane: vscode.OverviewRulerLane.Left,
    after: {
      margin: "0 0 0 1em",
      color: "#ff8c0099",
      fontStyle: "italic",
    },
  });

  // An inline badge on the first line of each conflict region, naming the
  // branches that disagree, so the editor shows what the conflict is rather
  // than only a coloured border. The colour is set per range.
  private readonly conflictLabelDecoration = vscode.window.createTextEditorDecorationType({
    after: {
      margin: "0 0 0 1.5em",
      fontStyle: "italic",
    },
  });

  private readonly commentDecoration = vscode.window.createTextEditorDecorationType({
    after: {
      margin: "0 0 0 2em",
      color: "#8ab4f888",
      fontStyle: "italic",
    },
    overviewRulerColor: "#8ab4f8",
    overviewRulerLane: vscode.OverviewRulerLane.Right,
  });

  private showingPeek = false;

  constructor(
    private readonly client: LightningGitClient,
    private readonly projectId: string,
    private readonly userId: string,
    private readonly debounceMs: number,
  ) {
    this.statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
    this.statusBarItem.command = "lightning-git.peekTeammates";
    this.updateStatusBar();
    this.statusBarItem.show();

    this.conflictStatusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 99);
    this.conflictStatusBarItem.command = "lightning-git.peekConflicts";
    this.updateConflictStatusBar();
    this.conflictStatusBarItem.show();
  }

  async start(): Promise<void> {
    void this.refreshMembers();

    this.disposables.push(
      vscode.commands.registerCommand("lightning-git.peekTeammates", () => {
        this.togglePeek();
      }),
    );

    this.disposables.push(
      vscode.commands.registerCommand("lightning-git.peekConflicts", () => {
        this.toggleConflictPeek();
      }),
    );

    this.disposables.push(
      vscode.languages.registerHoverProvider({ scheme: "file" }, {
        provideHover: (document, position) => this.provideLineHover(document, position),
      }),
    );

    this.disposables.push(
      vscode.window.onDidChangeActiveTextEditor((editor) => {
        this.hidePeek();

        if (!editor) {
          return;
        }

        this.openDocumentOverlay(editor.document)
          .then(() => this.renderCommentDecorations())
          .catch((error: unknown) => {
            const message = error instanceof Error ? error.message : String(error);
            void vscode.window.showErrorMessage(`Failed to connect overlay: ${message}`);
          });
      }),
    );

    // untitled docs become real files on save without firing
    // onDidChangeActiveTextEditor (the editor instance is the same, only the
    // URI flips from "untitled:..." to "file:..."). hook save so the new file
    // gets an overlay registered exactly like one opened from the tree.
    this.disposables.push(
      vscode.workspace.onDidSaveTextDocument((document) => {
        if (document.uri.scheme !== "file") return;
        this.openDocumentOverlay(document).catch((error: unknown) => {
          console.error("Lightning Git: openDocumentOverlay failed on save", error);
        });
      }),
    );

    this.disposables.push(
      vscode.workspace.onDidChangeTextDocument((event) => {
        if (event.document !== vscode.window.activeTextEditor?.document) {
          return;
        }

        if (this.showingPeek) {
          this.hidePeek();
        }

        this.queueDocumentChange(event.document);
      }),
    );

    const editor = vscode.window.activeTextEditor;

    if (editor) {
      await this.openDocumentOverlay(editor.document);
    }
  }

  getActiveUsers(): string[] {
    return Array.from(this.teammateChanges.keys());
  }

  // Self plus every teammate currently editing this file, used for stable
  // per-user colour assignment.
  private knownUserIds(): string[] {
    return [this.userId, ...this.teammateChanges.keys()];
  }

  private getTeammateDecoration(color: string): vscode.TextEditorDecorationType {
    let deco = this.teammateDecorations.get(color);
    if (!deco) {
      deco = vscode.window.createTextEditorDecorationType({
        isWholeLine: true,
        backgroundColor: withAlpha(color, 30),
        overviewRulerColor: color,
        overviewRulerLane: vscode.OverviewRulerLane.Left,
      });
      this.teammateDecorations.set(color, deco);
    }
    return deco;
  }

  private clearTeammateDecorations(editor: vscode.TextEditor | undefined): void {
    if (!editor) {
      return;
    }
    for (const deco of this.teammateDecorations.values()) {
      editor.setDecorations(deco, []);
    }
  }

  getOtherUserChange(userId: string): TeammateChange | undefined {
    return this.teammateChanges.get(userId);
  }

  getConflicts(): MergeConflict[] {
    return this.conflicts;
  }

  dispose(): void {
    this.ws?.close();
    this.ws = undefined;

    this.stopRenderTimer();

    if (this.debounceTimer) {
      clearTimeout(this.debounceTimer);
      this.debounceTimer = undefined;
    }

    if (this.peekHideTimer) {
      clearTimeout(this.peekHideTimer);
      this.peekHideTimer = undefined;
    }

    if (this.conflictPeekHideTimer) {
      clearTimeout(this.conflictPeekHideTimer);
      this.conflictPeekHideTimer = undefined;
    }

    this.hidePeek();

    for (const disposable of this.disposables) {
      disposable.dispose();
    }

    this.disposables.length = 0;
    this.teammateChanges.clear();
    this.conflicts = [];
    this.lastRendered = [];
    this.comments = [];

    this.conflictPanel?.dispose();
    this.conflictPanel = undefined;

    this.statusBarItem.dispose();
    this.conflictStatusBarItem.dispose();
    for (const deco of this.teammateDecorations.values()) {
      deco.dispose();
    }
    this.teammateDecorations.clear();
    this.conflictDecoration.dispose();
    this.conflictLabelDecoration.dispose();
    this.commentDecoration.dispose();
  }

  private async openDocumentOverlay(document: vscode.TextDocument): Promise<void> {
    const workspaceFolder = vscode.workspace.getWorkspaceFolder(document.uri);

    if (!workspaceFolder) {
      return;
    }

    const relativePath = vscode.workspace.asRelativePath(document.uri, false);
    const branch = await this.getCurrentBranch(workspaceFolder.uri.fsPath);

    if (this.activeFile === relativePath && this.activeBranch === branch) {
      return;
    }
    // when an untitled doc gets saved, both onDidSaveTextDocument and
    // onDidChangeActiveTextEditor can fire for the same path; the second one
    // would close the WS that the first one just opened. drop concurrent opens
    // of the same path here.
    if (this.pendingPath === relativePath) {
      return;
    }
    this.pendingPath = relativePath;

    try {
      this.ws?.close();

      this.hidePeek();

      this.activeFile = relativePath;
      this.activeBranch = branch;
      this.teammateChanges.clear();
      this.conflicts = [];
      this.lastRendered = [];
      this.comments = [];

      this.updateStatusBar();
      this.updateConflictStatusBar();

      await this.client.createOverlay(this.projectId, this.userId, branch, relativePath);

      const wsUrl = await this.client.getOverlayWsUrl(this.projectId, this.userId, relativePath);
      await this.connectWebSocket(wsUrl, relativePath);
    } finally {
      if (this.pendingPath === relativePath) {
        this.pendingPath = undefined;
      }
    }
  }

  private connectWebSocket(wsUrl: string, relativePath: string): Promise<void> {
    return new Promise((resolve, reject) => {
      const ws = new WebSocket(wsUrl);
      this.ws = ws;

      ws.on("open", () => {
        console.log(`Lightning Git overlay connected for ${relativePath}`);
        resolve();
      });

      ws.on("message", (data) => {
        this.handleIncomingMessage(data.toString());
      });

      ws.on("error", (error) => {
        reject(error);
      });

      ws.on("close", (code: number, reason: Buffer) => {
        const reasonText = reason.toString();
        console.log(
          `Lightning Git overlay closed (${code})${reasonText ? `: ${reasonText}` : ""}`,
        );
      });
    });
  }

  private queueDocumentChange(document: vscode.TextDocument): void {
    if (this.debounceTimer) {
      clearTimeout(this.debounceTimer);
    }

    this.debounceTimer = setTimeout(() => {
      this.sendDocumentChange(document);
    }, this.debounceMs);
  }

  private sendDocumentChange(document: vscode.TextDocument): void {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      return;
    }

    const message: WsMessage = {
      kind: "overlay",
      user_id: this.userId,
      content: document.getText(),
      line_section: [0, Math.max(document.lineCount - 1, 0)],
    };

    this.ws.send(JSON.stringify(message));
  }

  private handleIncomingMessage(raw: string): void {
    let msg: WsMessage;
    try {
      msg = JSON.parse(raw) as WsMessage;
    } catch {
      return;
    }
    switch (msg.kind) {
      case "snapshot":
        this.comments = msg.comments;
        this.renderCommentDecorations();
        // mirror frontend OverlayView.vue case "snapshot": seed teammate
        // overlays from the snapshot so a late joiner sees existing live
        // edits immediately, instead of waiting for the other user's
        // next keystroke to repopulate teammateChanges. clear first since
        // a WS reconnect resends the snapshot as the new truth. conflicts
        // arrive separately via the server's "conflicts" message.
        this.teammateChanges.clear();
        for (const u of msg.all_user_contents) {
          if (u.user_id === this.userId) continue;
          this.teammateChanges.set(u.user_id, {
            content: u.content,
            lines: u.edited_sections,
            lastUpdate: Date.now(),
          });
          this.maybeResolveUnknownAuthor(u.user_id);
        }
        for (const c of msg.comments) {
          this.maybeResolveUnknownAuthor(c.user_id);
        }
        this.updateStatusBar();
        return;
      case "conflicts":
        // whole-set replace: the backend owns conflict detection and pushes
        // the full conflict list for this file on every recompute.
        this.conflicts = msg.conflicts;
        this.renderConflicts();
        return;
      case "comment_created":
        // server-assigned id might already be in our list if the broadcast
        // races a local optimistic insert; idempotent upsert by id.
        this.comments = [
          ...this.comments.filter((c) => c.id !== msg.id),
          { id: msg.id, user_id: msg.user_id, line: msg.line, text: msg.text, created_at: msg.created_at },
        ];
        this.maybeResolveUnknownAuthor(msg.user_id);
        this.renderCommentDecorations();
        this.notifyNewComment({
          id: msg.id,
          user_id: msg.user_id,
          line: msg.line,
          text: msg.text,
          created_at: msg.created_at,
        });
        return;
      case "comment_deleted":
        this.comments = this.comments.filter((c) => c.id !== msg.id);
        this.renderCommentDecorations();
        return;
      case "overlay":
        if (msg.user_id === this.userId) return;
        this.teammateChanges.set(msg.user_id, {
          content: msg.content,
          lines: msg.line_section,
          lastUpdate: Date.now(),
        });
        this.maybeResolveUnknownAuthor(msg.user_id);
        this.updateStatusBar();
        this.flashStatusBar();
        return;
    }
  }

  getActiveFile(): string | undefined {
    return this.activeFile;
  }

  getCurrentUserId(): string {
    return this.userId;
  }

  authorLabelFor(userId: string): string {
    return this.authorLabel(userId);
  }

  private togglePeek(): void {
    if (this.showingPeek) {
      this.hidePeek();
      return;
    }

    this.showPeek();
  }

  private showPeek(): void {
    // resolve by tracked file, not focus. when the conflict panel webview
    // has focus, activeTextEditor is undefined and clicking "editing" did
    // nothing, same bug pattern that bit applyConflictGutter earlier.
    const editor = this.findVisibleEditor();
    if (!editor || this.teammateChanges.size === 0) {
      return;
    }

    // clear any prior peek so a teammate who stopped editing leaves no stale
    // tint behind.
    this.clearTeammateDecorations(editor);

    const known = this.knownUserIds();
    const editorLineCount = editor.document.lineCount;
    // group ranges per colour so one decoration type carries one teammate's
    // tint; the label (display name, not a uuid, and no quoted content) sits
    // on their first changed line. the actual text lives in the hover and the
    // full diff.
    const byColor = new Map<string, vscode.DecorationOptions[]>();

    for (const [userId, teammateChange] of this.teammateChanges) {
      const color = colorForUser(userId, known);
      const name = this.authorLabel(userId);
      const contentLines = teammateChange.content.split("\n");
      // strip trailing empty from a possible trailing newline so we dont
      // tint a phantom blank line.
      if (contentLines.length > 0 && contentLines[contentLines.length - 1] === "") {
        contentLines.pop();
      }

      const extraLines = Math.max(0, contentLines.length - editorLineCount);
      const label = (extras: number): string =>
        `  ${initialsFor(name)}  ${name}${extras > 0 ? `  +${extras} new line${extras === 1 ? "" : "s"}` : ""}`;

      const opts: vscode.DecorationOptions[] = [];
      // walk every comparable line; tint the ones that differ from our copy,
      // and put the label on the first changed line only.
      const compareEnd = Math.min(contentLines.length, editorLineCount);
      let labelled = false;
      for (let line = 0; line < compareEnd; line++) {
        if (contentLines[line] === editor.document.lineAt(line).text) {
          continue;
        }
        const range = new vscode.Range(line, 0, line, editor.document.lineAt(line).text.length);
        if (labelled) {
          opts.push({ range });
        } else {
          labelled = true;
          opts.push({
            range,
            renderOptions: { after: { contentText: label(extraLines), color, fontStyle: "italic" } },
          });
        }
      }

      // teammate only appended lines past the end of our copy: surface them
      // with a single labelled tint on the last line rather than dumping the
      // appended text inline.
      if (!labelled && extraLines > 0 && editorLineCount > 0) {
        const lastLine = editorLineCount - 1;
        opts.push({
          range: new vscode.Range(lastLine, 0, lastLine, editor.document.lineAt(lastLine).text.length),
          renderOptions: { after: { contentText: label(extraLines), color, fontStyle: "italic" } },
        });
      }

      if (opts.length > 0) {
        byColor.set(color, (byColor.get(color) ?? []).concat(opts));
      }
    }

    for (const [color, opts] of byColor) {
      editor.setDecorations(this.getTeammateDecoration(color), opts);
    }
    this.showingPeek = true;

    if (this.peekHideTimer) {
      clearTimeout(this.peekHideTimer);
    }

    this.peekHideTimer = setTimeout(() => {
      this.hidePeek();
    }, 6000);
  }

  private hidePeek(): void {
    this.clearTeammateDecorations(this.findVisibleEditor());
    this.showingPeek = false;
  }

  // Visible editor for the tracked active file. Used by peek/gutter paths
  // that otherwise silently no-op when the conflict panel webview steals
  // focus from the editor.
  private findVisibleEditor(): vscode.TextEditor | undefined {
    if (!this.activeFile) return undefined;
    return vscode.window.visibleTextEditors.find(
      (e) =>
        e.document.uri.scheme === "file"
        && vscode.workspace.asRelativePath(e.document.uri, false) === this.activeFile,
    );
  }

  private updateStatusBar(): void {
    const activeUsers = this.teammateChanges.size;

    if (activeUsers === 0) {
      this.statusBarItem.text = "$(circle-outline) No teammates";
      this.statusBarItem.tooltip = "No active teammates on this file";
      this.statusBarItem.color = undefined;
      return;
    }

    const users = Array.from(this.teammateChanges.keys())
      .map((id) => this.authorLabel(id))
      .join(", ");

    this.statusBarItem.text = `$(pulse) ${activeUsers} editing`;
    this.statusBarItem.tooltip = `${users} editing this file. Click to peek their changes.`;
    this.statusBarItem.color = "#4a9eff";
  }

  private flashStatusBar(): void {
    this.statusBarItem.backgroundColor = new vscode.ThemeColor("statusBarItem.warningBackground");

    setTimeout(() => {
      this.statusBarItem.backgroundColor = undefined;
    }, 300);
  }

  private updateConflictStatusBar(): void {
    if (this.conflicts.length === 0) {
      this.conflictStatusBarItem.text = "$(check) No conflicts";
      this.conflictStatusBarItem.tooltip = "No predicted merge conflicts on this file";
      this.conflictStatusBarItem.color = undefined;
      return;
    }

    const branchCounts = new Map<string, number>();

    for (const conflict of this.conflicts) {
      for (const hunk of conflict.hunks) {
        branchCounts.set(hunk.branch, (branchCounts.get(hunk.branch) ?? 0) + 1);
      }
    }

    const summary = Array.from(branchCounts.entries())
      .map(([branch, count]) => `  ${branch}: ${count} region${count === 1 ? "" : "s"}`)
      .join("\n");

    const conflictCount = this.conflicts.length;

    this.conflictStatusBarItem.text = `$(alert) ${conflictCount} conflict${conflictCount === 1 ? "" : "s"}`;
    this.conflictStatusBarItem.tooltip = `Predicted merge conflicts:\n${summary}\n\nClick to peek`;
    this.conflictStatusBarItem.color = "#ff8c00";
  }

  private conflictPanel: ConflictPanel | undefined;

  private toggleConflictPeek(): void {
    if (this.conflicts.length === 0) {
      void vscode.window.showInformationMessage("No predicted merge conflicts on this file.");
      return;
    }
    // reuse the live panel if it exists and hasn't been disposed: reveal it
    // and repaint. recreating on every click raced the async webview dispose
    // against update(), which is what left the reopened tab stale/empty. only
    // build a fresh panel once the old one is genuinely gone (isDisposed()).
    if (this.conflictPanel && !this.conflictPanel.isDisposed()) {
      this.conflictPanel.reveal();
      this.conflictPanel.update(this.conflicts);
      return;
    }
    this.conflictPanel = new ConflictPanel(this, () => {
      this.conflictPanel = undefined;
    });
    this.conflictPanel.update(this.conflicts);
  }

  // coalesce rapid renders. without this, a burst of "conflicts" messages
  // (server recomputing as several teammates type) fires a sync render per
  // message that touches status bar + decorations (+ webview html if the
  // panel is open), saturating the IPC channel between the extension host
  // and the renderer process. two users typing simultaneously was enough to
  // make VSC stop responding to commands like "open conflict panel". 60ms
  // is short enough to feel instant.
  private renderTimer: ReturnType<typeof setTimeout> | undefined;
  private renderConflicts(): void {
    if (this.renderTimer) return;
    this.renderTimer = setTimeout(() => {
      this.renderTimer = undefined;
      try {
        // dont re-issue setDecorations / webview html unless the conflict
        // set actually changed since the last paint; vscode's IPC stays
        // unblocked when nothing is happening.
        if (conflictsEqual(this.lastRendered, this.conflicts)) return;
        this.lastRendered = this.conflicts;
        this.updateConflictStatusBar();
        this.applyConflictGutter();
        this.conflictPanel?.update(this.conflicts);
      } catch (err) {
        console.error("Lightning Git: render failed", err);
      }
    }, 60);
  }

  /// Subtle left-border + overview ruler tick on every conflict line so the
  /// user has a visual cue without the cramped inline-label decoration the
  /// old peek used. The full comparison lives in the markdown doc opened via
  /// the status-bar command.
  private applyConflictGutter(): void {
    if (!this.activeFile) return;
    // visibleTextEditors holds every editor currently shown in the window,
    // including ones that arent the active focused tab. apply decorations
    // there so opening the conflict panel webview (which steals focus and
    // nullifies activeTextEditor) doesnt wipe the highlights.
    const editors = vscode.window.visibleTextEditors.filter(
      (e) =>
        e.document.uri.scheme === "file"
        && vscode.workspace.asRelativePath(e.document.uri, false) === this.activeFile,
    );
    if (editors.length === 0) return;
    for (const editor of editors) {
      const decorations: vscode.DecorationOptions[] = [];
      const labels: vscode.DecorationOptions[] = [];
      for (const c of this.conflicts) {
        const safeStart = Math.max(0, Math.min(c.base_start, editor.document.lineCount - 1));
        // for a pure-insert conflict (base_start === base_end, no base lines
        // consumed), the naive base_end - 1 is < safeStart and the loop
        // skips. clamp with Math.max(safeStart, …) so we always highlight
        // the insertion point itself, without this, overlay-vs-overlay
        // inserts had zero decorations until the 60s poll surfaced an
        // unrelated committed-branch conflict at a non-insert range.
        const rawEnd = Math.min(c.base_end - 1, editor.document.lineCount - 1);
        const safeEnd = Math.max(safeStart, rawEnd);
        for (let line = safeStart; line <= safeEnd; line++) {
          decorations.push({
            range: new vscode.Range(line, 0, line, editor.document.lineAt(line).text.length),
          });
        }
        // badge on the first line of the region naming the branches that
        // disagree, so the editor shows what the conflict is, not just a border.
        labels.push({
          range: new vscode.Range(safeStart, 0, safeStart, editor.document.lineAt(safeStart).text.length),
          renderOptions: { after: { contentText: `  ${this.conflictBadgeText(c)}`, color: "#ff8c00", fontStyle: "italic" } },
        });
      }
      editor.setDecorations(this.conflictDecoration, decorations);
      editor.setDecorations(this.conflictLabelDecoration, labels);
    }
  }

  private stopRenderTimer(): void {
    if (this.renderTimer) {
      clearTimeout(this.renderTimer);
      this.renderTimer = undefined;
    }
  }

  /// A teammate created a comment, surface it as a popup since the gutter
  /// decoration alone is easy to miss.
  private notifyNewComment(c: Comment): void {
    if (c.user_id === this.userId) return;
    const author = this.authorLabel(c.user_id);
    const preview = this.truncate(c.text, 80);
    void vscode.window.showInformationMessage(
      `New comment on line ${c.line} by ${author}: ${preview}`,
    );
  }

  /// Refreshes the cached project members map and repaints anything that shows
  /// an author, so a uuid fallback flips to the real name as soon as the names
  /// arrive. Best-effort: a failure leaves the cache untouched.
  private async refreshMembers(): Promise<void> {
    try {
      const list = await this.client.listProjectMembers(this.projectId);
      this.members = new Map(
        list
          .filter((m: ProjectMember) => Boolean(m.display_name && m.display_name.trim().length > 0))
          .map((m) => [m.id, m.display_name] as const),
      );
    } catch {
      return;
    }
    // names may have just become available: repaint the status bar, comment
    // decorations, the conflict gutter/panel, and the peek if it is showing,
    // so anything painted with a uuid fallback flips to the real name.
    this.updateStatusBar();
    this.renderCommentDecorations();
    this.lastRendered = [];
    this.renderConflicts();
    if (this.showingPeek) {
      this.showPeek();
    }
  }

  // When an author id appears that we have no name for (a teammate who joined
  // after the session started, say), refetch the member list once so the uuid
  // fallback can resolve. Guarded so an id we already failed to resolve, or a
  // refetch already in flight, does not spam the endpoint.
  private maybeResolveUnknownAuthor(userId: string): void {
    if (
      userId === this.userId
      || this.members.has(userId)
      || this.resolveAttempted.has(userId)
      || this.pendingMemberRefresh
    ) {
      return;
    }
    this.resolveAttempted.add(userId);
    this.pendingMemberRefresh = true;
    void this.refreshMembers().finally(() => {
      this.pendingMemberRefresh = false;
    });
  }

  /// Returns the rendered author name for a comment: "you" for the caller,
  /// the cached display name otherwise, or a short uuid prefix as fallback.
  private authorLabel(userId: string): string {
    if (userId === this.userId) return "you";
    return this.members.get(userId) ?? userId.slice(0, 8);
  }

  private renderCommentDecorations(): void {
    // resolve by tracked file, not focus. activeTextEditor is undefined when
    // the conflict panel webview (or any non-editor view) has focus, and is a
    // DIFFERENT document when the user tabs to another file, both cases would
    // land the comment decoration on the wrong document or drop it entirely.
    const editor = this.findVisibleEditor();
    if (!editor) {
      return;
    }

    const byLine = new Map<number, Comment[]>();
    for (const c of this.comments) {
      const lineIdx = Math.max(0, Math.min(c.line - 1, editor.document.lineCount - 1));
      const list = byLine.get(lineIdx) ?? [];
      list.push(c);
      byLine.set(lineIdx, list);
    }

    const decorations: vscode.DecorationOptions[] = [];
    for (const [lineIdx, list] of byLine) {
      const first = list[0];
      const label =
        list.length === 1
          ? `${this.authorLabel(first.user_id)}: ${this.truncate(first.text, 50)}`
          : `${list.length} comments (latest by ${this.authorLabel(list[list.length - 1].user_id)})`;

      decorations.push({
        range: new vscode.Range(lineIdx, 0, lineIdx, editor.document.lineAt(lineIdx).text.length),
        renderOptions: {
          after: { contentText: `  ${label}` },
        },
      });
    }

    editor.setDecorations(this.commentDecoration, decorations);
  }

  private provideLineHover(
    document: vscode.TextDocument,
    position: vscode.Position,
  ): vscode.Hover | undefined {
    if (document !== vscode.window.activeTextEditor?.document) {
      return undefined;
    }
    const line = position.line + 1;

    const md = new vscode.MarkdownString();
    // allow only the view-diff command link we emit below; everything else
    // stays untrusted.
    md.isTrusted = { enabledCommands: ["lightning-git.viewChange"] };
    md.supportThemeIcons = true;
    md.supportHtml = false;
    let appended = false;

    // 1) conflict info first if this line is part of a backend-detected
    // merge conflict region. shows each branch + per-version content in a
    // proper markdown code block instead of the cramped inline decoration.
    const conflict = this.conflicts.find((c) =>
      c.base_start === c.base_end
        ? line === c.base_start + 1
        : line > c.base_start && line <= c.base_end,
    );
    if (conflict) {
      this.appendConflictMarkdown(md, conflict, document.languageId);
      appended = true;
    }

    // 2) teammates whose live version of THIS line differs from ours. show
    // their version syntax-highlighted plus a link to the full diff, instead
    // of the cramped inline string the old peek dumped into the editor.
    const currentText = document.lineAt(position.line).text;
    const teammatesHere: { userId: string; name: string; text: string }[] = [];
    for (const [userId, change] of this.teammateChanges) {
      const their = change.content.split("\n")[position.line];
      if (their !== undefined && their !== currentText) {
        teammatesHere.push({ userId, name: this.authorLabel(userId), text: their });
      }
    }
    if (teammatesHere.length > 0) {
      if (appended) {
        md.appendMarkdown("\n\n---\n\n");
      }
      teammatesHere.forEach((t, idx) => {
        if (idx > 0) {
          md.appendMarkdown("\n\n");
        }
        const args = encodeURIComponent(JSON.stringify([t.userId]));
        md.appendMarkdown(`$(account) **${this.escapeMd(t.name)}** is editing this line\n\n`);
        md.appendCodeblock(t.text, document.languageId);
        md.appendMarkdown(`[$(diff) Open full diff](command:lightning-git.viewChange?${args})`);
      });
      appended = true;
    }

    // 3) comments on this line
    const commentsHere = this.comments.filter((c) => c.line === line);
    if (commentsHere.length > 0) {
      if (appended) md.appendMarkdown("\n\n---\n\n");
      commentsHere
        .slice()
        .sort((a, b) => a.created_at - b.created_at)
        .forEach((c, idx) => {
          if (idx > 0) md.appendMarkdown("\n\n---\n\n");
          md.appendMarkdown(
            `**${this.escapeMd(this.authorLabel(c.user_id))}** · ${this.fmtAge(c.created_at)}\n\n${this.escapeMd(c.text)}`,
          );
        });
      appended = true;
    }

    if (!appended) return undefined;
    return new vscode.Hover(md);
  }

  /// Renders the conflict the way the web frontend does: group hunks by
  /// branch, then by content within the branch so identical-content rows
  /// (multiple users agreeing on the same branch) collapse to one entry
  /// with all contributors credited.
  private appendConflictMarkdown(md: vscode.MarkdownString, conflict: MergeConflict, languageId: string): void {
    md.appendMarkdown(
      `**Merge conflict** · ${this.conflictRangeLabel(conflict.base_start, conflict.base_end)}\n\n`,
    );
    type Version = { contributors: (string | null | undefined)[]; content: string[] };
    const byBranch = new Map<string, Map<string, Version>>();
    for (const h of conflict.hunks) {
      const key = h.content.join("\n");
      let versions = byBranch.get(h.branch);
      if (!versions) {
        versions = new Map();
        byBranch.set(h.branch, versions);
      }
      const existing = versions.get(key);
      if (existing) {
        existing.contributors.push(h.user_id ?? null);
      } else {
        versions.set(key, { contributors: [h.user_id ?? null], content: h.content });
      }
    }
    let first = true;
    for (const [branch, versions] of byBranch) {
      if (!first) md.appendMarkdown("\n\n");
      first = false;
      md.appendMarkdown(`**\`${branch}\`**, ${versions.size} version${versions.size === 1 ? "" : "s"}\n\n`);
      let vi = 0;
      for (const v of versions.values()) {
        if (vi > 0) md.appendMarkdown("\n\n");
        vi++;
        const labels = v.contributors
          .map((id) => (id ? `${this.authorLabel(id)} (live)` : "committed"))
          .join(", ");
        if (v.content.length === 0) {
          md.appendMarkdown(`_${this.escapeMd(labels)}_\n\n_(removed)_`);
        } else {
          md.appendMarkdown(`_${this.escapeMd(labels)}_\n\n`);
          md.appendCodeblock(v.content.join("\n"), languageId);
        }
      }
    }
  }

  // Short inline badge for a conflict region: the branches that disagree,
  // e.g. "conflict: main ↔ feature/x", or a count past two branches.
  private conflictBadgeText(conflict: MergeConflict): string {
    const branches = Array.from(new Set(conflict.hunks.map((h) => h.branch)));
    if (branches.length === 0) {
      return "⚠ conflict";
    }
    if (branches.length === 1) {
      // same-branch divergence (two overlays on one branch disagree).
      return `⚠ conflict on ${branches[0]}`;
    }
    if (branches.length === 2) {
      return `⚠ conflict: ${branches[0]} ↔ ${branches[1]}`;
    }
    return `⚠ conflict: ${branches.length} branches differ`;
  }

  /// Human label for a conflict's base range. base_start/base_end are 0-based,
  /// half-open [start, end). A pure insert / single line has base_start ===
  /// base_end; the old `${base_start+1}-${base_end}` form rendered that as
  /// e.g. "5-4" (start bumped, end not). Show a single 1-based line number in
  /// that case, otherwise a proper "first–last" 1-based span.
  private conflictRangeLabel(baseStart: number, baseEnd: number): string {
    if (baseStart === baseEnd) {
      return `line ${baseStart + 1}`;
    }
    return `lines ${baseStart + 1}–${baseEnd}`;
  }

  private fmtAge(unixSecs: number): string {
    const ageSec = Math.max(0, Math.floor(Date.now() / 1000 - unixSecs));
    if (ageSec < 60) return `${ageSec}s ago`;
    if (ageSec < 3600) return `${Math.floor(ageSec / 60)}m ago`;
    if (ageSec < 86400) return `${Math.floor(ageSec / 3600)}h ago`;
    return `${Math.floor(ageSec / 86400)}d ago`;
  }

  private escapeMd(text: string): string {
    return text.replace(/([\\`*_{}\[\]()#+\-.!])/g, "\\$1");
  }

  private truncate(value: string, maxLength: number): string {
    if (value.length <= maxLength) {
      return value;
    }

    return `${value.slice(0, maxLength)}...`;
  }

  private async getCurrentBranch(repoPath: string): Promise<string> {
    const childProcess = await import("child_process");

    return new Promise((resolve, reject) => {
      childProcess.exec("git rev-parse --abbrev-ref HEAD", { cwd: repoPath }, (error, stdout) => {
        if (error) {
          reject(error);
          return;
        }

        resolve(stdout.trim());
      });
    });
  }
}

