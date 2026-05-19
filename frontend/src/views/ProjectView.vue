<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { useRoute, useRouter, RouterLink } from "vue-router";
import draggable from "vuedraggable";
import { useAuthStore } from "@/stores/auth";
import { useProjectStore } from "@/stores/project";
import type { ActiveEdit, Task } from "@/types/api";

const route = useRoute();
const router = useRouter();
const auth = useAuthStore();
const projectStore = useProjectStore();
const error = ref<string | null>(null);

const projectId = computed(() => route.params.id as string);

// hide pre-existing main/master task rows that were created before the
// backend learned to skip integration branches. defense-in-depth.
const visibleTasks = computed(() =>
  projectStore.tasks.filter((t) => t.branch_name !== "main" && t.branch_name !== "master"),
);

function openOverlay(branch: string) {
  router.push({ name: "overlay", params: { id: projectId.value }, query: { branch } });
}

const projectEdits = ref<ActiveEdit[]>([]);
const wsState = ref<"closed" | "connecting" | "live">("closed");
let activityWs: WebSocket | null = null;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let handshakeTimeout: ReturnType<typeof setTimeout> | null = null;

function connectActivity() {
  if (!auth.token) {
    console.warn("[activity ws] skipped: no auth token");
    wsState.value = "closed";
    return;
  }
  const base = import.meta.env.VITE_WS_URL || "ws://localhost:8080";
  const url = `${base}/api/projects/${projectId.value}/activity/ws?token=${encodeURIComponent(auth.token)}`;
  console.log("[activity ws] connecting", url);
  wsState.value = "connecting";
  activityWs = new WebSocket(url);

  handshakeTimeout = setTimeout(() => {
    if (wsState.value === "connecting") {
      console.warn("[activity ws] handshake timed out after 10s, forcing close");
      activityWs?.close();
    }
  }, 10000);

  activityWs.onopen = () => {
    if (handshakeTimeout) clearTimeout(handshakeTimeout);
    console.log("[activity ws] open");
    wsState.value = "live";
  };
  activityWs.onmessage = (event) => {
    try { projectEdits.value = JSON.parse(event.data) as ActiveEdit[]; } catch { /* ignore */ }
  };
  activityWs.onerror = (e) => {
    console.error("[activity ws] error", e);
  };
  activityWs.onclose = (e) => {
    if (handshakeTimeout) clearTimeout(handshakeTimeout);
    console.warn("[activity ws] closed", e.code, e.reason);
    wsState.value = "closed";
    reconnectTimer = setTimeout(connectActivity, 3000);
  };
}

// Board state. Local arrays so vuedraggable can mutate them on drag.
// Once a user has dragged a task, it stays where they put it for the session
// (auto-derived state only seeds the board on first load and adds new tasks).
type ColumnId = "todo" | "in_progress";
const board = ref<Record<ColumnId, Task[]>>({
  todo: [],
  in_progress: [],
});
const placed = ref<Set<string>>(new Set());

// Existence of a non-main branch is the signal — every auto-derived task
// starts in In Progress. To Do is a manual-drag parking lot.
function naturalColumnFor(_t: Task): ColumnId {
  return "in_progress";
}

// seed / reconcile the board against fetched tasks. only add new ones; never
// move ones already on the board (so manual drags are preserved within a session).
function reconcileBoard() {
  const seen = new Set<string>();
  for (const t of visibleTasks.value) {
    seen.add(t.id);
    if (!placed.value.has(t.id)) {
      board.value[naturalColumnFor(t)].push(t);
      placed.value.add(t.id);
    }
  }
  // remove tasks that disappeared server-side
  for (const col of Object.keys(board.value) as ColumnId[]) {
    board.value[col] = board.value[col].filter((t) => seen.has(t.id));
  }
}

watch(visibleTasks, reconcileBoard, { deep: true });

function editorsForBranch(branch: string): ActiveEdit[] {
  return projectEdits.value.filter((e) => e.branch === branch);
}

function typeColor(t: Task["task_type"]): string {
  if (t === "Feature") return "bg-sky-900/50 text-sky-300";
  if (t === "Bug") return "bg-red-900/50 text-red-300";
  if (t === "Improvement") return "bg-emerald-900/50 text-emerald-300";
  return "bg-zinc-800 text-zinc-400";
}

onMounted(async () => {
  try {
    await projectStore.get(projectId.value);
    await projectStore.fetchTasks(projectId.value);
    projectStore.fetchMembers(projectId.value).catch(() => undefined);
    reconcileBoard();
    connectActivity();
  } catch {
    error.value = "Failed to load project";
  }
});

onUnmounted(() => {
  if (reconnectTimer) clearTimeout(reconnectTimer);
  activityWs?.close();
  activityWs = null;
});
</script>

<template>
  <div class="min-h-screen bg-zinc-950 text-zinc-100 px-6 py-8">
    <header class="flex items-center justify-between mb-6">
      <div>
        <RouterLink to="/dashboard" class="text-sm text-zinc-500 hover:text-zinc-300">
          &larr; Projects
        </RouterLink>
        <h1 class="text-xl font-semibold mt-1">
          {{ projectStore.current?.name || "Loading..." }}
        </h1>
        <p v-if="projectStore.current" class="text-xs text-zinc-500">
          {{ projectStore.current.repo_url }}
        </p>
      </div>
      <div class="flex items-center gap-4">
        <span class="flex items-center gap-1.5 text-xs text-zinc-500">
          <span
            class="inline-block w-2 h-2 rounded-full"
            :class="{
              'bg-emerald-400': wsState === 'live',
              'bg-amber-400 animate-pulse': wsState === 'connecting',
              'bg-red-400': wsState === 'closed',
            }"
          ></span>
          {{ wsState }}
        </span>
        <RouterLink
          :to="{ name: 'overlay', params: { id: projectId }, query: { branch: 'main' } }"
          class="text-sm text-zinc-300 hover:text-zinc-100 underline"
        >
          Live view
        </RouterLink>
      </div>
    </header>

    <p v-if="error" class="text-sm text-red-400 mb-3">{{ error }}</p>

    <div class="grid grid-cols-2 gap-3">
      <!-- To Do -->
      <section class="border border-zinc-800 rounded p-3 min-h-[300px] bg-zinc-900/30">
        <h2 class="text-sm font-medium text-zinc-300 mb-3">
          To Do <span class="text-zinc-500">({{ board.todo.length }})</span>
        </h2>
        <draggable
          v-model="board.todo"
          group="kanban"
          item-key="id"
          class="space-y-2 min-h-[200px]"
        >
          <template #item="{ element: task }">
            <div
              class="bg-zinc-900 border border-zinc-800 rounded p-2 cursor-pointer hover:border-zinc-600"
              @click="openOverlay(task.branch_name)"
            >
              <p class="text-sm font-medium truncate">{{ task.name || task.branch_name }}</p>
              <p class="text-xs text-zinc-500 truncate">{{ task.branch_name }}</p>
              <span class="mt-1 inline-block text-xs px-1.5 py-0.5 rounded" :class="typeColor(task.task_type)">
                {{ task.task_type }}
              </span>
            </div>
          </template>
        </draggable>
      </section>

      <!-- In Progress -->
      <section class="border border-amber-900/50 rounded p-3 min-h-[300px] bg-amber-900/10">
        <h2 class="text-sm font-medium text-zinc-300 mb-3 flex items-center gap-2">
          <span class="inline-block w-2 h-2 rounded-full bg-amber-400"></span>
          In Progress
          <span class="text-zinc-500">({{ board.in_progress.length }})</span>
        </h2>
        <draggable
          v-model="board.in_progress"
          group="kanban"
          item-key="id"
          class="space-y-2 min-h-[200px]"
        >
          <template #item="{ element: task }">
            <div
              class="bg-zinc-900 border border-amber-900/30 rounded p-2 cursor-pointer hover:border-amber-700/50"
              @click="openOverlay(task.branch_name)"
            >
              <p class="text-sm font-medium truncate">{{ task.name || task.branch_name }}</p>
              <p class="text-xs text-zinc-500 truncate">{{ task.branch_name }}</p>
              <ul class="mt-1 space-y-0.5">
                <li
                  v-for="e in editorsForBranch(task.branch_name)"
                  :key="e.user_id + e.file"
                  class="text-xs text-amber-200 truncate"
                >
                  {{ projectStore.memberName(e.user_id) }} — {{ e.file }}
                </li>
              </ul>
              <span class="mt-1 inline-block text-xs px-1.5 py-0.5 rounded" :class="typeColor(task.task_type)">
                {{ task.task_type }}
              </span>
            </div>
          </template>
        </draggable>
      </section>

    </div>
  </div>
</template>
