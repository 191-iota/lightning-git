import * as vscode from "vscode";
import { AuthClient } from "./auth/authClient";
import { bindAuthStatusBar } from "./auth/statusBar";
import { getConfig } from "./util/config";
import { createLogger } from "./util/logger";

export function activate(context: vscode.ExtensionContext) {
  const log = createLogger();
  context.subscriptions.push({ dispose: () => log.dispose() });

  const config = getConfig();
  const auth = new AuthClient(context, config, log);
  context.subscriptions.push(auth);

  context.subscriptions.push(bindAuthStatusBar(auth));

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

  log.info("Lightning Git activated");
}

export function deactivate() {}