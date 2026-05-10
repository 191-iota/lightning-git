import * as vscode from "vscode";
import { AuthClient } from "./auth/authClient";
import { bindAuthStatusBar } from "./auth/statusBar";
import { getConfig } from "./util/config";
import { createLogger } from "./util/logger";
import { registerProjectsView, type SavedProject } from "./views/projectsProvider";

export function activate(context: vscode.ExtensionContext) {
  const log = createLogger();
  context.subscriptions.push({ dispose: () => log.dispose() });

  const config = getConfig();
  const auth = new AuthClient(context, config, log);
  context.subscriptions.push(auth);

  context.subscriptions.push(bindAuthStatusBar(auth));

  const { provider, disposable } = registerProjectsView(context);
  context.subscriptions.push(disposable);

  context.subscriptions.push(
    vscode.commands.registerCommand("lightningGit.signIn", async () => {
      if (!config.supabaseUrl || !config.supabaseAnonKey) {
        void vscode.window.showErrorMessage(
          "Set lightning-git.supabaseUrl and lightning-git.supabaseAnonKey first."
        );
        return;
      }

      await auth.signIn();
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("lightningGit.signOut", async () => {
      await auth.signOut();
      void vscode.window.showInformationMessage("Lightning Git: signed out.");
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("lightningGit.addProject", async () => {
      // TODO: replace with GET /api/projects when backend ships it.
      await provider.addProject();
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("lightningGit.openProject", async (project?: SavedProject) => {
      if (!project) {
        const projects = await provider.getProjects();

        if (projects.length === 0) {
          void vscode.window.showWarningMessage("No Lightning Git projects saved yet.");
          return;
        }

        const picked = await vscode.window.showQuickPick(
          projects.map((item) => ({
            label: item.label,
            description: item.id,
            project: item
          })),
          {
            title: "Lightning Git: Open Project",
            placeHolder: "Choose a saved project"
          }
        );

        if (!picked) {
          return;
        }

        await provider.openProject(picked.project);
        return;
      }

      await provider.openProject(project);
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("lightningGit.refreshProjects", () => {
      provider.refresh();
    })
  );

  log.info("Lightning Git activated");
}

export function deactivate() {}