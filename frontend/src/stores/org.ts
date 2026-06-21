import { defineStore } from "pinia";
import { ref } from "vue";
import api from "@/services/api";
import type {
  AddOrgMemberReq,
  CreateOrgReq,
  CreateOrgRes,
  MyOrg,
  OrgMember,
  UserSearchEntry,
} from "@/types/api";

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
    // the backend registers the creator as owner; mirror that into the local
    // list so role-gated views (e.g. the org members/invite UI) resolve the
    // role without waiting for a refetch that only happens when the list is empty.
    orgs.value = [...orgs.value, { id: data.org_id, name: req.name, role: "owner" }];
    return data.org_id;
  }

  async function rename(orgId: string, name: string): Promise<void> {
    await api.put(`/api/orgs/${orgId}`, { name });
    const idx = orgs.value.findIndex((o) => o.id === orgId);
    if (idx >= 0) orgs.value[idx] = { ...orgs.value[idx], name };
  }

  async function transferOwnership(orgId: string, newOwnerId: string): Promise<void> {
    await api.post(`/api/orgs/${orgId}/transfer`, { new_owner_id: newOwnerId });
    const idx = orgs.value.findIndex((o) => o.id === orgId);
    if (idx >= 0) orgs.value[idx] = { ...orgs.value[idx], role: "member" };
  }

  async function remove(orgId: string): Promise<void> {
    await api.delete(`/api/orgs/${orgId}`);
    orgs.value = orgs.value.filter((o) => o.id !== orgId);
    if (currentOrgId.value === orgId) {
      currentOrgId.value = null;
      localStorage.removeItem(ORG_KEY);
    }
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

  async function listMembers(orgId: string): Promise<OrgMember[]> {
    const { data } = await api.get<OrgMember[]>(`/api/orgs/${orgId}/members`);
    return data;
  }

  async function addMember(orgId: string, req: AddOrgMemberReq): Promise<void> {
    await api.post(`/api/orgs/${orgId}/members`, req);
  }

  async function removeMember(orgId: string, userId: string): Promise<void> {
    await api.delete(`/api/orgs/${orgId}/members/${userId}`);
  }

  async function findUserByUsername(username: string): Promise<UserSearchEntry | null> {
    try {
      const { data } = await api.get<UserSearchEntry[]>(`/api/user/${encodeURIComponent(username)}`);
      // backend returns an array of up to 5 candidate matches; for an exact
      // display_name lookup we take the first row.
      return data[0] ?? null;
    } catch {
      return null;
    }
  }

  function roleIn(orgId: string): "owner" | "member" | null {
    return orgs.value.find((o) => o.id === orgId)?.role ?? null;
  }

  return {
    orgs,
    currentOrgId,
    fetch,
    create,
    rename,
    transferOwnership,
    remove,
    select,
    clear,
    listMembers,
    addMember,
    removeMember,
    findUserByUsername,
    roleIn,
  };
});
