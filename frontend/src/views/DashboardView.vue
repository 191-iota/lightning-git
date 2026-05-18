<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useRouter, RouterLink } from "vue-router";
import { useAuthStore } from "@/stores/auth";
import { useOrgStore } from "@/stores/org";
import { useProjectStore } from "@/stores/project";

const authStore = useAuthStore();
const orgStore = useOrgStore();
const projectStore = useProjectStore();
const router = useRouter();
const error = ref<string | null>(null);

// project_id -> number of active editors right now
const activeCounts = ref<Record<string, number>>({});

onMounted(async () => {
  if (!orgStore.currentOrgId) return;
  try {
    await projectStore.fetch(orgStore.currentOrgId);
    // pull current activity per project so dashboard dots reflect realtime state
    await Promise.all(
      projectStore.projects.map(async (p) => {
        const edits = await projectStore.fetchActivity(p.id).catch(() => []);
        activeCounts.value[p.id] = edits.length;
      }),
    );
  } catch {
    error.value = "Failed to load projects";
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
  <div class="min-h-screen bg-zinc-950 text-zinc-100 px-6 py-8">
    <header class="flex items-center justify-between mb-8">
      <h1 class="text-xl font-semibold">Lightning Git</h1>
      <div class="flex items-center gap-4">
        <span class="text-sm text-zinc-400">{{ authStore.user?.email }}</span>
        <RouterLink to="/orgs" class="text-sm text-zinc-300 hover:text-zinc-100">
          Switch org
        </RouterLink>
        <button class="text-sm text-zinc-300 hover:text-zinc-100" @click="onLogout">
          Sign out
        </button>
      </div>
    </header>

    <main>
      <div class="flex items-center justify-between mb-4">
        <h2 class="text-lg font-medium">Projects</h2>
        <RouterLink
          to="/projects/new"
          class="bg-zinc-100 text-zinc-900 rounded px-3 py-1.5 text-sm font-medium"
        >
          Create project
        </RouterLink>
      </div>

      <p v-if="error" class="text-sm text-red-400 mb-4">{{ error }}</p>

      <div v-if="projectStore.projects.length === 0 && !error" class="text-zinc-500">
        No projects yet.
      </div>

      <ul class="space-y-2">
        <RouterLink
          v-for="project in projectStore.projects"
          :key="project.id"
          :to="{ name: 'project', params: { id: project.id } }"
          class="block border border-zinc-800 rounded p-3 hover:bg-zinc-900"
        >
          <div class="flex items-center gap-2">
            <span
              class="inline-block w-2 h-2 rounded-full flex-shrink-0"
              :class="activeCounts[project.id] ? 'bg-amber-400' : 'bg-zinc-700'"
              :title="
                activeCounts[project.id]
                  ? `${activeCounts[project.id]} active edits`
                  : 'no live activity'
              "
            ></span>
            <p class="font-medium">{{ project.name }}</p>
            <span
              v-if="activeCounts[project.id]"
              class="text-xs text-amber-400 ml-1"
            >
              {{ activeCounts[project.id] }} editing
            </span>
          </div>
          <p class="text-xs text-zinc-500 mt-1">{{ project.repo_url }}</p>
        </RouterLink>
      </ul>
    </main>
  </div>
</template>
