import * as vscode from "vscode";
import { createClient, type Session, type SupabaseClient } from "@supabase/supabase-js";
import type { LightningGitConfig } from "../util/config";
import type { Logger } from "../util/logger";
import { SecretsStorage } from "./secretsStorage";

export type AuthState =
  | { kind: "signedOut" }
  | { kind: "signedIn"; session: Session };

export class AuthClient implements vscode.Disposable {
  readonly #log: Logger;
  readonly #supabase: SupabaseClient;
  readonly #onDidChangeState = new vscode.EventEmitter<AuthState>();

  #state: AuthState = { kind: "signedOut" };
  #subscription?: { unsubscribe: () => void };

  constructor(context: vscode.ExtensionContext, config: LightningGitConfig, log: Logger) {
    this.#log = log;

    const storage = new SecretsStorage(context.secrets);

    this.#supabase = createClient(config.supabaseUrl, config.supabaseAnonKey, {
      auth: {
        persistSession: true,
        autoRefreshToken: true,
        storage: {
          getItem: (key) => storage.getItem(key),
          setItem: (key, value) => storage.setItem(key, value),
          removeItem: (key) => storage.removeItem(key)
        }
      }
    });

    this.#subscription = this.#supabase.auth.onAuthStateChange((_event, session) => {
      if (session) {
        this.#setState({ kind: "signedIn", session });
      } else {
        this.#setState({ kind: "signedOut" });
      }
    }).data.subscription;

    void this.restoreSession();
  }

  dispose(): void {
    this.#subscription?.unsubscribe();
    this.#onDidChangeState.dispose();
  }

  get onDidChangeState(): vscode.Event<AuthState> {
    return this.#onDidChangeState.event;
  }

  get state(): AuthState {
    return this.#state;
  }

  async restoreSession(): Promise<void> {
    const { data, error } = await this.#supabase.auth.getSession();

    if (error) {
      this.#log.warn(`restoreSession failed: ${error.message}`);
      return;
    }

    if (data.session) {
      this.#setState({ kind: "signedIn", session: data.session });
    }
  }

  async getAccessToken(): Promise<string | null> {
    const { data, error } = await this.#supabase.auth.getSession();

    if (error) {
      this.#log.warn(`getAccessToken failed: ${error.message}`);
      return null;
    }

    return data.session?.access_token ?? null;
  }

  async signIn(): Promise<void> {
    const token = await vscode.window.showInputBox({
      title: "Lightning Git: Paste Supabase JWT",
      prompt: "Temporary dev flow until the full auth handoff is wired.",
      password: true,
      ignoreFocusOut: true
    });

    if (!token) {
      return;
    }

    const { data, error } = await this.#supabase.auth.setSession({
      access_token: token,
      refresh_token: token
    });

    if (error) {
      this.#log.error(`signIn failed: ${error.message}`);
      void vscode.window.showErrorMessage(`Lightning Git sign-in failed: ${error.message}`);
      return;
    }

    if (data.session) {
      this.#setState({ kind: "signedIn", session: data.session });
      void vscode.window.showInformationMessage("Lightning Git: signed in.");
      return;
    }

    void vscode.window.showWarningMessage("Lightning Git: sign-in produced no session.");
  }

  async signOut(): Promise<void> {
    const { error } = await this.#supabase.auth.signOut();

    if (error) {
      this.#log.warn(`signOut failed: ${error.message}`);
    }

    this.#setState({ kind: "signedOut" });
  }

  #setState(next: AuthState): void {
    this.#state = next;
    this.#onDidChangeState.fire(next);
  }
}