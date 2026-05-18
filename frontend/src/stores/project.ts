import { defineStore } from "pinia";
import { ref } from "vue";
import api from "@/services/api";
import type {
  ActiveEdit,
  CreateProjectReq,
  CreateProjectRes,
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
    clear,
  };
});
