import * as vscode from "vscode";
import { execFile } from "child_process";
import { AuthManager } from "./auth";
import { LightningGitClient } from "./client";
import { OverlaySession } from "./overlaySession";
import { normalizeGitUrl } from "./gitUrl";
import { getErrorMessage } from "./errorMessage";

let authManager: AuthManager;
let client: LightningGitClient;
let overlaySession: OverlaySession | undefined;

const WORKSPACE_PROJECT_KEY = "lightningGit.projectId";

export function activate(context: vscode.ExtensionContext): void {
  console.log("Lightning Git extension is now active!");

  const config = vscode.workspace.getConfiguration("lightningGit");
  const apiUrl = config.get<string>("apiUrl", "http://localhost:8080");
  const wsUrl = config.get<string>("wsUrl", "ws://localhost:8080");
  const debounceMs = config.get<number>("debounceMs", 250);

  authManager = new AuthManager(context, apiUrl);
  client = new LightningGitClient(apiUrl, wsUrl, authManager);

  // Notbremse status bar item. Lives at the extension level (not inside
  // OverlaySession) because the spec requires it to be visible at all times,
  // independently of whether a session is currently active.
  const notbremseStatusItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 1000);
  notbremseStatusItem.text = "$(zap) Notbremse";
  notbremseStatusItem.tooltip =
    "Reset your live overlay back to the committed branch state. Use if you typed credentials by accident.";
  notbremseStatusItem.command = "lightning-git.notbremse";
  notbremseStatusItem.color = new vscode.ThemeColor("statusBarItem.warningForeground");
  notbremseStatusItem.backgroundColor = new vscode.ThemeColor("statusBarItem.warningBackground");
  notbremseStatusItem.show();
  context.subscriptions.push(notbremseStatusItem);

  // Backs the right-hand side of the teammate diff. Keyed by URI so several
  // teammate diffs can be open at once, and fires onDidChange so reopening the
  // same teammate refreshes instead of showing a stale buffer.
  const contentProvider = new (class implements vscode.TextDocumentContentProvider {
    private readonly contents = new Map<string, string>();
    private readonly emitter = new vscode.EventEmitter<vscode.Uri>();
    readonly onDidChange = this.emitter.event;
    setContent(uri: vscode.Uri, content: string): void {
      this.contents.set(uri.toString(), content);
      this.emitter.fire(uri);
    }
    provideTextDocumentContent(uri: vscode.Uri): string {
      return this.contents.get(uri.toString()) ?? "";
    }
  })();

  context.subscriptions.push(vscode.workspace.registerTextDocumentContentProvider("lightning-git", contentProvider));

  // Helpers
  async function ensureLoggedIn(): Promise<boolean> {
    if (await authManager.isLoggedIn()) return true;

    const email = await vscode.window.showInputBox({
      prompt: "Enter your email",
      placeHolder: "user@example.com",
      value: authManager.getEmail() ?? "",
      ignoreFocusOut: true,
    });
    if (!email) return false;

    const password = await vscode.window.showInputBox({
      prompt: "Enter your password",
      password: true,
      ignoreFocusOut: true,
    });
    if (!password) return false;

    try {
      await authManager.login(email, password);
      void vscode.window.showInformationMessage(`Logged in as ${email}`);
      return true;
    } catch (error) {
      void vscode.window.showErrorMessage(`Login failed: ${getErrorMessage(error)}`);
      return false;
    }
  }

  async function getWorkspaceRemoteUrl(): Promise<string | undefined> {
    const folder = vscode.workspace.workspaceFolders?.[0];
    if (!folder) return undefined;
    return new Promise((resolve) => {
      execFile("git", ["config", "--get", "remote.origin.url"], { cwd: folder.uri.fsPath }, (err, stdout) =>
        resolve(err ? undefined : stdout.trim()),
      );
    });
  }

  async function findProjectByRepoUrl(repoUrl: string): Promise<string | undefined> {
    const target = normalizeGitUrl(repoUrl);
    const orgs = await client.listMyOrgs();
    for (const org of orgs) {
      const projects = await client.listOrgProjects(org.id);
      const match = projects.find((p) => normalizeGitUrl(p.repo_url) === target);
      if (match) return match.id;
    }
    return undefined;
  }

  // returns the project id linked to the current workspace, doing the
  // auto-detect + create flow on first use. Caches in workspaceState.
  async function ensureProject(): Promise<string | undefined> {
    const cached = context.workspaceState.get<string>(WORKSPACE_PROJECT_KEY);
    if (cached) return cached;

    const repoUrl = await getWorkspaceRemoteUrl();
    if (!repoUrl) {
      void vscode.window.showWarningMessage("Open a git workspace with a remote configured before starting a session.");
      return undefined;
    }

    try {
      const found = await findProjectByRepoUrl(repoUrl);
      if (found) {
        await context.workspaceState.update(WORKSPACE_PROJECT_KEY, found);
        return found;
      }
    } catch (error) {
      void vscode.window.showErrorMessage(`Lookup failed: ${getErrorMessage(error)}`);
      return undefined;
    }

    // Projects are created in the web frontend only; the extension just links
    // an existing project to this workspace.
    void vscode.window.showWarningMessage(
      `No Lightning Git project found for ${repoUrl}. Create the project in the web frontend, then start a session here.`,
    );
    return undefined;
  }

  // ---------- commands ----------

  const registerCommand = vscode.commands.registerCommand("lightning-git.register", async () => {
    const email = await vscode.window.showInputBox({
      prompt: "Enter your email",
      placeHolder: "user@example.com",
      ignoreFocusOut: true,
    });
    if (!email) return;

    const password = await vscode.window.showInputBox({
      prompt: "Enter your password",
      password: true,
      ignoreFocusOut: true,
    });
    if (!password) return;

    const username = await vscode.window.showInputBox({
      prompt: "Pick a username",
      placeHolder: "octocat",
      ignoreFocusOut: true,
    });
    if (!username) return;

    try {
      await authManager.register(email, password, username);
      void vscode.window.showInformationMessage("Registration successful! You can now login.");
    } catch (error) {
      void vscode.window.showErrorMessage(`Registration failed: ${getErrorMessage(error)}`);
    }
  });

  const loginCommand = vscode.commands.registerCommand("lightning-git.login", async () => {
    // always re-prompt so the command is never a silent no-op on a stale token
    await authManager.logout();
    await ensureLoggedIn();
  });

  const logoutCommand = vscode.commands.registerCommand("lightning-git.logout", async () => {
    await authManager.logout();
    overlaySession?.dispose();
    overlaySession = undefined;
    void vscode.window.showInformationMessage("Logged out successfully.");
  });

  const startSessionCommand = vscode.commands.registerCommand("lightning-git.startSession", async () => {
    if (!(await ensureLoggedIn())) return;
    const projectId = await ensureProject();
    if (!projectId) return;
    const userId = authManager.getUserId();
    if (!userId) {
      void vscode.window.showErrorMessage("User ID missing. Please logout and login again.");
      return;
    }

    overlaySession?.dispose();
    overlaySession = new OverlaySession(client, projectId, userId, debounceMs);

    try {
      await overlaySession.start();
      void vscode.window.showInformationMessage("Lightning Git session started.");
    } catch (error) {
      overlaySession.dispose();
      overlaySession = undefined;
      void vscode.window.showErrorMessage(`Failed to start session: ${getErrorMessage(error)}`);
    }
  });

  const stopSessionCommand = vscode.commands.registerCommand("lightning-git.stopSession", () => {
    overlaySession?.dispose();
    overlaySession = undefined;
    void vscode.window.showInformationMessage("Lightning Git session stopped.");
  });

  const notbremseCommand = vscode.commands.registerCommand("lightning-git.notbremse", async () => {
    const projectId = context.workspaceState.get<string>(WORKSPACE_PROJECT_KEY);
    if (!projectId) {
      void vscode.window.showInformationMessage(
        "Nothing to reset. No Lightning Git session is or has been active here.",
      );
      return;
    }
    if (!(await authManager.isLoggedIn())) {
      void vscode.window.showWarningMessage("You are not logged in.");
      return;
    }

    const confirmation = await vscode.window.showWarningMessage(
      "Reset your live overlay to the committed branch state?",
      {
        modal: true,
        detail:
          "Your in-flight edits in this project are reverted on the server back to the latest committed state of your branch. Teammates' live views are updated immediately. Edits already broadcast cannot be recalled. Edits still sitting in your local editor are not touched, undo them yourself if needed.",
      },
      "Reset now",
    );
    if (confirmation !== "Reset now") return;

    try {
      const reset = await client.wipeMyOverlay(projectId);
      void vscode.window.showInformationMessage(
        reset > 0
          ? `Notbremse triggered. Reset ${reset} file overlay${reset === 1 ? "" : "s"} on the server.`
          : "Notbremse triggered. Nothing was on the server to reset.",
      );
    } catch (error) {
      void vscode.window.showErrorMessage(`Notbremse failed: ${getErrorMessage(error)}`);
    }
  });

  const viewChangeCommand = vscode.commands.registerCommand(
    "lightning-git.viewChange",
    async (presetUserId?: string) => {
      const session = overlaySession;
      if (!session) {
        void vscode.window.showWarningMessage("No active session.");
        return;
      }
      const users = session.getActiveUsers();
      if (users.length === 0) {
        void vscode.window.showInformationMessage("No other users have made changes yet.");
        return;
      }

      // the line hover passes the teammate's id directly; otherwise prompt
      // with display names, not raw uuids.
      let userId =
        typeof presetUserId === "string" && users.includes(presetUserId) ? presetUserId : undefined;
      if (!userId) {
        const selected = await vscode.window.showQuickPick(
          users.map((id) => ({
            label: session.authorLabelFor(id),
            description: "View their live changes",
            userId: id,
          })),
          { placeHolder: "Select a teammate to view their changes" },
        );
        if (!selected) {
          return;
        }
        userId = selected.userId;
      }

      const change = session.getOtherUserChange(userId);
      if (!change) {
        return;
      }

      const currentDocument = vscode.window.activeTextEditor?.document;
      if (!currentDocument) {
        void vscode.window.showWarningMessage("Open a file before viewing teammate changes.");
        return;
      }

      const name = session.authorLabelFor(userId);
      // path drives the pretty filename in the diff title; the user id rides in
      // the query so two teammates sharing a display name still get distinct
      // virtual documents.
      const teammateUri = vscode.Uri.from({ scheme: "lightning-git", path: `/${name} (live)`, query: userId });
      contentProvider.setContent(teammateUri, change.content);
      await vscode.commands.executeCommand(
        "vscode.diff",
        currentDocument.uri,
        teammateUri,
        `Your version ↔ ${name}'s live version`,
      );
    },
  );

  context.subscriptions.push(
    registerCommand,
    loginCommand,
    logoutCommand,
    startSessionCommand,
    stopSessionCommand,
    viewChangeCommand,
    notbremseCommand,
  );
}

export function deactivate(): void {
  overlaySession?.dispose();
}
