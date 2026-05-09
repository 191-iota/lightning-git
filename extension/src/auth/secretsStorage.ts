import * as vscode from "vscode";

export class SecretsStorage {
  readonly #secrets: vscode.SecretStorage;
  readonly #prefix: string;

  constructor(secrets: vscode.SecretStorage, prefix = "lightningGit.supabase") {
    this.#secrets = secrets;
    this.#prefix = prefix;
  }

  #key(key: string): string {
    return `${this.#prefix}:${key}`;
  }

  async getItem(key: string): Promise<string | null> {
    const value = await this.#secrets.get(this.#key(key));
    return value ?? null;
  }

  async setItem(key: string, value: string): Promise<void> {
    await this.#secrets.store(this.#key(key), value);
  }

  async removeItem(key: string): Promise<void> {
    await this.#secrets.delete(this.#key(key));
  }
}