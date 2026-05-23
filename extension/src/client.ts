import axios, { type AxiosInstance } from "axios";
import { AuthManager } from "./auth";

export interface LightningGitProject {
  id: string;
  name: string;
  repo_url: string;
}

export interface Org {
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

export interface Comment {
  id: string;
  user_id: string;
  line: number;
  text: string;
  created_at: number;
}

export interface ProjectMember {
  id: string;
  display_name: string;
  role: "admin" | "member";
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
      headers: { "Content-Type": "application/json" },
    });

    this.http.interceptors.request.use(async (config) => {
      const token = await this.authManager.getToken();
      if (token) {
        config.headers.Authorization = `Bearer ${token}`;
      }
      return config;
    });

    // on 401, try a single-flight refresh once; if it works, retry the original
    // request. if refresh fails, the AuthManager has already cleared the tokens.
    this.http.interceptors.response.use(
      (response) => response,
      async (error) => {
        if (!axios.isAxiosError(error) || error.response?.status !== 401) {
          return Promise.reject(error);
        }
        const original = error.config as (typeof error.config & { _retried?: boolean }) | undefined;
        const isRefreshCall = original?.url?.endsWith("/refresh");
        if (!original || original._retried || isRefreshCall) {
          return Promise.reject(error);
        }
        original._retried = true;
        const fresh = await this.authManager.refresh();
        if (!fresh) return Promise.reject(error);
        original.headers = original.headers || {};
        original.headers.Authorization = `Bearer ${fresh}`;
        return this.http.request(original);
      },
    );
  }

  async listMyOrgs(): Promise<Org[]> {
    const response = await this.http.get<Org[]>("/api/orgs/mine");
    return response.data;
  }

  async listOrgProjects(orgId: string): Promise<LightningGitProject[]> {
    const response = await this.http.get<LightningGitProject[]>(`/api/orgs/${orgId}/projects`);
    return response.data;
  }

  async createProject(repoUrl: string, name: string, orgId: string): Promise<string> {
    const response = await this.http.post("/api/projects", {
      repo_url: repoUrl,
      name,
      org_id: orgId,
      create_tasks_retroactively: false,
    });
    return response.data as string;
  }

  async getProject(projectId: string): Promise<LightningGitProject> {
    const response = await this.http.get(`/api/projects/${projectId}`);
    return response.data as LightningGitProject;
  }

  async createOverlay(projectId: string, userId: string, branch: string, fileName: string): Promise<void> {
    // branch goes as query param; file_name carries slashes in path
    const encodedFile = fileName.split("/").map(encodeURIComponent).join("/");
    try {
      await this.http.put(
        `/api/overlay/${projectId}/${userId}/${encodedFile}?branch=${encodeURIComponent(branch)}`,
      );
    } catch (error) {
      if (axios.isAxiosError(error) && error.response?.status === 400) {
        return;
      }
      throw error;
    }
  }

  async getOverlayWsUrl(projectId: string, userId: string, fileName: string): Promise<string> {
    const token = await this.authManager.getToken();
    const encodedFile = fileName.split("/").map(encodeURIComponent).join("/");
    const baseUrl = `${this.wsUrl}/api/overlay/ws/${projectId}/${userId}/${encodedFile}`;
    return token ? `${baseUrl}?token=${encodeURIComponent(token)}` : baseUrl;
  }

  /// Members of the given project, used by OverlaySession to render comment
  /// authors by name instead of UUID prefix.
  async listProjectMembers(projectId: string): Promise<ProjectMember[]> {
    try {
      const response = await this.http.get<ProjectMember[]>(`/api/projects/${projectId}/members`);
      return response.data;
    } catch {
      return [];
    }
  }

  async getMergeConflicts(projectId: string, userId: string, fileName: string): Promise<MergeConflict[]> {
    try {
      const encodedFile = fileName.split("/").map(encodeURIComponent).join("/");
      const response = await this.http.get(`/api/merge/${projectId}/${encodedFile}`, {
        params: { user_id: userId },
      });
      return response.data as MergeConflict[];
    } catch {
      return [];
    }
  }

  async listComments(projectId: string, fileName: string): Promise<Comment[]> {
    try {
      const encodedFile = fileName.split("/").map(encodeURIComponent).join("/");
      const response = await this.http.get(`/api/comments/${projectId}/${encodedFile}`);
      return response.data as Comment[];
    } catch {
      return [];
    }
  }

  // Notbremse. Resets the caller's overlay state on the server back to the
  // committed branch state. Returns the number of file overlays affected.
  async wipeMyOverlay(projectId: string): Promise<number> {
    const response = await this.http.delete<{ reset: number }>(`/api/overlay/me/${projectId}`);
    return response.data?.reset ?? 0;
  }
}
