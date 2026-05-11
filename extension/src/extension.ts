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

    const username = await vscode.window.showInputBox({
      prompt: "Enter your GitHub username",
      placeHolder: "octocat",
      value: authManager.getUsername() ?? "",
      ignoreFocusOut: true,
    });

    if (!username) {
      return false;
    }

    try {
      await authManager.login(email, password, username);
      void vscode.window.showInformationMessage(`Logged in as ${username}`);
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

  context.subscriptions.push(registerCommand, loginCommand, logoutCommand, createProjectCommand);
}

export function deactivate(): void {}

function getErrorMessage(error: unknown): string {
  if (axios.isAxiosError(error)) {
    const responseData = error.response?.data;

    if (typeof responseData === "string") {
      return responseData;
    }

    if (responseData && typeof responseData === "object" && "message" in responseData) {
      return String(responseData.message);
    }

    return error.message;
  }

  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}
