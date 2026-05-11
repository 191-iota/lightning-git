import * as vscode from "vscode";
import axios from "axios";
import { AuthManager } from "./auth";
import { LightningGitClient } from "./client";

let authManager: AuthManager;
let client: LightningGitClient;

export function activate(context: vscode.ExtensionContext): void {
  console.log("Lightning Git extension is now active!");

  const config = vscode.workspace.getConfiguration("lightningGit");
  const apiUrl = config.get<string>("apiUrl", "http://localhost:8080");

  authManager = new AuthManager(context, apiUrl);
  client = new LightningGitClient(apiUrl, authManager);

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
      const projectId = await client.createProject(repoUrl, name);
      await context.globalState.update("lightningGit.lastProjectId", projectId);

      void vscode.window.showInformationMessage(`Project "${name}" created! ID: ${projectId}`);
      return projectId;
    } catch (error) {
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

  context.subscriptions.push(
    registerCommand,
    loginCommand,
    logoutCommand,
    createProjectCommand,
    connectGithubCommand,
    viewProjectMembersCommand,
    addProjectMemberCommand,
  );
}

export function deactivate(): void {}

function getErrorMessage(error: unknown): string {
  if (axios.isAxiosError(error)) {
    const status = error.response?.status;
    const responseData = error.response?.data;

    if (typeof responseData === "string") {
      return responseData;
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
