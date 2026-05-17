import { defineStore } from "pinia";
import { ref } from "vue";
import api from "@/services/api";
import type { CreateProjectReq, CreateProjectRes, Project, Task } from "@/types/api";

export const useProjectStore = defineStore("project", () => {
  const projects = ref<Project[]>([]);
  const current = ref<Project | null>(null);
  const tasks = ref<Task[]>([]);

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

  async function create(req: CreateProjectReq): Promise<string> {
    const { data } = await api.post<CreateProjectRes>("/api/projects", req);
    return data.proj_id;
  }

  function clear() {
    projects.value = [];
    current.value = null;
    tasks.value = [];
  }

  return { projects, current, tasks, fetch, get, fetchTasks, create, clear };
});
