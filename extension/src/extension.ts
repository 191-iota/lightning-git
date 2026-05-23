import * as vscode from "vscode";
import axios from "axios";
import { execFile } from "child_process";
import { AuthManager } from "./auth";
import { LightningGitClient } from "./client";
import { OverlaySession } from "./overlaySession";

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

  const contentProvider = new (class implements vscode.TextDocumentContentProvider {
    private content = "";
    setContent(content: string): void {
      this.content = content;
    }
    provideTextDocumentContent(): string {
      return this.content;
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

  function getWorkspaceFolderName(): string {
    return vscode.workspace.workspaceFolders?.[0]?.name ?? "Untitled";
  }

  // converts ssh form (git@github.com:owner/repo.git) to https-equivalent
  // and strips .git suffix so we can compare across clone styles
  function normalizeGitUrl(url: string): string {
    const sshMatch = url.match(/^git@([^:]+):(.+?)(?:\.git)?$/);
    if (sshMatch) {
      return `https://${sshMatch[1]}/${sshMatch[2]}`;
    }
    return url.replace(/\.git$/, "");
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

    const create = await vscode.window.showInformationMessage(
      `No Lightning Git project found for ${repoUrl}. Create one?`,
      { modal: true },
      "Create",
    );
    if (create !== "Create") return undefined;

    return runCreateProject(repoUrl);
  }

  async function runCreateProject(prefillRepoUrl?: string): Promise<string | undefined> {
    let repoUrl = prefillRepoUrl ?? (await getWorkspaceRemoteUrl());
    if (!repoUrl) {
      repoUrl = await vscode.window.showInputBox({
        prompt: "Enter the Git repository URL",
        placeHolder: "https://github.com/owner/repo.git",
        ignoreFocusOut: true,
      });
      if (!repoUrl) return undefined;
    }

    const name = await vscode.window.showInputBox({
      prompt: "Project name",
      value: getWorkspaceFolderName(),
      ignoreFocusOut: true,
    });
    if (!name) return undefined;

    // org picker instead of pasting a UUID
    let orgs: { id: string; name: string }[];
    try {
      orgs = await client.listMyOrgs();
    } catch (error) {
      void vscode.window.showErrorMessage(`Failed to load your orgs: ${getErrorMessage(error)}`);
      return undefined;
    }
    if (orgs.length === 0) {
      void vscode.window.showWarningMessage(
        "You don't belong to any organization yet. Create one in the web frontend first.",
      );
      return undefined;
    }
    const orgPick = await vscode.window.showQuickPick(
      orgs.map((o) => ({ label: o.name, description: o.id, id: o.id })),
      { placeHolder: "Pick an organization", ignoreFocusOut: true },
    );
    if (!orgPick) return undefined;

    try {
      const projectId = await client.createProject(repoUrl, name, orgPick.id);
      await context.workspaceState.update(WORKSPACE_PROJECT_KEY, projectId);
      await context.globalState.update("lightningGit.lastOrgId", orgPick.id);
      void vscode.window.showInformationMessage(`Project "${name}" created.`);
      return projectId;
    } catch (error) {
      if (axios.isAxiosError(error) && error.response?.status === 401) {
        void vscode.window.showErrorMessage("You're not a member of this organization. Ask the org owner to add you.");
        return undefined;
      }
      void vscode.window.showErrorMessage(`Failed to create project: ${getErrorMessage(error)}`);
      return undefined;
    }
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

  const createProjectCommand = vscode.commands.registerCommand("lightning-git.createProject", async () => {
    if (!(await ensureLoggedIn())) return;
    await runCreateProject();
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

  const viewChangeCommand = vscode.commands.registerCommand("lightning-git.viewChange", async () => {
    if (!overlaySession) {
      void vscode.window.showWarningMessage("No active session.");
      return;
    }
    const users = overlaySession.getActiveUsers();
    if (users.length === 0) {
      void vscode.window.showInformationMessage("No other users have made changes yet.");
      return;
    }
    const selected = await vscode.window.showQuickPick(
      users.map((userId) => ({ label: userId.slice(0, 8), description: "View their changes", userId })),
      { placeHolder: "Select a teammate to view their changes" },
    );
    if (!selected) return;

    const change = overlaySession.getOtherUserChange(selected.userId);
    if (!change) return;

    const currentDocument = vscode.window.activeTextEditor?.document;
    if (!currentDocument) {
      void vscode.window.showWarningMessage("Open a file before viewing teammate changes.");
      return;
    }

    contentProvider.setContent(change.content);
    const teammateUri = vscode.Uri.parse(`lightning-git:${selected.label}-version`);
    await vscode.commands.executeCommand(
      "vscode.diff",
      currentDocument.uri,
      teammateUri,
      `Your version ↔ ${selected.label}'s version`,
    );
  });

  context.subscriptions.push(
    registerCommand,
    loginCommand,
    logoutCommand,
    createProjectCommand,
    startSessionCommand,
    stopSessionCommand,
    viewChangeCommand,
    notbremseCommand,
  );
}

export function deactivate(): void {
  overlaySession?.dispose();
}

function getErrorMessage(error: unknown): string {
  if (axios.isAxiosError(error)) {
    const status = error.response?.status;
    const responseData = error.response?.data;
    if (typeof responseData === "string" && responseData.trim()) {
      return responseData.trim();
    }
    if (responseData && typeof responseData === "object") {
      if ("error" in responseData && responseData.error) {
        return String(responseData.error);
      }
      try {
        return JSON.stringify(responseData);
      } catch {
        return status ? `Request failed with status ${status}` : error.message;
      }
    }
    if (status) {
      return `Request failed with status ${status}: ${error.message}`;
    }
    return error.message;
  }
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}
