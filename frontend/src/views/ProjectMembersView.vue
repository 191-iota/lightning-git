<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter, RouterLink } from "vue-router";
import axios from "axios";
import { useProjectStore } from "@/stores/project";
import { useOrgStore } from "@/stores/org";
import { useAuthStore } from "@/stores/auth";
import { useToastStore } from "@/stores/toast";
import type { OrgMember, ProjectMember } from "@/types/api";
import NavBar from "@/components/NavBar.vue";
import UserMenu from "@/components/UserMenu.vue";
import TabStrip, { type Tab } from "@/components/TabStrip.vue";
import { confirmDialog } from "@/utils/confirm";

const router = useRouter();

async function onLogout() {
  auth.logout();
  orgStore.clear();
  projectStore.clear();
  await router.push({ name: "login" });
}

const projectTabs = computed<Tab[]>(() => [
  { label: "Board", to: { name: "project", params: { id: projectId.value } } },
  { label: "Live view", to: { name: "overlay", params: { id: projectId.value }, query: { branch: "main" } } },
  { label: "Members", to: { name: "project-members", params: { id: projectId.value } } },
]);

const parentOrg = computed(() => {
  const orgId = projectStore.current?.org_id;
  if (!orgId) return null;
  const o = orgStore.orgs.find((x) => x.id === orgId);
  return o ? { id: o.id, name: o.name } : null;
});

const route = useRoute();
const projectStore = useProjectStore();
const orgStore = useOrgStore();
const auth = useAuthStore();
const toast = useToastStore();

const projectId = computed(() => route.params.id as string);
const loading = ref(false);

const username = ref("");
const role = ref<"member" | "admin">("member");
const submitting = ref(false);

const myRole = computed(() => projectStore.myRole(auth.user?.id));
const isAdmin = computed(() => myRole.value === "admin");

const projectName = computed(() => projectStore.current?.name ?? "");
const members = computed<ProjectMember[]>(() => projectStore.members);

// org members not yet in the project, used by the quick-add chip row.
const orgMembers = ref<OrgMember[]>([]);
const orgMembersLoading = ref(false);
const candidatePool = computed(() => {
  const inProject = new Set(members.value.map((m) => m.id));
  return orgMembers.value.filter((m) => !inProject.has(m.user_id));
});

async function loadOrgMembers() {
  const orgId = projectStore.current?.org_id;
  if (!orgId) return;
  orgMembersLoading.value = true;
  try {
    orgMembers.value = await orgStore.listMembers(orgId);
  } catch {
    orgMembers.value = [];
  } finally {
    orgMembersLoading.value = false;
  }
}

watch(
  () => projectStore.current?.org_id,
  (orgId) => {
    if (orgId) void loadOrgMembers();
  },
);

async function refresh() {
  loading.value = true;
  try {
    await projectStore.fetchMembers(projectId.value);
  } catch {
    toast.error("Could not load project members.");
  } finally {
    loading.value = false;
  }
}

onMounted(async () => {
  if (!projectStore.current || projectStore.current.id !== projectId.value) {
    try {
      await projectStore.get(projectId.value);
    } catch {
      // not fatal, fetchMembers below will surface the real error
    }
  }
  await refresh();
  await loadOrgMembers();
});

async function submitInvite() {
  const handle = username.value.trim();
  if (!handle) return;
  submitting.value = true;
  try {
    const found = await orgStore.findUserByUsername(handle);
    if (!found) {
      toast.error(`No user named "${handle}".`);
      return;
    }
    if (members.value.some((m) => m.id === found.id)) {
      toast.error(`${found.display_name} is already a project member.`);
      return;
    }
    await projectStore.addMember(projectId.value, { user_id: found.id, role: role.value });
    toast.success(`Added ${found.display_name} as ${role.value}.`);
    username.value = "";
    role.value = "member";
    await refresh();
    await loadOrgMembers();
  } catch (error) {
    toast.error(addMemberErrorMessage(error));
  } finally {
    submitting.value = false;
  }
}

async function quickAdd(m: OrgMember, asRole: "member" | "admin") {
  submitting.value = true;
  try {
    await projectStore.addMember(projectId.value, { user_id: m.user_id, role: asRole });
    toast.success(`Added ${m.display_name} as ${asRole}.`);
    await refresh();
    await loadOrgMembers();
  } catch (error) {
    toast.error(addMemberErrorMessage(error));
  } finally {
    submitting.value = false;
  }
}

function addMemberErrorMessage(error: unknown): string {
  if (!axios.isAxiosError(error)) return "Failed to add member.";
  const status = error.response?.status;
  const body = error.response?.data;
  if (status === 401) return "Failed to add member. You must be a project admin.";
  if (status === 409) return "That user is already a project member.";
  if (typeof body === "string" && body.trim().length > 0) {
    return `Failed to add member: ${body}`;
  }
  return "Failed to add member.";
}

async function remove(member: ProjectMember) {
  const ok = await confirmDialog({
    title: "Remove member?",
    message: `${member.display_name} will lose access to ${projectName.value}.`,
    confirmLabel: "Remove",
    danger: true,
  });
  if (!ok) return;
  try {
    await projectStore.removeMember(projectId.value, member.id);
    toast.success(`Removed ${member.display_name}.`);
    await refresh();
  } catch {
    toast.error("Failed to remove member.");
  }
}

// destructive op: drops the project row, every membership, the local git
// mirror, and every overlay. Backend gates with require_project_admin.
const deleting = ref(false);
async function deleteProject() {
  if (!isAdmin.value) return;
  const ok = await confirmDialog({
    title: `Delete ${projectName.value}?`,
    message: "The local git mirror, every task, every membership, and every active overlay are removed. There is no undo.",
    confirmLabel: "Delete project",
    danger: true,
  });
  if (!ok) return;
  deleting.value = true;
  try {
    await projectStore.remove(projectId.value);
    toast.success(`${projectName.value} deleted.`);
    await router.push({ name: "dashboard" });
  } catch {
    toast.error("Failed to delete project.");
  } finally {
    deleting.value = false;
  }
}
</script>

<template>
  <div class="min-h-screen bg-lg-bg text-lg-text">
    <NavBar>
      <RouterLink to="/orgs" class="lg-link">Switch org</RouterLink>
      <UserMenu @logout="onLogout" />
    </NavBar>

    <main class="lg-container py-10">
      <RouterLink to="/dashboard" class="lg-breadcrumb mb-3">
        &larr; {{ parentOrg?.name ? `${parentOrg.name} / Projects` : "Projects" }}
      </RouterLink>

      <header class="mt-3 mb-6 min-h-[6rem] flex items-center justify-between gap-4">
        <div class="flex flex-col gap-2 items-start">
          <span class="lg-scope lg-scope-project">Project</span>
          <h1 class="text-3xl font-bold">{{ projectName || "Members" }}</h1>
          <p class="text-[0.7rem] text-lg-text-muted/80 font-mono min-h-[1rem]">&nbsp;</p>
        </div>
        <span
          v-if="myRole"
          class="text-xs"
          :class="myRole === 'admin' ? 'text-lg-accent-bright' : 'text-lg-text-muted'"
        >
          you are {{ myRole }}
        </span>
      </header>

      <TabStrip :tabs="projectTabs" class="mb-6" />

      <section v-if="isAdmin" class="lg-card p-6 mb-6 space-y-5">
        <div>
          <h2 class="text-base font-medium mb-1">Add from org</h2>
          <p class="text-xs text-lg-text-muted mb-3">
            Only org members can be added to this project. Click a name to add them as
            <span class="text-lg-text">{{ role }}</span>.
          </p>

          <div v-if="orgMembersLoading" class="text-xs text-lg-text-muted">Loading org members...</div>
          <div v-else-if="candidatePool.length === 0" class="text-xs text-lg-text-muted">
            Everyone in this org is already in the project.
          </div>
          <ul v-else class="flex flex-wrap gap-2">
            <li v-for="m in candidatePool" :key="m.user_id">
              <button
                type="button"
                class="lg-btn-secondary text-xs px-3 py-1.5 disabled:opacity-50"
                :disabled="submitting"
                @click="quickAdd(m, role)"
                :title="'Add ' + m.display_name + ' as ' + role"
              >
                + {{ m.display_name }}
                <span class="text-lg-text-muted">{{ m.role === 'owner' ? '(org owner)' : '' }}</span>
              </button>
            </li>
          </ul>

          <div class="flex items-center gap-2 mt-3">
            <span class="text-xs uppercase tracking-wider text-lg-text-sec">Add as</span>
            <select v-model="role" class="lg-input text-xs py-1 max-w-[140px]">
              <option value="member">Member</option>
              <option value="admin">Admin</option>
            </select>
          </div>
        </div>

        <div class="border-t border-lg-border pt-4">
          <h3 class="text-xs uppercase tracking-wider text-lg-text-sec font-medium mb-2">
            Or by username
          </h3>
          <form class="flex flex-col sm:flex-row gap-3" @submit.prevent="submitInvite">
            <input
              v-model="username"
              type="text"
              placeholder="username"
              class="lg-input flex-1"
              minlength="3"
              maxlength="32"
              autocomplete="off"
            />
            <button type="submit" class="lg-btn-primary disabled:opacity-50" :disabled="submitting">
              {{ submitting ? "Adding..." : "Add" }}
            </button>
          </form>
        </div>
      </section>

      <section>
        <div class="flex items-end justify-between mb-3">
          <h2 class="text-lg font-semibold">Members</h2>
          <span class="text-sm text-lg-text-muted">{{ members.length }} total</span>
        </div>

        <div v-if="loading && members.length === 0" class="space-y-2">
          <div
            v-for="i in 3"
            :key="i"
            class="lg-card p-4 flex items-center gap-4 animate-pulse"
          >
            <div class="flex-1 space-y-2">
              <div class="h-3 w-32 bg-lg-surface-2 rounded"></div>
              <div class="h-2 w-64 bg-lg-surface-2 rounded"></div>
            </div>
          </div>
        </div>

        <ul v-else-if="members.length > 0" class="space-y-2">
          <li
            v-for="m in members"
            :key="m.id"
            class="lg-card p-4 flex items-center gap-4"
          >
            <div class="flex-1 min-w-0">
              <p class="text-lg-text flex items-baseline gap-2.5 flex-wrap">
                <span class="font-medium">{{ m.display_name }}</span>
                <span
                  class="text-xs"
                  :class="m.role === 'admin' ? 'text-lg-accent-bright' : 'text-lg-text-muted'"
                >{{ m.role }}</span>
                <span v-if="m.id === auth.user?.id" class="text-xs text-lg-text-muted">you</span>
              </p>
              <p class="text-xs text-lg-text-muted font-mono mt-1 truncate">{{ m.id }}</p>
            </div>
            <button
              v-if="isAdmin"
              type="button"
              class="lg-btn-secondary text-xs px-3 py-1.5 hover:border-lg-rose hover:text-lg-rose transition-colors"
              @click="remove(m)"
              :title="'Remove ' + m.display_name"
            >
              Remove
            </button>
          </li>
        </ul>

        <div v-else class="lg-card p-6 text-sm text-lg-text-sec">No members yet.</div>
      </section>

      <section v-if="isAdmin" class="mt-12 border-t border-lg-border pt-6">
        <h2 class="text-sm font-mono uppercase tracking-wider text-lg-rose mb-2">Danger zone</h2>
        <div class="lg-card p-4 flex items-center justify-between gap-4 border-lg-rose/40">
          <div>
            <p class="text-sm font-medium">Delete this project</p>
            <p class="text-xs text-lg-text-sec mt-1">
              Removes the project record, all memberships, tasks, the local git mirror, and any active overlay state.
            </p>
          </div>
          <button
            type="button"
            class="lg-btn-secondary text-xs px-3 py-2 border-lg-rose/60 text-lg-rose hover:bg-lg-rose/10 hover:border-lg-rose disabled:opacity-50"
            :disabled="deleting"
            @click="deleteProject"
          >
            {{ deleting ? "Deleting..." : "Delete project" }}
          </button>
        </div>
      </section>
    </main>
  </div>
</template>
