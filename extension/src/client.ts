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

export interface ConflictHunk {
  branch: string;
  base_start: number;
  base_end: number;
  content: string[];
}

export interface MergeConflict {
  base_start: number;
  base_end: number;
  hunks: ConflictHunk[];
}

export class LightningGitClient {
  private readonly http: AxiosInstance;

  constructor(
    apiUrl: string,
    private readonly wsUrl: string,
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

  async createOverlay(projectId: string, userId: string, branch: string, fileName: string): Promise<void> {
    try {
      await this.http.put(`/api/overlay/${projectId}/${userId}/${encodeURIComponent(fileName)}/${branch}`);
    } catch (error) {
      if (axios.isAxiosError(error) && error.response?.status === 400) {
        return;
      }

      throw error;
    }
  }

  async getOverlayWsUrl(projectId: string, userId: string, fileName: string): Promise<string> {
    const token = await this.authManager.getToken();
    const baseUrl = `${this.wsUrl}/api/overlay/ws/${projectId}/${userId}/${encodeURIComponent(fileName)}`;

    return token ? `${baseUrl}?token=${encodeURIComponent(token)}` : baseUrl;
  }

  async getMergeConflicts(projectId: string, userId: string, fileName: string): Promise<MergeConflict[]> {
    try {
      const response = await this.http.get(`/api/merge/${projectId}/${encodeURIComponent(fileName)}`, {
        params: {
          user_id: userId,
        },
      });

      return response.data as MergeConflict[];
    } catch {
      return [];
    }
  }
}
