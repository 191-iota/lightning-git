import * as vscode from "vscode";

export function activate(context: vscode.ExtensionContext) {
  // TODO: auth, views, and backend clients.
  context.subscriptions.push(
    vscode.commands.registerCommand("lightningGit.signIn", async () => {
      void vscode.window.showInformationMessage("Lightning Git: not wired up yet.");
    })
  );
}

export function deactivate() {
  // no-op
}