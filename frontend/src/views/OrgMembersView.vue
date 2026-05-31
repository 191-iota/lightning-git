<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRoute, useRouter, RouterLink } from "vue-router";
import axios from "axios";
import { useOrgStore } from "@/stores/org";
import { useAuthStore } from "@/stores/auth";
import { useToastStore } from "@/stores/toast";
import { useProjectStore } from "@/stores/project";
import type { OrgMember } from "@/types/api";
import NavBar from "@/components/NavBar.vue";
import TabStrip, { type Tab } from "@/components/TabStrip.vue";
import { confirmDialog, promptDialog } from "@/utils/confirm";

const router = useRouter();
const projectStore = useProjectStore();

async function onLogout() {
  auth.logout();
  orgStore.clear();
  projectStore.clear();
  await router.push({ name: "login" });
}

const orgTabs = computed<Tab[]>(() => [
  { label: "Projects", to: { name: "dashboard" } },
  { label: "Members", to: { name: "org-members", params: { id: orgId.value } } },
]);

const route = useRoute();
const orgStore = useOrgStore();
const auth = useAuthStore();
const toast = useToastStore();

const orgId = computed(() => route.params.id as string);
const members = ref<OrgMember[]>([]);
const loading = ref(false);

const username = ref("");
const submitting = ref(false);

// the orgs list may not be loaded yet when we land here directly via URL.
// fall back to fetching if we cant resolve the current users role.
const myRole = computed(() => orgStore.roleIn(orgId.value));
const isOwner = computed(() => myRole.value === "owner");

const orgName = computed(
  () => orgStore.orgs.find((o) => o.id === orgId.value)?.name ?? "",
);

const renaming = ref(false);

async function renameOrg() {
  if (renaming.value) return;
  const next = await promptDialog({
    title: "Rename organization",
    label: "Organization name",
    defaultValue: orgName.value,
    minLength: 2,
    maxLength: 64,
    confirmLabel: "Rename",
  });
  if (next === null || next === orgName.value) return;
  renaming.value = true;
  try {
    await orgStore.rename(orgId.value, next);
    toast.success("Org renamed.");
  } catch (error) {
    toast.error(renameErrorMessage(error));
  } finally {
    renaming.value = false;
  }
}

function renameErrorMessage(error: unknown): string {
  if (!axios.isAxiosError(error)) return "Failed to rename org.";
  const status = error.response?.status;
  if (status === 401) return "Failed to rename org. You must be the owner.";
  if (status === 400) return "Name is not valid. Use 2 to 64 characters.";
  if (status && status >= 500) return "Server error renaming org.";
  return "Failed to rename org.";
}

async function promote(member: OrgMember) {
  const ok = await confirmDialog({
    title: "Transfer ownership?",
    message: `${member.display_name} will become the new owner of ${orgName.value}. You will be demoted to member.`,
    confirmLabel: "Transfer",
  });
  if (!ok) return;
  try {
    await orgStore.transferOwnership(orgId.value, member.user_id);
    toast.success(`${member.display_name} is now the owner.`);
    await refresh();
  } catch {
    toast.error("Failed to transfer ownership.");
  }
}

async function refresh() {
  loading.value = true;
  try {
    members.value = await orgStore.listMembers(orgId.value);
  } catch {
    toast.error("Could not load members.");
  } finally {
    loading.value = false;
  }
}

onMounted(async () => {
  if (orgStore.orgs.length === 0) {
    try {
      await orgStore.fetch();
    } catch {
      // we still try to load members below; if both fail the user sees
      // the inline error
    }
  }
  await refresh();
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
    if (members.value.some((m) => m.user_id === found.id)) {
      toast.error(`${found.display_name} is already a member.`);
      return;
    }
    await orgStore.addMember(orgId.value, {
      user_id: found.id,
      role: "member",
    });
    toast.success(`Added ${found.display_name}.`);
    username.value = "";
    await refresh();
  } catch (error) {
    toast.error(addMemberErrorMessage(error));
  } finally {
    submitting.value = false;
  }
}

// pull the actual reason out of the backend response so the user sees
// the real problem instead of a generic "you must be an owner" line.
function addMemberErrorMessage(error: unknown): string {
  if (!axios.isAxiosError(error)) return "Failed to add member.";
  const status = error.response?.status;
  const body = error.response?.data;
  if (status === 401) return "Failed to add member. You must be the org owner.";
  if (status === 409) return "That user is already a member of this org.";
  if (typeof body === "string" && body.trim().length > 0) {
    return `Failed to add member: ${body}`;
  }
  if (body && typeof body === "object" && "error" in body && typeof (body as { error: unknown }).error === "string") {
    return `Failed to add member: ${(body as { error: string }).error}`;
  }
  return "Failed to add member.";
}

async function remove(member: OrgMember) {
  if (member.user_id === auth.user?.id) return;
  const ok = await confirmDialog({
    title: "Remove member?",
    message: `${member.display_name} will lose access to ${orgName.value} and every project under it.`,
    confirmLabel: "Remove",
    danger: true,
  });
  if (!ok) return;
  try {
    await orgStore.removeMember(orgId.value, member.user_id);
    toast.success(`Removed ${member.display_name}.`);
    await refresh();
  } catch {
    toast.error("Failed to remove member.");
  }
}

// destructive op: drops the org row, every project under it, and every
// membership. Backend gates with require_org_owner.
const deleting = ref(false);
async function deleteOrg() {
  if (!isOwner.value) return;
  const ok = await confirmDialog({
    title: `Delete ${orgName.value}?`,
    message: "Every project, membership, task, and overlay state under this organization is removed. There is no undo.",
    confirmLabel: "Delete org",
    danger: true,
  });
  if (!ok) return;
  deleting.value = true;
  try {
    await orgStore.remove(orgId.value);
    toast.success(`${orgName.value} deleted.`);
    await router.push({ name: "orgs" });
  } catch {
    toast.error("Failed to delete org.");
  } finally {
    deleting.value = false;
  }
}
</script>

<template>
  <div class="min-h-screen bg-lg-bg text-lg-text">
    <NavBar>
      <RouterLink to="/pricing" class="lg-link">Pricing</RouterLink>
      <RouterLink to="/orgs" class="lg-link">Switch org</RouterLink>
      <span class="text-lg-text-muted hidden sm:inline">{{ auth.user?.email }}</span>
      <button class="lg-link" @click="onLogout">Sign out</button>
    </NavBar>

    <main class="lg-container py-10">
      <header class="mb-6 flex items-end justify-between gap-4">
        <div class="flex flex-col gap-2 items-start">
          <span class="lg-scope lg-scope-org">Organization</span>
          <div class="flex items-center gap-3">
            <h1 class="text-3xl font-bold">{{ orgName || "Members" }}</h1>
            <button
              v-if="isOwner"
              type="button"
              class="lg-btn-secondary text-xs px-3 py-1.5 disabled:opacity-50"
              :disabled="renaming"
              @click="renameOrg"
              title="Rename org"
            >
              {{ renaming ? "Saving..." : "Rename" }}
            </button>
          </div>
        </div>
        <span
          v-if="myRole"
          class="text-xs"
          :class="myRole === 'owner' ? 'text-lg-accent-bright' : 'text-lg-text-muted'"
        >
          you are {{ myRole }}
        </span>
      </header>

      <TabStrip :tabs="orgTabs" class="mb-8" />

      <section v-if="isOwner" class="lg-card p-6 mb-6">
        <h2 class="text-base font-medium mb-4">Invite a member</h2>
        <form class="flex flex-col sm:flex-row gap-3" @submit.prevent="submitInvite">
          <input
            v-model="username"
            type="text"
            placeholder="username"
            class="lg-input flex-1"
            required
            minlength="3"
            maxlength="32"
            autocomplete="off"
          />
          <button type="submit" class="lg-btn-primary disabled:opacity-50" :disabled="submitting">
            {{ submitting ? "Adding..." : "Add" }}
          </button>
        </form>
      </section>

      <section>
        <div class="flex items-end justify-between mb-3">
          <h2 class="text-lg font-semibold">Members</h2>
          <span class="text-sm text-lg-text-muted">{{ members.length }} total</span>
        </div>

        <div v-if="loading && members.length === 0" class="lg-card p-6 text-sm text-lg-text-sec">
          Loading members...
        </div>

        <ul v-else-if="members.length > 0" class="space-y-2">
          <li
            v-for="m in members"
            :key="m.user_id"
            class="lg-card p-4 flex items-center gap-4"
          >
            <div class="flex-1 min-w-0">
              <p class="text-lg-text flex items-baseline gap-2.5 flex-wrap">
                <span class="font-medium">{{ m.display_name }}</span>
                <span
                  class="text-xs"
                  :class="m.role === 'owner' ? 'text-lg-accent-bright' : 'text-lg-text-muted'"
                >{{ m.role }}</span>
                <span v-if="m.user_id === auth.user?.id" class="text-xs text-lg-text-muted">you</span>
              </p>
              <p class="text-xs text-lg-text-muted font-mono mt-1 truncate">{{ m.user_id }}</p>
            </div>
            <button
              v-if="isOwner && m.user_id !== auth.user?.id && m.role !== 'owner'"
              type="button"
              class="lg-btn-secondary text-xs px-3 py-1.5 hover:border-lg-accent-bright hover:text-lg-accent-bright transition-colors"
              @click="promote(m)"
              :title="'Transfer ownership to ' + m.display_name"
            >
              Make owner
            </button>
            <button
              v-if="isOwner && m.user_id !== auth.user?.id"
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

      <section v-if="isOwner" class="mt-12 border-t border-lg-border pt-6">
        <h2 class="text-sm font-mono uppercase tracking-wider text-lg-rose mb-2">Danger zone</h2>
        <div class="lg-card p-4 flex items-center justify-between gap-4 border-lg-rose/40">
          <div>
            <p class="text-sm font-medium">Delete this organization</p>
            <p class="text-xs text-lg-text-sec mt-1">
              Removes the org and all of its projects, memberships, tasks, and live overlay state.
            </p>
          </div>
          <button
            type="button"
            class="lg-btn-secondary text-xs px-3 py-2 border-lg-rose/60 text-lg-rose hover:bg-lg-rose/10 hover:border-lg-rose disabled:opacity-50"
            :disabled="deleting"
            @click="deleteOrg"
          >
            {{ deleting ? "Deleting..." : "Delete org" }}
          </button>
        </div>
      </section>
    </main>
  </div>
</template>
