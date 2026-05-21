export interface LoginPayload {
  email: string;
  password: string;
}

export interface RegisterPayload {
  email: string;
  username: string;
  password: string;
}

export interface LoginRes {
  user_id: string;
  email: string;
  access_token: string;
  refresh_token: string;
}

export interface RefreshRes {
  access_token: string;
  refresh_token: string;
}

export interface User {
  id: string;
  email: string;
}

export interface MyOrg {
  id: string;
  name: string;
  role: "owner" | "member";
}

export interface CreateOrgReq {
  name: string;
}

export interface CreateOrgRes {
  org_id: string;
}

export interface Project {
  id: string;
  org_id: string;
  name: string;
  repo_url: string;
  created_at?: string;
}

export interface CreateProjectReq {
  repo_url: string;
  name: string;
  create_tasks_retroactively: boolean;
  org_id: string;
}

export interface CreateProjectRes {
  proj_id: string;
}

export type TaskType = "Bug" | "Feature" | "Improvement" | "Unknown";

export interface Task {
  id: string;
  project_id: string;
  name: string;
  branch_name: string;
  task_type: TaskType;
  created_at?: string;
}

export interface ActiveEdit {
  file: string;
  user_id: string;
  branch: string;
  edited_sections: [number, number];
}

export interface ProjectMember {
  id: string;
  display_name: string;
  role: "admin" | "member";
}
