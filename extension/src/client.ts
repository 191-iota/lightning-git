import axios, { type AxiosInstance } from "axios";
import { AuthManager } from "./auth";

export interface ProjectMember {
  id: string;
  username: string;
}

export interface LightningGitProject {
  id: string;
  name: string;
}

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

  async getProject(projectId: string): Promise<LightningGitProject> {
    const response = await this.http.get(`/api/projects/${projectId}`);
    return response.data as LightningGitProject;
  }

  async getProjectMembers(projectId: string): Promise<ProjectMember[]> {
    const response = await this.http.get(`/api/projects/${projectId}/members`);
    return response.data as ProjectMember[];
  }

  async updateProject(projectId: string, name: string, userIds: string[]): Promise<void> {
    await this.http.put(`/api/projects/${projectId}`, {
      name,
      user_ids: userIds,
    });
  }
}
