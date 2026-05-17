<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRoute, RouterLink } from "vue-router";
import { useProjectStore } from "@/stores/project";
import type { Task, TaskType } from "@/types/api";

const route = useRoute();
const projectStore = useProjectStore();
const error = ref<string | null>(null);

const projectId = computed(() => route.params.id as string);

const columns: TaskType[] = ["Feature", "Improvement", "Bug", "Unknown"];

// task_type is derived from branch name on the backend, so no drag-drop persists
const tasksByType = computed(() => {
  const grouped: Record<TaskType, Task[]> = { Bug: [], Feature: [], Improvement: [], Unknown: [] };
  for (const task of projectStore.tasks) {
    grouped[task.task_type]?.push(task);
  }
  return grouped;
});

onMounted(async () => {
  try {
    await projectStore.get(projectId.value);
    await projectStore.fetchTasks(projectId.value);
  } catch {
    error.value = "Failed to load project";
  }
});
</script>

<template>
  <div class="min-h-screen bg-zinc-950 text-zinc-100 px-6 py-8">
    <header class="flex items-center justify-between mb-6">
      <div>
        <RouterLink to="/dashboard" class="text-sm text-zinc-500 hover:text-zinc-300">
          &larr; Dashboard
        </RouterLink>
        <h1 class="text-xl font-semibold mt-1">
          {{ projectStore.current?.name || "Loading..." }}
        </h1>
        <p v-if="projectStore.current" class="text-xs text-zinc-500">
          {{ projectStore.current.repo_url }}
        </p>
      </div>
    </header>

    <p v-if="error" class="text-sm text-red-400 mb-4">{{ error }}</p>

    <div class="grid grid-cols-4 gap-4">
      <div
        v-for="col in columns"
        :key="col"
        class="bg-zinc-900 border border-zinc-800 rounded p-3 min-h-[200px]"
      >
        <h2 class="text-sm font-medium mb-3 text-zinc-300">
          {{ col }}
          <span class="text-zinc-500">({{ tasksByType[col].length }})</span>
        </h2>
        <ul class="space-y-2">
          <li
            v-for="task in tasksByType[col]"
            :key="task.id"
            class="bg-zinc-800 border border-zinc-700 rounded p-2"
          >
            <p class="text-sm font-medium truncate">{{ task.name }}</p>
            <p class="text-xs text-zinc-500 truncate">{{ task.branch_name }}</p>
          </li>
        </ul>
      </div>
    </div>
  </div>
</template>
