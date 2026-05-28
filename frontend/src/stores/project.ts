import { defineStore } from "pinia";
import { ref } from "vue";
import api from "@/services/api";
import type {
  ActiveEdit,
  AddProjectMemberReq,
  CreateProjectReq,
  CreateProjectRes,
  KanbanColumn,
  Project,
  ProjectMember,
  Task,
} from "@/types/api";

export const useProjectStore = defineStore("project", () => {
  const projects = ref<Project[]>([]);
  const current = ref<Project | null>(null);
  const tasks = ref<Task[]>([]);
  const members = ref<ProjectMember[]>([]);

  async function fetch(orgId: string) {
    const { data } = await api.get<Project[]>(`/api/orgs/${orgId}/projects`);
    projects.value = data;
  }

  async function get(projectId: string) {
    const { data } = await api.get<Project>(`/api/projects/${projectId}`);
    current.value = data;
  }

  async function fetchTasks(projectId: string) {
    const { data } = await api.get<Task[]>(`/api/tasks/project/${projectId}`);
    tasks.value = data;
  }

  async function fetchActivity(projectId: string): Promise<ActiveEdit[]> {
    const { data } = await api.get<ActiveEdit[]>(`/api/projects/${projectId}/activity`);
    return data;
  }

  async function fetchMembers(projectId: string) {
    const { data } = await api.get<ProjectMember[]>(`/api/projects/${projectId}/members`);
    members.value = data;
  }

  // fallback to a uuid slice when the user_id isnt a current project member
  // (e.g. an org owner with implicit access)
  function memberName(userId: string): string {
    const m = members.value.find((x) => x.id === userId);
    return m?.display_name || userId.slice(0, 8);
  }

  async function create(req: CreateProjectReq): Promise<string> {
    const { data } = await api.post<CreateProjectRes>("/api/projects", req);
    return data.proj_id;
  }

  async function rename(projectId: string, name: string): Promise<void> {
    await api.put(`/api/projects/${projectId}`, { name });
    if (current.value?.id === projectId) current.value = { ...current.value, name };
    const idx = projects.value.findIndex((p) => p.id === projectId);
    if (idx >= 0) projects.value[idx] = { ...projects.value[idx], name };
  }

  async function remove(projectId: string): Promise<void> {
    await api.delete(`/api/projects/${projectId}`);
    projects.value = projects.value.filter((p) => p.id !== projectId);
    if (current.value?.id === projectId) current.value = null;
  }

  // Notbremse: wipes the calling user's overlays in this project back to the
  // committed git base. Server-side endpoint is gated by project membership.
  async function wipeMyOverlays(projectId: string): Promise<void> {
    await api.delete(`/api/overlay/me/${projectId}`);
  }

  async function addMember(projectId: string, req: AddProjectMemberReq): Promise<void> {
    await api.post(`/api/projects/${projectId}/members`, req);
  }

  async function removeMember(projectId: string, userId: string): Promise<void> {
    await api.delete(`/api/projects/${projectId}/members/${userId}`);
  }

  async function setTaskArchived(taskId: string, archived: boolean): Promise<void> {
    await api.patch(`/api/tasks/${taskId}/archive`, { archived });
    const t = tasks.value.find((x) => x.id === taskId);
    if (t) t.archived = archived;
  }

  async function setTaskColumn(taskId: string, column: KanbanColumn): Promise<void> {
    await api.patch(`/api/tasks/${taskId}/column`, { kanban_column: column });
    const t = tasks.value.find((x) => x.id === taskId);
    if (t) t.kanban_column = column;
  }

  function myRole(userId: string | undefined): "admin" | "member" | null {
    if (!userId) return null;
    return members.value.find((m) => m.id === userId)?.role ?? null;
  }

  function clear() {
    projects.value = [];
    current.value = null;
    tasks.value = [];
    members.value = [];
  }

  return {
    projects,
    current,
    tasks,
    members,
    fetch,
    get,
    fetchTasks,
    fetchActivity,
    fetchMembers,
    memberName,
    create,
    rename,
    remove,
    wipeMyOverlays,
    addMember,
    removeMember,
    setTaskArchived,
    setTaskColumn,
    myRole,
    clear,
  };
});
