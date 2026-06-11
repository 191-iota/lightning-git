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

export interface UpdateUsernamePayload {
  username: string;
}

export interface UpdateUsernameRes {
  display_name: string;
}

export interface User {
  id: string;
  email: string;
  // the handle (display_name). seeded from the JWT user_metadata at login and
  // kept current after a username change. optional because older sessions may
  // have been stored before this field existed.
  display_name?: string;
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
export type KanbanColumn = "todo" | "in_progress" | "review" | "merged";

export interface Task {
  id: string;
  project_id: string;
  name: string;
  branch_name: string;
  task_type: TaskType;
  archived?: boolean;
  kanban_column?: KanbanColumn;
  created_at?: string;
}

export interface ActiveEdit {
  file: string;
  user_id: string;
  branch: string;
  edited_sections: [number, number];
}

export interface ConflictHunk {
  branch: string;
  // present for hunks sourced from a live overlay; absent for committed
  // branch content. lets the UI distinguish "branch X committed" from
  // "user A typing on branch X right now".
  user_id?: string | null;
  base_start: number;
  base_end: number;
  content: string[];
}

export interface MergeConflict {
  base_start: number;
  base_end: number;
  hunks: ConflictHunk[];
}

export interface ProjectMember {
  id: string;
  display_name: string;
  role: "admin" | "member";
}

export interface ProjectTree {
  committed: string[];
  drafts: string[];
}

export interface OrgMember {
  user_id: string;
  display_name: string;
  role: "owner" | "member";
}

export interface AddOrgMemberReq {
  user_id: string;
  role: "owner" | "member";
}

export interface UserSearchEntry {
  display_name: string;
  id: string;
}

export interface AddProjectMemberReq {
  user_id: string;
  role: "admin" | "member";
}

export interface Comment {
  id: string;
  user_id: string;
  line: number;
  text: string;
  created_at: number;
}
