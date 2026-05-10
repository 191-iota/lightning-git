import * as vscode from "vscode";
import type { AuthClient, AuthState } from "./authClient";

function getPresentation(state: AuthState): { text: string; tooltip: string; command: string } {
  switch (state.kind) {
    case "signedIn":
      return {
        text: "$(account) Lightning Git",
        tooltip: "Lightning Git: signed in",
        command: "lightningGit.signOut"
      };
    case "refreshing":
      return {
        text: "$(loading~spin) Lightning Git",
        tooltip: "Lightning Git: refreshing session",
        command: "lightningGit.signIn"
      };
    case "signedOut":
    default:
      return {
        text: "$(account) Sign in to Lightning Git",
        tooltip: "Lightning Git: signed out",
        command: "lightningGit.signIn"
      };
  }
}

export function bindAuthStatusBar(auth: AuthClient): vscode.Disposable {
  const item = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
  item.name = "Lightning Git Auth";

  const render = (state: AuthState) => {
    const presentation = getPresentation(state);
    item.text = presentation.text;
    item.tooltip = presentation.tooltip;
    item.command = presentation.command;
    item.show();
  };

  render(auth.state);

  const sub = auth.onDidChangeState((state) => {
    render(state);
  });

  return {
    dispose: () => {
      sub.dispose();
      item.dispose();
    }
  };
}