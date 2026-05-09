import * as vscode from "vscode";

export type LightningGitConfig = Readonly<{
  backendUrl: string;
  supabaseUrl: string;
  supabaseAnonKey: string;
}>;

const SECTION = "lightning-git";

export function getConfig(): LightningGitConfig {
  const cfg = vscode.workspace.getConfiguration(SECTION);

  return {
    backendUrl: cfg.get<string>("backendUrl", "http://127.0.0.1:8787").replace(/\/$/, ""),
    supabaseUrl: cfg.get<string>("supabaseUrl", ""),
    supabaseAnonKey: cfg.get<string>("supabaseAnonKey", "")
  };
}