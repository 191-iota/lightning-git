<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { useRoute, useRouter, RouterLink } from "vue-router";
import draggable from "vuedraggable";
import { useAuthStore } from "@/stores/auth";
import { useProjectStore } from "@/stores/project";
import { useOrgStore } from "@/stores/org";
import { useActivityStore } from "@/stores/activity";
import { useToastStore } from "@/stores/toast";
import type { ActiveEdit, Task } from "@/types/api";
import NavBar from "@/components/NavBar.vue";
import Skeleton from "@/components/Skeleton.vue";
import TabStrip, { type Tab } from "@/components/TabStrip.vue";
import CloseIcon from "@/components/CloseIcon.vue";

const orgStore = useOrgStore();
const toast = useToastStore();
const loading = ref(true);

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
const router = useRouter();
const auth = useAuthStore();
const projectStore = useProjectStore();
const activityStore = useActivityStore();
const projectId = computed(() => route.params.id as string);

const showArchived = ref(false);

const editingName = ref(false);
const nameDraft = ref("");
const renaming = ref(false);
const nameInput = ref<HTMLInputElement | null>(null);

const canEditName = computed(() => {
  const role = projectStore.myRole(auth.user?.id);
  if (role === "admin") return true;
  const orgId = projectStore.current?.org_id;
  return !!orgId && orgStore.orgs.find((o) => o.id === orgId)?.role === "owner";
});

async function startEditName() {
  nameDraft.value = projectStore.current?.name ?? "";
  editingName.value = true;
  await nextTick();
  nameInput.value?.focus();
  nameInput.value?.select();
}

function cancelEditName() {
  editingName.value = false;
}

async function commitName() {
  if (!editingName.value) return;
  const next = nameDraft.value.trim();
  if (!next || next === projectStore.current?.name) {
    editingName.value = false;
    return;
  }
  renaming.value = true;
  try {
    await projectStore.rename(projectId.value, next);
    toast.success("Project renamed.");
    editingName.value = false;
  } catch {
    toast.error("Failed to rename project. You must be an admin.");
  } finally {
    renaming.value = false;
  }
}

async function archiveTask(taskId: string) {
  // optimistic: drop from columns immediately, then ask the backend to persist
  for (const col of COLUMN_IDS) {
    board.value[col] = board.value[col].filter((t) => t.id !== taskId);
  }
  try {
    await projectStore.setTaskArchived(taskId, true);
    toast.success("Task archived.");
  } catch {
    toast.error("Failed to archive task.");
    // refetch so state matches the server
    await projectStore.fetchTasks(projectId.value);
    reconcileBoard();
  }
}

async function restoreTask(taskId: string) {
  try {
    await projectStore.setTaskArchived(taskId, false);
    reconcileBoard();
    toast.success("Task restored.");
  } catch {
    toast.error("Failed to restore task.");
  }
}

// hide pre-existing main/master task rows that were created before the
// backend learned to skip integration branches. defense-in-depth.
const visibleTasks = computed(() =>
  projectStore.tasks.filter(
    (t) =>
      t.branch_name !== "main" &&
      t.branch_name !== "master" &&
      !t.archived,
  ),
);

const archivedTasks = computed(() =>
  projectStore.tasks.filter((t) => t.archived === true),
);

async function onLogout() {
  auth.logout();
  orgStore.clear();
  projectStore.clear();
  await router.push({ name: "login" });
}

function openOverlay(branch: string) {
  // prefer the current user's own active edit on that branch; fall back to any
  // teammate's edit. ?file= triggers OverlayView's existing auto-open watcher.
  const mine = activityStore.edits.find(
    (e) => e.branch === branch && e.user_id === auth.user?.id,
  );
  const target = mine ?? activityStore.edits.find((e) => e.branch === branch);
  router.push({
    name: "overlay",
    params: { id: projectId.value },
    query: target ? { branch, file: target.file } : { branch },
  });
}

// activity WS lives in the shared store, so it persists across the
// ProjectView <-> OverlayView navigation instead of tearing down on each.
const projectEdits = computed<ActiveEdit[]>(() => activityStore.edits);
const wsState = computed(() => activityStore.state);

// Board state. Local arrays so vuedraggable can mutate them on drag.
// Column assignments persist per (project, task) via localStorage , no backend
// Each task carries its column server-side. Drag updates the DB so every
// teammate sees the same board.
type ColumnId = "todo" | "in_progress" | "review" | "merged";
const COLUMN_IDS: ColumnId[] = ["todo", "in_progress", "review", "merged"];
const board = ref<Record<ColumnId, Task[]>>({
  todo: [],
  in_progress: [],
  review: [],
  merged: [],
});

function columnOf(t: Task): ColumnId {
  return (t.kanban_column as ColumnId | undefined) ?? "in_progress";
}

// rebuild the board from the canonical server task list. called on initial
// load and whenever the task list changes (refetch after archive/restore).
function reconcileBoard() {
  for (const col of COLUMN_IDS) {
    board.value[col] = [];
  }
  for (const t of visibleTasks.value) {
    board.value[columnOf(t)].push(t);
  }
}

watch(visibleTasks, reconcileBoard, { deep: true });

async function persistColumnChange(col: ColumnId, newList: Task[]) {
  const previous = board.value[col];
  const incoming = newList.filter(
    (t) => !previous.some((p) => p.id === t.id),
  );
  board.value[col] = newList;

  for (const t of incoming) {
    try {
      await projectStore.setTaskColumn(t.id, col);
    } catch (error) {
      const body = (error as { response?: { data?: unknown } })?.response?.data;
      const reason =
        typeof body === "string" && body.trim().length > 0
          ? body
          : "backend rejected the update";
      toast.error(`Failed to move task: ${reason}`);
      board.value[col] = previous;
      const oldCol = (t.kanban_column as ColumnId | undefined) ?? "in_progress";
      if (oldCol !== col && !board.value[oldCol].some((x) => x.id === t.id)) {
        board.value[oldCol] = [...board.value[oldCol], t];
      }
      return;
    }
  }
}

function editorsForBranch(branch: string): ActiveEdit[] {
  return projectEdits.value.filter((e) => e.branch === branch);
}

interface BoardColumn {
  id: ColumnId;
  label: string;
  dot: string;
  rule: string;
}
// only the active stage (In Progress) gets the violet accent; the others sit
// quietly in neutral so the eye finds the action without competing colors.
const boardColumns: BoardColumn[] = [
  { id: "todo", label: "To Do", dot: "bg-lg-text-muted", rule: "bg-lg-border" },
  { id: "in_progress", label: "In Progress", dot: "bg-lg-accent-bright", rule: "bg-lg-accent/60" },
  { id: "review", label: "Review", dot: "bg-lg-text-muted", rule: "bg-lg-border" },
  { id: "merged", label: "Merged", dot: "bg-lg-text-muted", rule: "bg-lg-border" },
];

// keep type indicators neutral, single accent reserved for interactive +
// active states. type info is carried by the word, not by hue.
function typeDotColor(_t: Task["task_type"]): string {
  return "bg-lg-text-muted/70";
}

onMounted(async () => {
  loading.value = true;
  try {
    await projectStore.get(projectId.value);
    await projectStore.fetchTasks(projectId.value);
    projectStore.fetchMembers(projectId.value).catch(() => undefined);
    reconcileBoard();
    activityStore.ensure(projectId.value);
  } catch {
    toast.error("Could not load this project.");
  } finally {
    loading.value = false;
  }
});
</script>

<template>
  <div class="min-h-screen bg-lg-bg text-lg-text">
    <NavBar>
      <span class="flex items-center gap-1.5 text-lg-text-muted">
        <span
          class="inline-block w-1.5 h-1.5 rounded-full"
          :class="{
            'bg-emerald-400': wsState === 'live',
            'bg-amber-400 animate-pulse': wsState === 'connecting',
            'bg-lg-rose': wsState === 'closed',
          }"
        ></span>
        {{ wsState }}
      </span>
      <RouterLink to="/orgs" class="lg-link">Switch org</RouterLink>
      <span class="text-lg-text-muted hidden sm:inline">{{ auth.user?.email }}</span>
      <button class="lg-link" @click="onLogout">Sign out</button>
    </NavBar>

    <main class="lg-container py-10">
      <RouterLink to="/dashboard" class="lg-breadcrumb mb-3">
        &larr; {{ parentOrg?.name ? `${parentOrg.name} / Projects` : "Projects" }}
      </RouterLink>

      <header class="mt-3 mb-6 min-h-[6rem] flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
        <div class="flex flex-col gap-2 items-start">
          <span class="lg-scope lg-scope-project">Project</span>
          <div v-if="editingName" class="flex items-center gap-2">
            <input
              ref="nameInput"
              v-model="nameDraft"
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
            <h1 class="text-3xl font-bold">
              {{ projectStore.current?.name || "Loading..." }}
            </h1>
            <button
              v-if="canEditName && projectStore.current"
              type="button"
              class="opacity-0 group-hover:opacity-100 text-[0.7rem] uppercase tracking-wider text-lg-text-muted hover:text-lg-accent-bright transition-opacity"
              @click="startEditName"
              title="Rename project"
            >rename</button>
          </div>
          <p class="text-[0.7rem] text-lg-text-muted/80 font-mono min-h-[1rem]">
            {{ projectStore.current?.repo_url || " " }}
          </p>
        </div>
        <RouterLink
          v-if="projectStore.members.length > 0"
          :to="{ name: 'project-members', params: { id: projectId } }"
          class="flex items-center gap-2 group"
          :title="projectStore.members.map((m) => m.display_name).join(', ') + ' (click to manage)'"
        >
          <div class="flex -space-x-2">
            <span
              v-for="m in projectStore.members.slice(0, 5)"
              :key="m.id"
              class="w-7 h-7 rounded-full border-2 border-lg-bg bg-lg-surface-2 flex items-center justify-center text-xs font-semibold uppercase"
              :class="
                m.role === 'admin'
                  ? 'text-lg-accent-bright'
                  : 'text-lg-text-sec'
              "
            >
              {{ m.display_name.slice(0, 1) }}
            </span>
            <span
              v-if="projectStore.members.length > 5"
              class="w-7 h-7 rounded-full border-2 border-lg-bg bg-lg-surface-2 flex items-center justify-center text-[0.65rem] font-semibold text-lg-text-sec"
            >
              +{{ projectStore.members.length - 5 }}
            </span>
          </div>
          <span class="text-sm text-lg-text-sec group-hover:text-lg-accent-bright transition-colors">
            {{ projectStore.members.length }}
            {{ projectStore.members.length === 1 ? "member" : "members" }}
          </span>
        </RouterLink>
      </header>

      <TabStrip :tabs="projectTabs" class="mb-6" />

      <div v-if="loading" class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-4">
        <section
          v-for="col in 4"
          :key="col"
          class="rounded-xl border border-lg-border bg-lg-surface p-4 min-h-[320px] space-y-3"
        >
          <Skeleton width="6rem" height="0.875rem" />
          <div
            v-for="i in 3"
            :key="i"
            class="rounded-lg border border-lg-border bg-lg-surface-2 p-3 space-y-2"
          >
            <Skeleton width="70%" height="0.875rem" />
            <Skeleton width="50%" height="0.625rem" />
            <Skeleton width="4rem" height="0.875rem" rounded="full" />
          </div>
        </section>
      </div>

      <div v-else class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-5">
        <section v-for="col in boardColumns" :key="col.id" class="flex flex-col">
          <div class="mb-3">
            <h2 class="flex items-center gap-2 pb-2">
              <span class="h-1.5 w-1.5 rounded-full" :class="col.dot"></span>
              <span class="text-sm text-lg-text font-medium">{{ col.label }}</span>
              <span class="text-xs text-lg-text-muted ml-0.5">{{ board[col.id].length }}</span>
            </h2>
            <div class="h-px w-full" :class="col.rule"></div>
          </div>
          <draggable
            :model-value="board[col.id]"
            @update:model-value="(list: Task[]) => persistColumnChange(col.id, list)"
            :group="{ name: 'kanban' }"
            item-key="id"
            class="space-y-2 flex-1 min-h-[220px]"
          >
            <template #item="{ element: task }">
              <div
                class="group relative rounded-md border bg-lg-surface-2 p-3.5 pr-7 cursor-pointer transition-colors hover:border-lg-accent/40"
                :class="[
                  col.id === 'merged' ? 'opacity-60' : '',
                  editorsForBranch(task.branch_name).length > 0
                    ? 'border-lg-accent-bright/60 border-l-2 border-l-lg-accent-bright'
                    : 'border-lg-border',
                ]"
                style="box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.03)"
                @click="openOverlay(task.branch_name)"
              >
                <button
                  type="button"
                  class="absolute top-2 right-2 opacity-0 group-hover:opacity-100 text-lg-text-muted hover:text-lg-rose transition-opacity p-1"
                  title="Archive"
                  @click.stop="archiveTask(task.id)"
                >
                  <CloseIcon :size="12" />
                </button>
                <p class="text-[0.95rem] font-semibold leading-snug flex items-center gap-2 truncate">
                  <span
                    v-if="editorsForBranch(task.branch_name).length > 0"
                    class="inline-block w-1.5 h-1.5 rounded-full bg-lg-accent-bright animate-pulse flex-shrink-0"
                    title="Live editing"
                  ></span>
                  <span class="truncate" :class="editorsForBranch(task.branch_name).length > 0 ? 'text-lg-accent-bright' : 'text-lg-text'">
                    {{ task.name || task.branch_name }}
                  </span>
                </p>
                <p class="text-[0.7rem] text-lg-text-muted/80 truncate font-mono mt-1">
                  {{ task.branch_name }}
                </p>
                <ul
                  v-if="editorsForBranch(task.branch_name).length > 0"
                  class="mt-2.5 space-y-0.5"
                >
                  <li
                    v-for="e in editorsForBranch(task.branch_name)"
                    :key="e.user_id + e.file"
                    class="text-[0.7rem] text-lg-accent-bright truncate"
                  >
                    {{ projectStore.memberName(e.user_id) }} · {{ e.file }}
                  </li>
                </ul>
                <span class="mt-3 flex items-center gap-1.5 text-[0.7rem] text-lg-text-muted/70">
                  <span class="h-1 w-1 rounded-full" :class="typeDotColor(task.task_type)"></span>
                  {{ task.task_type }}
                </span>
              </div>
            </template>
          </draggable>
        </section>
      </div>

      <section
        v-if="!loading && archivedTasks.length > 0"
        class="mt-8 rounded-xl border border-lg-border bg-lg-surface"
      >
        <button
          type="button"
          class="w-full flex items-center justify-between px-5 py-3 text-sm text-lg-text-sec hover:text-lg-text transition-colors"
          @click="showArchived = !showArchived"
        >
          <span class="flex items-center gap-2">
            <span class="text-xs uppercase tracking-wider font-semibold">Archived</span>
            <span class="text-lg-text-muted">({{ archivedTasks.length }})</span>
          </span>
          <span class="text-lg-text-muted">{{ showArchived ? "−" : "+" }}</span>
        </button>
        <div v-if="showArchived" class="border-t border-lg-border p-4 space-y-2">
          <div
            v-for="task in archivedTasks"
            :key="task.id"
            class="flex items-center gap-3 rounded-lg border border-lg-border bg-lg-surface-2 p-3"
          >
            <div class="flex-1 min-w-0">
              <p class="text-sm font-medium text-lg-text truncate">
                {{ task.name || task.branch_name }}
              </p>
              <p class="text-xs text-lg-text-muted truncate font-mono mt-0.5">
                {{ task.branch_name }}
              </p>
            </div>
            <span class="flex items-center gap-1.5 text-[0.7rem] text-lg-text-muted">
              <span class="h-1.5 w-1.5 rounded-full" :class="typeDotColor(task.task_type)"></span>
              {{ task.task_type }}
            </span>
            <button
              type="button"
              class="lg-btn-secondary text-xs px-3 py-1.5"
              @click="restoreTask(task.id)"
            >
              Restore
            </button>
          </div>
        </div>
      </section>
    </main>
  </div>
</template>
