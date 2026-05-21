import * as vscode from "vscode";
import axios, { type AxiosInstance } from "axios";

export class AuthManager {
  private static readonly TOKEN_KEY = "lightningGit.authToken";
  private static readonly REFRESH_KEY = "lightningGit.refreshToken";
  private static readonly EMAIL_KEY = "lightningGit.authEmail";
  private static readonly USER_ID_KEY = "lightningGit.userId";

  private inFlightRefresh: Promise<string | null> | null = null;

  constructor(
    private readonly context: vscode.ExtensionContext,
    private readonly apiUrl: string,
  ) {}

  private get http(): AxiosInstance {
    return axios.create({
      baseURL: this.apiUrl,
      headers: { "Content-Type": "application/json" },
    });
  }

  async register(email: string, password: string, username: string): Promise<void> {
    await this.http.post("/register", { email, password, username });
  }

  async login(email: string, password: string): Promise<string> {
    const response = await this.http.post("/login", { email, password });
    const accessToken = response.data.access_token as string;
    const refreshToken = response.data.refresh_token as string;
    const userId = response.data.user_id as string;

    await this.context.secrets.store(AuthManager.TOKEN_KEY, accessToken);
    await this.context.secrets.store(AuthManager.REFRESH_KEY, refreshToken);
    await this.context.globalState.update(AuthManager.EMAIL_KEY, email);
    await this.context.globalState.update(AuthManager.USER_ID_KEY, userId);
    return accessToken;
  }

  async getToken(): Promise<string | null> {
    return (await this.context.secrets.get(AuthManager.TOKEN_KEY)) ?? null;
  }

  async getRefreshToken(): Promise<string | null> {
    return (await this.context.secrets.get(AuthManager.REFRESH_KEY)) ?? null;
  }

  getEmail(): string | null {
    return this.context.globalState.get<string>(AuthManager.EMAIL_KEY) ?? null;
  }

  getUserId(): string | null {
    return this.context.globalState.get<string>(AuthManager.USER_ID_KEY) ?? null;
  }

  async isLoggedIn(): Promise<boolean> {
    return (await this.getToken()) !== null;
  }

  // single-flight refresh: concurrent 401s share one in-flight call
  async refresh(): Promise<string | null> {
    if (this.inFlightRefresh) return this.inFlightRefresh;
    this.inFlightRefresh = (async () => {
      try {
        const refreshToken = await this.getRefreshToken();
        if (!refreshToken) return null;
        const response = await this.http.post("/refresh", { refresh_token: refreshToken });
        const access = response.data.access_token as string;
        const refresh = response.data.refresh_token as string;
        await this.context.secrets.store(AuthManager.TOKEN_KEY, access);
        await this.context.secrets.store(AuthManager.REFRESH_KEY, refresh);
        return access;
      } catch {
        await this.logout();
        return null;
      } finally {
        this.inFlightRefresh = null;
      }
    })();
    return this.inFlightRefresh;
  }

  async logout(): Promise<void> {
    await this.context.secrets.delete(AuthManager.TOKEN_KEY);
    await this.context.secrets.delete(AuthManager.REFRESH_KEY);
    await this.context.globalState.update(AuthManager.EMAIL_KEY, undefined);
    await this.context.globalState.update(AuthManager.USER_ID_KEY, undefined);
  }
}
