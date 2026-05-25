<script setup lang="ts">
import { computed, nextTick, onMounted, ref } from "vue";
import { useRoute, useRouter, RouterLink } from "vue-router";
import axios from "axios";
import { useOrgStore } from "@/stores/org";
import { useAuthStore } from "@/stores/auth";
import { useToastStore } from "@/stores/toast";
import { useProjectStore } from "@/stores/project";
import type { OrgMember } from "@/types/api";
import NavBar from "@/components/NavBar.vue";
import TabStrip, { type Tab } from "@/components/TabStrip.vue";

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

const editing = ref(false);
const draftName = ref("");
const renaming = ref(false);
const nameInput = ref<HTMLInputElement | null>(null);

async function startEditName() {
  draftName.value = orgName.value;
  editing.value = true;
  await nextTick();
  nameInput.value?.focus();
  nameInput.value?.select();
}

function cancelEditName() {
  editing.value = false;
}

async function commitName() {
  if (!editing.value) return;
  const next = draftName.value.trim();
  if (!next || next === orgName.value) {
    editing.value = false;
    return;
  }
  renaming.value = true;
  try {
    await orgStore.rename(orgId.value, next);
    toast.success("Org renamed.");
    editing.value = false;
  } catch {
    toast.error("Failed to rename org.");
  } finally {
    renaming.value = false;
  }
}

async function promote(member: OrgMember) {
  if (!confirm(
    `Transfer ownership of ${orgName.value} to ${member.display_name}? You will be demoted to member.`,
  )) return;
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
  if (!confirm(`Remove ${member.display_name} from ${orgName.value}?`)) return;
  try {
    await orgStore.removeMember(orgId.value, member.user_id);
    toast.success(`Removed ${member.display_name}.`);
    await refresh();
  } catch {
    toast.error("Failed to remove member.");
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
          <div v-if="editing" class="flex items-center gap-2">
            <input
              ref="nameInput"
              v-model="draftName"
              minlength="2"
              maxlength="64"
              class="text-3xl font-bold bg-transparent border-0 border-b-2 border-lg-accent-bright/70 focus:border-lg-accent-bright focus:outline-none px-0 py-0 -mb-[2px] min-w-[10rem]"
              @keydown.enter="commitName"
              @keydown.esc="cancelEditName"
              @blur="commitName"
            />
            <span v-if="renaming" class="text-xs text-lg-text-muted">saving</span>
          </div>
          <div v-else class="group flex items-center gap-2">
            <h1 class="text-3xl font-bold">{{ orgName || "Members" }}</h1>
            <button
              v-if="isOwner"
              type="button"
              class="opacity-0 group-hover:opacity-100 text-[0.7rem] uppercase tracking-wider text-lg-text-muted hover:text-lg-accent-bright transition-opacity"
              @click="startEditName"
              title="Rename org"
            >rename</button>
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
    </main>
  </div>
</template>
