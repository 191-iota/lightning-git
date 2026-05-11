import * as vscode from "vscode";
import axios, { type AxiosInstance } from "axios";

export class AuthManager {
  private static readonly TOKEN_KEY = "lightningGit.authToken";
  private static readonly EMAIL_KEY = "lightningGit.authEmail";
  private static readonly USER_ID_KEY = "lightningGit.userId";
  private static readonly USERNAME_KEY = "lightningGit.username";

  constructor(
    private readonly context: vscode.ExtensionContext,
    private readonly apiUrl: string,
  ) {}

  private get http(): AxiosInstance {
    return axios.create({
      baseURL: this.apiUrl,
      headers: {
        "Content-Type": "application/json",
      },
    });
  }

  async register(email: string, password: string, username: string): Promise<void> {
    await this.http.post("/register", {
      email,
      password,
      username,
    });
  }

  async login(email: string, password: string): Promise<string> {
    const response = await this.http.post("/login", {
      email,
      password,
    });

    const accessToken = response.data.access_token as string;
    const userId = response.data.user_id as string;

    await this.context.secrets.store(AuthManager.TOKEN_KEY, accessToken);
    await this.context.globalState.update(AuthManager.EMAIL_KEY, email);
    await this.context.globalState.update(AuthManager.USER_ID_KEY, userId);

    return accessToken;
  }

  async getToken(): Promise<string | null> {
    return (await this.context.secrets.get(AuthManager.TOKEN_KEY)) ?? null;
  }

  getEmail(): string | null {
    return this.context.globalState.get<string>(AuthManager.EMAIL_KEY) ?? null;
  }

  getUserId(): string | null {
    return this.context.globalState.get<string>(AuthManager.USER_ID_KEY) ?? null;
  }

  getUsername(): string | null {
    return this.context.globalState.get<string>(AuthManager.USERNAME_KEY) ?? null;
  }

  async isLoggedIn(): Promise<boolean> {
    return (await this.getToken()) !== null;
  }

  async logout(): Promise<void> {
    await this.context.secrets.delete(AuthManager.TOKEN_KEY);
    await this.context.globalState.update(AuthManager.EMAIL_KEY, undefined);
    await this.context.globalState.update(AuthManager.USER_ID_KEY, undefined);
    await this.context.globalState.update(AuthManager.USERNAME_KEY, undefined);
  }
}
