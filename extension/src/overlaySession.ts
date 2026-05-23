import * as vscode from "vscode";
import WebSocket from "ws";
import {
  LightningGitClient,
  type Comment,
  type ConflictHunk,
  type MergeConflict,
  type ProjectMember,
} from "./client";

interface OverlayChangeRequest {
  user_id: string;
  content: string;
  line_section: [number, number];
}

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
  private conflictPollTimer: ReturnType<typeof setInterval> | undefined;
  private commentPollTimer: ReturnType<typeof setInterval> | undefined;

  private activeFile: string | undefined;
  private activeBranch: string | undefined;
  private pendingPath: string | undefined;
  private readonly teammateChanges = new Map<string, TeammateChange>();
  private conflicts: MergeConflict[] = [];
  private comments: Comment[] = [];
  // user_id -> display_name, refreshed when the session starts. fallback to
  // a short uuid prefix for users not in the cache.
  private members = new Map<string, string>();

  private readonly statusBarItem: vscode.StatusBarItem;
  private readonly conflictStatusBarItem: vscode.StatusBarItem;

  private readonly peekDecoration = vscode.window.createTextEditorDecorationType({
    backgroundColor: "rgba(74, 158, 255, 0.15)",
    isWholeLine: true,
    after: {
      margin: "0 0 0 1em",
      color: "#4a9eff99",
      fontStyle: "italic",
    },
  });

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
  private showingConflicts = false;

  private static readonly CONFLICT_POLL_INTERVAL_MS = 30_000;
  private static readonly COMMENT_POLL_INTERVAL_MS = 5_000;

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
        provideHover: (document, position) => this.provideCommentHover(document, position),
      }),
    );

    this.disposables.push(
      vscode.window.onDidChangeActiveTextEditor((editor) => {
        this.hidePeek();
        this.hideConflictPeek();

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

        if (this.showingConflicts) {
          this.hideConflictPeek();
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

  getOtherUserChange(userId: string): TeammateChange | undefined {
    return this.teammateChanges.get(userId);
  }

  getConflicts(): MergeConflict[] {
    return this.conflicts;
  }

  dispose(): void {
    this.ws?.close();
    this.ws = undefined;

    this.stopConflictPolling();
    this.stopCommentPolling();

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
    this.hideConflictPeek();

    for (const disposable of this.disposables) {
      disposable.dispose();
    }

    this.disposables.length = 0;
    this.teammateChanges.clear();
    this.conflicts = [];
    this.comments = [];

    this.statusBarItem.dispose();
    this.conflictStatusBarItem.dispose();
    this.peekDecoration.dispose();
    this.conflictDecoration.dispose();
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
      this.hideConflictPeek();

      this.activeFile = relativePath;
      this.activeBranch = branch;
      this.teammateChanges.clear();
      this.conflicts = [];
      this.comments = [];

      this.updateStatusBar();
      this.updateConflictStatusBar();

      await this.client.createOverlay(this.projectId, this.userId, branch, relativePath);

      this.startConflictPolling();
      this.startCommentPolling();

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

    const message: OverlayChangeRequest = {
      user_id: this.userId,
      content: document.getText(),
      line_section: [0, Math.max(document.lineCount - 1, 0)],
    };

    this.ws.send(JSON.stringify(message));
  }

  private handleIncomingMessage(raw: string): void {
    let change: { user_id?: unknown; content?: unknown; line_section?: unknown };
    try {
      change = JSON.parse(raw);
    } catch {
      return;
    }
    if (typeof change.user_id !== "string" || change.user_id === this.userId) {
      return;
    }
    const content = typeof change.content === "string" ? change.content : "";
    const ls = change.line_section;
    const lines: [number, number] =
      Array.isArray(ls) && typeof ls[0] === "number" && typeof ls[1] === "number"
        ? [ls[0], ls[1]]
        : [0, 0];

    this.teammateChanges.set(change.user_id, {
      content,
      lines,
      lastUpdate: Date.now(),
    });

    this.updateStatusBar();
    this.flashStatusBar();
  }

  private togglePeek(): void {
    if (this.showingPeek) {
      this.hidePeek();
      return;
    }

    this.showPeek();
  }

  private showPeek(): void {
    const editor = vscode.window.activeTextEditor;

    if (!editor || this.teammateChanges.size === 0) {
      return;
    }

    const decorations: vscode.DecorationOptions[] = [];

    for (const [userId, teammateChange] of this.teammateChanges) {
      const [start, end] = teammateChange.lines;
      const safeStart = Math.max(0, Math.min(start, editor.document.lineCount - 1));
      const safeEnd = Math.max(0, Math.min(end, editor.document.lineCount - 1));
      const contentLines = teammateChange.content.split("\n");

      for (let line = safeStart; line <= safeEnd; line++) {
        const teammateLineIndex = line - safeStart;
        const teammateLine = contentLines[teammateLineIndex] ?? "";
        const currentLine = editor.document.lineAt(line).text;

        if (teammateLine === currentLine) {
          continue;
        }

        decorations.push({
          range: new vscode.Range(line, 0, line, editor.document.lineAt(line).text.length),
          renderOptions: {
            after: {
              contentText: ` ← ${userId.slice(0, 8)}: "${this.truncate(teammateLine, 60)}"`,
            },
          },
        });
      }
    }

    editor.setDecorations(this.peekDecoration, decorations);
    this.showingPeek = true;

    if (this.peekHideTimer) {
      clearTimeout(this.peekHideTimer);
    }

    this.peekHideTimer = setTimeout(() => {
      this.hidePeek();
    }, 4000);
  }

  private hidePeek(): void {
    const editor = vscode.window.activeTextEditor;

    if (editor) {
      editor.setDecorations(this.peekDecoration, []);
    }

    this.showingPeek = false;
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
      .map((id) => id.slice(0, 8))
      .join(", ");

    this.statusBarItem.text = `$(pulse) ${activeUsers} editing`;
    this.statusBarItem.tooltip = `Click to peek teammate changes: ${users}`;
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

  private toggleConflictPeek(): void {
    if (this.showingConflicts) {
      this.hideConflictPeek();
      return;
    }

    this.showConflictPeek();
  }

  private showConflictPeek(): void {
    const editor = vscode.window.activeTextEditor;

    if (!editor || this.conflicts.length === 0) {
      return;
    }

    const decorations: vscode.DecorationOptions[] = [];

    for (const conflict of this.conflicts) {
      const rawStart = conflict.base_start;
      const rawEnd = Math.max(conflict.base_start, conflict.base_end - 1);

      const safeStart = Math.max(0, Math.min(rawStart, editor.document.lineCount - 1));
      const safeEnd = Math.max(0, Math.min(rawEnd, editor.document.lineCount - 1));

      const inlineLabel = conflict.hunks.map((hunk) => `${hunk.branch} ${this.summariseHunk(hunk)}`).join("  ·  ");

      for (let line = safeStart; line <= safeEnd; line++) {
        const isFirst = line === safeStart;

        decorations.push({
          range: new vscode.Range(line, 0, line, editor.document.lineAt(line).text.length),
          renderOptions: isFirst
            ? {
                after: {
                  contentText: ` ⚠ ${inlineLabel}`,
                  color: new vscode.ThemeColor("editorWarning.foreground"),
                  fontStyle: "italic",
                },
              }
            : undefined,
        });
      }
    }

    editor.setDecorations(this.conflictDecoration, decorations);
    this.showingConflicts = true;

    if (this.conflictPeekHideTimer) {
      clearTimeout(this.conflictPeekHideTimer);
    }

    this.conflictPeekHideTimer = setTimeout(() => {
      this.hideConflictPeek();
    }, 10_000);
  }

  private hideConflictPeek(): void {
    const editor = vscode.window.activeTextEditor;

    if (editor) {
      editor.setDecorations(this.conflictDecoration, []);
    }

    this.showingConflicts = false;
  }

  private summariseHunk(hunk: ConflictHunk): string {
    const linesChanged = hunk.base_end - hunk.base_start;
    const linesAdded = hunk.content.length;

    if (linesChanged === 0 && linesAdded > 0) {
      return `+${linesAdded}`;
    }

    if (linesAdded === 0 && linesChanged > 0) {
      return `-${linesChanged}`;
    }

    return `~${Math.max(linesChanged, linesAdded)}`;
  }

  private async pollConflicts(): Promise<void> {
    if (!this.activeFile) {
      return;
    }

    try {
      this.conflicts = await this.client.getMergeConflicts(this.projectId, this.userId, this.activeFile);
    } catch {
      this.conflicts = [];
    }

    this.updateConflictStatusBar();

    if (this.showingConflicts) {
      this.showConflictPeek();
    }
  }

  private startConflictPolling(): void {
    this.stopConflictPolling();

    void this.pollConflicts();

    this.conflictPollTimer = setInterval(() => {
      void this.pollConflicts();
    }, OverlaySession.CONFLICT_POLL_INTERVAL_MS);
  }

  private stopConflictPolling(): void {
    if (this.conflictPollTimer) {
      clearInterval(this.conflictPollTimer);
      this.conflictPollTimer = undefined;
    }
  }

  private async pollComments(): Promise<void> {
    if (!this.activeFile) {
      return;
    }
    this.comments = await this.client.listComments(this.projectId, this.activeFile);
    this.renderCommentDecorations();
  }

  private startCommentPolling(): void {
    this.stopCommentPolling();
    void this.pollComments();
    this.commentPollTimer = setInterval(() => {
      void this.pollComments();
    }, OverlaySession.COMMENT_POLL_INTERVAL_MS);
  }

  private stopCommentPolling(): void {
    if (this.commentPollTimer) {
      clearInterval(this.commentPollTimer);
      this.commentPollTimer = undefined;
    }
  }

  /// Refreshes the cached project members map. Best-effort: failure leaves the
  /// cache untouched and authors fall back to a short uuid prefix.
  private async refreshMembers(): Promise<void> {
    const list = await this.client.listProjectMembers(this.projectId);
    this.members = new Map(list.map((m: ProjectMember) => [m.id, m.display_name]));
  }

  /// Returns the rendered author name for a comment: "you" for the caller,
  /// the cached display name otherwise, or a short uuid prefix as fallback.
  private authorLabel(userId: string): string {
    if (userId === this.userId) return "you";
    return this.members.get(userId) ?? userId.slice(0, 8);
  }

  private renderCommentDecorations(): void {
    const editor = vscode.window.activeTextEditor;
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

  private provideCommentHover(
    document: vscode.TextDocument,
    position: vscode.Position,
  ): vscode.Hover | undefined {
    if (document !== vscode.window.activeTextEditor?.document) {
      return undefined;
    }
    const line = position.line + 1;
    const here = this.comments.filter((c) => c.line === line);
    if (here.length === 0) {
      return undefined;
    }

    const md = new vscode.MarkdownString();
    md.isTrusted = false;
    md.supportHtml = false;
    here
      .slice()
      .sort((a, b) => a.created_at - b.created_at)
      .forEach((c, idx) => {
        if (idx > 0) md.appendMarkdown("\n\n---\n\n");
        md.appendMarkdown(
          `**${this.escapeMd(this.authorLabel(c.user_id))}** · ${this.fmtAge(c.created_at)}\n\n${this.escapeMd(c.text)}`,
        );
      });
    return new vscode.Hover(md);
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
