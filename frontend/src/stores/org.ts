import { defineStore } from "pinia";
import { ref } from "vue";
import api from "@/services/api";
import type { CreateOrgReq, CreateOrgRes, MyOrg } from "@/types/api";

export const useOrgStore = defineStore("org", () => {
  const ORG_KEY = "currentOrgId";

  const orgs = ref<MyOrg[]>([]);
  const currentOrgId = ref<string | null>(localStorage.getItem(ORG_KEY));

  async function fetch() {
    const { data } = await api.get<MyOrg[]>("/api/orgs/mine");
    orgs.value = data;
  }

  async function create(req: CreateOrgReq): Promise<string> {
    const { data } = await api.post<CreateOrgRes>("/api/orgs", req);
    return data.org_id;
  }

  function select(id: string) {
    currentOrgId.value = id;
    localStorage.setItem(ORG_KEY, id);
  }

  function clear() {
    orgs.value = [];
    currentOrgId.value = null;
    localStorage.removeItem(ORG_KEY);
  }

  return { orgs, currentOrgId, fetch, create, select, clear };
});
