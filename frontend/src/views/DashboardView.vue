<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRouter, RouterLink } from "vue-router";
import { useAuthStore } from "@/stores/auth";
import { useOrgStore } from "@/stores/org";
import { useProjectStore } from "@/stores/project";
import { useToastStore } from "@/stores/toast";
import NavBar from "@/components/NavBar.vue";
import Skeleton from "@/components/Skeleton.vue";
import TabStrip, { type Tab } from "@/components/TabStrip.vue";

const toast = useToastStore();
const loading = ref(true);

const currentOrgName = computed(
  () =>
    orgStore.orgs.find((o) => o.id === orgStore.currentOrgId)?.name ??
    "Organization",
);
const orgTabs = computed<Tab[]>(() =>
  orgStore.currentOrgId
    ? [
        { label: "Projects", to: { name: "dashboard" } },
        { label: "Members", to: { name: "org-members", params: { id: orgStore.currentOrgId } } },
      ]
    : [],
);

const authStore = useAuthStore();
const orgStore = useOrgStore();
const projectStore = useProjectStore();
const router = useRouter();

// project_id -> number of active editors right now
const activeCounts = ref<Record<string, number>>({});

onMounted(async () => {
  if (!orgStore.currentOrgId) return;
  loading.value = true;
  try {
    // hydrate the orgs list so currentOrgName resolves to the real name
    // instead of the "Organization" fallback. on cold loads (refresh, deep
    // link) the dashboard mounts before the org list has been fetched.
    if (orgStore.orgs.length === 0) {
      await orgStore.fetch().catch(() => undefined);
    }
    await projectStore.fetch(orgStore.currentOrgId);
    // pull current activity per project so dashboard dots reflect realtime state
    await Promise.all(
      projectStore.projects.map(async (p) => {
        const edits = await projectStore.fetchActivity(p.id).catch(() => []);
        activeCounts.value[p.id] = edits.length;
      }),
    );
  } catch {
    toast.error("Could not load projects.");
  } finally {
    loading.value = false;
  }
});

async function onLogout() {
  authStore.logout();
  orgStore.clear();
  projectStore.clear();
  await router.push({ name: "login" });
}
</script>

<template>
  <div class="min-h-screen bg-lg-bg text-lg-text">
    <NavBar>
      <RouterLink to="/pricing" class="lg-link">Pricing</RouterLink>
      <RouterLink to="/orgs" class="lg-link">Switch org</RouterLink>
      <span class="text-lg-text-muted hidden sm:inline">{{ authStore.user?.email }}</span>
      <button class="lg-link" @click="onLogout">Sign out</button>
    </NavBar>

    <main class="lg-container py-10">
      <header class="mb-6 flex items-end justify-between gap-4">
        <div class="flex flex-col gap-2 items-start">
          <span class="lg-scope lg-scope-org">Organization</span>
          <h1 class="text-3xl font-bold">{{ currentOrgName }}</h1>
        </div>
        <RouterLink
          v-if="projectStore.projects.length > 0"
          to="/projects/new"
          class="lg-btn-primary"
        >
          New project
        </RouterLink>
      </header>

      <TabStrip :tabs="orgTabs" class="mb-8" />

      <div class="flex items-end justify-between mb-4">
        <h2 class="text-lg font-semibold">Projects</h2>
        <span class="text-sm text-lg-text-muted">
          {{ projectStore.projects.length }} total
        </span>
      </div>

      <ul v-if="loading" class="grid grid-cols-1 sm:grid-cols-2 gap-4">
        <li v-for="i in 4" :key="i" class="lg-card p-5 space-y-3">
          <div class="flex items-center gap-2">
            <Skeleton width="0.5rem" height="0.5rem" rounded="full" />
            <Skeleton width="60%" height="0.875rem" />
          </div>
          <Skeleton width="80%" height="0.625rem" />
        </li>
      </ul>

      <div
        v-else-if="projectStore.projects.length === 0"
        class="lg-card p-10 text-center"
      >
        <p class="text-lg-text-sec mb-5 text-sm">No projects in this organization.</p>
        <RouterLink to="/projects/new" class="lg-btn-primary inline-flex">
          New project
        </RouterLink>
      </div>

      <ul v-else-if="projectStore.projects.length > 0" class="grid grid-cols-1 sm:grid-cols-2 gap-4">
        <RouterLink
          v-for="project in projectStore.projects"
          :key="project.id"
          :to="{ name: 'project', params: { id: project.id } }"
          class="lg-card p-5 hover:border-lg-border-strong hover:bg-lg-surface-2 transition-colors group"
        >
          <div class="flex items-center gap-2 mb-1">
            <span
              class="inline-block w-2 h-2 rounded-full flex-shrink-0"
              :class="activeCounts[project.id] ? 'bg-lg-accent-bright animate-pulse' : 'bg-lg-text-muted'"
              :title="
                activeCounts[project.id]
                  ? `${activeCounts[project.id]} ${activeCounts[project.id] === 1 ? 'file' : 'files'} being edited`
                  : 'no live activity'
              "
            ></span>
            <p class="font-semibold text-lg-text group-hover:text-lg-accent-bright transition-colors">
              {{ project.name }}
            </p>
            <span
              v-if="activeCounts[project.id]"
              class="text-xs text-lg-accent-bright ml-auto"
            >
              {{ activeCounts[project.id] }} {{ activeCounts[project.id] === 1 ? "file" : "files" }} edited
            </span>
          </div>
          <p class="text-xs text-lg-text-muted truncate">{{ project.repo_url }}</p>
        </RouterLink>
      </ul>
    </main>
  </div>
</template>
