import * as vscode from "vscode";
import axios from "axios";
import { AuthManager } from "./auth";
import { LightningGitClient } from "./client";
import { OverlaySession } from "./overlaySession";

let authManager: AuthManager;
let client: LightningGitClient;
let overlaySession: OverlaySession | undefined;

export function activate(context: vscode.ExtensionContext): void {
  console.log("Lightning Git extension is now active!");

  const config = vscode.workspace.getConfiguration("lightningGit");
  const apiUrl = config.get<string>("apiUrl", "http://localhost:8080");
  const wsUrl = config.get<string>("wsUrl", "ws://localhost:8080");
  const debounceMs = config.get<number>("debounceMs", 1000);

  authManager = new AuthManager(context, apiUrl);
  client = new LightningGitClient(apiUrl, wsUrl, authManager);

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

  async function ensureLoggedIn(): Promise<boolean> {
    if (await authManager.isLoggedIn()) {
      return true;
    }

    const email = await vscode.window.showInputBox({
      prompt: "Enter your email",
      placeHolder: "user@example.com",
      value: authManager.getEmail() ?? "",
      ignoreFocusOut: true,
    });

    if (!email) {
      return false;
    }

    const password = await vscode.window.showInputBox({
      prompt: "Enter your password",
      password: true,
      ignoreFocusOut: true,
    });

    if (!password) {
      return false;
    }

    try {
      await authManager.login(email, password);
      void vscode.window.showInformationMessage(`Logged in as ${email}`);
      return true;
    } catch (error) {
      void vscode.window.showErrorMessage(`Login failed: ${getErrorMessage(error)}`);
      return false;
    }
  }

  async function ensureProject(): Promise<string | undefined> {
    const existing = context.globalState.get<string>("lightningGit.lastProjectId");

    if (existing) {
      const choice = await vscode.window.showQuickPick(
        [
          {
            label: "Use existing project",
            description: `${existing.slice(0, 8)}…`,
            id: "existing",
          },
          {
            label: "Create new project",
            id: "new",
          },
        ],
        {
          placeHolder: "Which project?",
          ignoreFocusOut: true,
        },
      );

      if (!choice) {
        return undefined;
      }

      if (choice.id === "existing") {
        return existing;
      }
    }

    return runCreateProject();
  }

  async function runCreateProject(): Promise<string | undefined> {
    const repoUrl = await vscode.window.showInputBox({
      prompt: "Enter the Git repository URL",
      placeHolder: "https://github.com/owner/repo.git",
      ignoreFocusOut: true,
    });

    if (!repoUrl) {
      return undefined;
    }

    const name = await vscode.window.showInputBox({
      prompt: "Enter a project name",
      placeHolder: "My Project",
      ignoreFocusOut: true,
    });

    if (!name) {
      return undefined;
    }

    const lastOrgId = context.globalState.get<string>("lightningGit.lastOrgId") ?? "";

    const orgId = await vscode.window.showInputBox({
      prompt: "Enter the Organization ID (UUID) this project belongs to",
      placeHolder: "3fa85f64-5717-4562-b3fc-2c963f66afa6",
      value: lastOrgId,
      ignoreFocusOut: true,
    });

    if (!orgId) {
      return undefined;
    }

    const visibility = await vscode.window.showQuickPick(["Public", "Private"], {
      placeHolder: "Is this repository private?",
      ignoreFocusOut: true,
    });

    if (!visibility) {
      return undefined;
    }

    if (visibility === "Private") {
      const userId = authManager.getUserId();

      if (!userId) {
        void vscode.window.showWarningMessage("User ID not found. Please login again.");
        return undefined;
      }

      await vscode.env.openExternal(vscode.Uri.parse(`${apiUrl}/auth/github/${userId}`));

      const confirmed = await vscode.window.showInformationMessage(
        'Complete GitHub authorization in your browser, then click "Done" to continue.',
        { modal: true },
        "Done",
      );

      if (confirmed !== "Done") {
        return undefined;
      }
    }

    try {
      const projectId = await client.createProject(repoUrl, name, orgId);
      await context.globalState.update("lightningGit.lastProjectId", projectId);
      await context.globalState.update("lightningGit.lastOrgId", orgId);

      void vscode.window.showInformationMessage(`Project "${name}" created! ID: ${projectId}`);
      return projectId;
    } catch (error) {
      // 401 here means the user isn't a member of that org (backend's require_org_permission)
      if (axios.isAxiosError(error) && error.response?.status === 401) {
        void vscode.window.showErrorMessage(
          "You're not a member of this organization. Ask the org owner to add you, or pick a different org.",
        );
        return undefined;
      }
      void vscode.window.showErrorMessage(`Failed to create project: ${getErrorMessage(error)}`);
      return undefined;
    }
  }

  const registerCommand = vscode.commands.registerCommand("lightning-git.register", async () => {
    const email = await vscode.window.showInputBox({
      prompt: "Enter your email",
      placeHolder: "user@example.com",
      ignoreFocusOut: true,
    });

    if (!email) {
      return;
    }

    const password = await vscode.window.showInputBox({
      prompt: "Enter your password",
      password: true,
      ignoreFocusOut: true,
    });

    if (!password) {
      return;
    }

    const username = await vscode.window.showInputBox({
      prompt: "Enter your GitHub username",
      placeHolder: "octocat",
      ignoreFocusOut: true,
    });

    if (!username) {
      return;
    }

    try {
      await authManager.register(email, password, username);
      void vscode.window.showInformationMessage("Registration successful! You can now login.");
    } catch (error) {
      void vscode.window.showErrorMessage(`Registration failed: ${getErrorMessage(error)}`);
    }
  });

  const loginCommand = vscode.commands.registerCommand("lightning-git.login", async () => {
    await ensureLoggedIn();
  });

  const logoutCommand = vscode.commands.registerCommand("lightning-git.logout", async () => {
    await authManager.logout();
    overlaySession?.dispose();
    overlaySession = undefined;
    void vscode.window.showInformationMessage("Logged out successfully.");
  });

  const createProjectCommand = vscode.commands.registerCommand("lightning-git.createProject", async () => {
    if (!(await ensureLoggedIn())) {
      return;
    }

    await runCreateProject();
  });

  const connectGithubCommand = vscode.commands.registerCommand("lightning-git.connectGithub", async () => {
    if (!(await ensureLoggedIn())) {
      return;
    }

    const userId = authManager.getUserId();

    if (!userId) {
      void vscode.window.showWarningMessage("User ID not found. Please login again.");
      return;
    }

    await vscode.env.openExternal(vscode.Uri.parse(`${apiUrl}/auth/github/${userId}`));
  });

  const viewProjectMembersCommand = vscode.commands.registerCommand("lightning-git.viewProjectMembers", async () => {
    if (!(await ensureLoggedIn())) {
      return;
    }

    const lastProjectId = context.globalState.get<string>("lightningGit.lastProjectId") ?? "";

    const projectId = await vscode.window.showInputBox({
      prompt: "Enter Project ID (UUID)",
      placeHolder: "3fa85f64-5717-4562-b3fc-2c963f66afa6",
      value: lastProjectId,
      ignoreFocusOut: true,
    });

    if (!projectId) {
      return;
    }

    try {
      const members = await client.getProjectMembers(projectId);

      if (members.length === 0) {
        void vscode.window.showInformationMessage("No members found for this project.");
        return;
      }

      const items = members.map((member) => ({
        label: member.username,
        description: member.id,
      }));

      await vscode.window.showQuickPick(items, {
        placeHolder: "Project Members",
        canPickMany: false,
      });
    } catch (error) {
      void vscode.window.showErrorMessage(`Failed to get project members: ${getErrorMessage(error)}`);
    }
  });

  const addProjectMemberCommand = vscode.commands.registerCommand("lightning-git.addProjectMember", async () => {
    if (!(await ensureLoggedIn())) {
      return;
    }

    const lastProjectId = context.globalState.get<string>("lightningGit.lastProjectId") ?? "";

    const projectId = await vscode.window.showInputBox({
      prompt: "Enter Project ID (UUID)",
      placeHolder: "3fa85f64-5717-4562-b3fc-2c963f66afa6",
      value: lastProjectId,
      ignoreFocusOut: true,
    });

    if (!projectId) {
      return;
    }

    const newUserId = await vscode.window.showInputBox({
      prompt: "Enter User ID (UUID) to add",
      placeHolder: "3fa85f64-5717-4562-b3fc-2c963f66afa6",
      ignoreFocusOut: true,
    });

    if (!newUserId) {
      return;
    }

    try {
      const project = await client.getProject(projectId);
      const currentMembers = await client.getProjectMembers(projectId);
      const currentMemberIds = currentMembers.map((member) => member.id);

      await client.updateProject(projectId, project.name, [...currentMemberIds, newUserId]);

      void vscode.window.showInformationMessage("User added to project successfully!");
    } catch (error) {
      void vscode.window.showErrorMessage(`Failed to add member: ${getErrorMessage(error)}`);
    }
  });

  const startSessionCommand = vscode.commands.registerCommand("lightning-git.startSession", async () => {
    if (!(await ensureLoggedIn())) {
      return;
    }

    const projectId = await ensureProject();

    if (!projectId) {
      return;
    }

    const userId = authManager.getUserId();

    if (!userId) {
      void vscode.window.showErrorMessage("User ID missing. Please logout and login again.");
      return;
    }

    overlaySession?.dispose();
    overlaySession = new OverlaySession(client, projectId, userId, debounceMs);

    try {
      await overlaySession.start();
      void vscode.window.showInformationMessage("Lightning Git session started!");
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
      users.map((userId) => ({
        label: userId.slice(0, 8),
        description: "View their changes",
        userId,
      })),
      {
        placeHolder: "Select a teammate to view their changes",
      },
    );

    if (!selected) {
      return;
    }

    const change = overlaySession.getOtherUserChange(selected.userId);

    if (!change) {
      return;
    }

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
    connectGithubCommand,
    viewProjectMembersCommand,
    addProjectMemberCommand,
    startSessionCommand,
    stopSessionCommand,
    viewChangeCommand,
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
