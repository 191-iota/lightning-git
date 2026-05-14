import * as vscode from "vscode";
import WebSocket from "ws";
import { LightningGitClient } from "./client";

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
  private activeFile: string | undefined;
  private activeBranch: string | undefined;
  private readonly teammateChanges = new Map<string, TeammateChange>();

  private readonly statusBarItem: vscode.StatusBarItem;

  private readonly peekDecoration = vscode.window.createTextEditorDecorationType({
    backgroundColor: "rgba(74, 158, 255, 0.15)",
    isWholeLine: true,
    after: {
      margin: "0 0 0 1em",
      color: "#4a9eff99",
      fontStyle: "italic",
    },
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
  }

  async start(): Promise<void> {
    this.disposables.push(
      vscode.commands.registerCommand("lightning-git.peekTeammates", () => {
        this.togglePeek();
      }),
    );

    this.disposables.push(
      vscode.window.onDidChangeActiveTextEditor((editor) => {
        this.hidePeek();

        if (!editor) {
          return;
        }

        this.openDocumentOverlay(editor.document).catch((error: unknown) => {
          const message = error instanceof Error ? error.message : String(error);
          void vscode.window.showErrorMessage(`Failed to connect overlay: ${message}`);
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

  getOtherUserChange(userId: string): TeammateChange | undefined {
    return this.teammateChanges.get(userId);
  }

  dispose(): void {
    this.ws?.close();
    this.ws = undefined;

    if (this.debounceTimer) {
      clearTimeout(this.debounceTimer);
      this.debounceTimer = undefined;
    }

    this.hidePeek();

    for (const disposable of this.disposables) {
      disposable.dispose();
    }

    this.disposables.length = 0;
    this.teammateChanges.clear();

    this.statusBarItem.dispose();
    this.peekDecoration.dispose();
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

    this.ws?.close();
    this.hidePeek();
    this.teammateChanges.clear();
    this.updateStatusBar();

    this.activeFile = relativePath;
    this.activeBranch = branch;

    await this.client.createOverlay(this.projectId, this.userId, branch, relativePath);

    const wsUrl = await this.client.getOverlayWsUrl(this.projectId, this.userId, relativePath);
    await this.connectWebSocket(wsUrl, relativePath);
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

        if (code !== 1000 && code !== 1005) {
          void vscode.window.showWarningMessage(
            `Lightning Git overlay closed (${code})${reasonText ? `: ${reasonText}` : ""}`,
          );
        }
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
    let parsed: unknown;

    try {
      parsed = JSON.parse(raw);
    } catch {
      console.warn("Lightning Git received non-JSON overlay message", raw);
      return;
    }

    if (!parsed || typeof parsed !== "object") {
      return;
    }

    const message = parsed as Record<string, unknown>;
    const change = this.unwrapChangeMessage(message);

    const userId = this.readString(change, "user_id", "userId", "User_id");

    if (!userId || userId === this.userId) {
      return;
    }

    const content = this.readString(change, "content", "Content") ?? "";
    const lineSection = this.readLineSection(change);

    this.teammateChanges.set(userId, {
      content,
      lines: lineSection,
      lastUpdate: Date.now(),
    });

    this.updateStatusBar();
    this.flashStatusBar();
  }

  private unwrapChangeMessage(message: Record<string, unknown>): Record<string, unknown> {
    const nested = message.Change ?? message.change;

    if (nested && typeof nested === "object") {
      return nested as Record<string, unknown>;
    }

    return message;
  }

  private readString(source: Record<string, unknown>, ...keys: string[]): string | undefined {
    for (const key of keys) {
      const value = source[key];

      if (typeof value === "string") {
        return value;
      }
    }

    return undefined;
  }

  private readLineSection(source: Record<string, unknown>): [number, number] {
    const value = source.line_section ?? source.lineSection ?? source.Line_section;

    if (Array.isArray(value) && value.length >= 2 && typeof value[0] === "number" && typeof value[1] === "number") {
      return [value[0], value[1]];
    }

    return [0, 0];
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

    setTimeout(() => {
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
