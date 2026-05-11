import axios, { type AxiosInstance } from "axios";
import { AuthManager } from "./auth";

export class LightningGitClient {
  private readonly http: AxiosInstance;

  constructor(
    apiUrl: string,
    private readonly authManager: AuthManager,
  ) {
    this.http = axios.create({
      baseURL: apiUrl,
      headers: {
        "Content-Type": "application/json",
      },
    });

    this.http.interceptors.request.use(async (config) => {
      const token = await this.authManager.getToken();

      if (token) {
        config.headers.Authorization = `Bearer ${token}`;
      }

      return config;
    });
  }

  async createProject(repoUrl: string, name: string): Promise<string> {
    const response = await this.http.post("/api/projects", {
      repo_url: repoUrl,
      name,
    });

    return response.data as string;
  }
}
