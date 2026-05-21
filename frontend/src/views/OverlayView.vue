<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { useRoute, RouterLink } from "vue-router";
import api from "@/services/api";
import { OverlayWebSocket, type OverlayUserView, type OverlayWsMsg } from "@/services/ws";
import { useAuthStore } from "@/stores/auth";
import { useProjectStore } from "@/stores/project";
import { useActivityStore } from "@/stores/activity";
import type { ActiveEdit } from "@/types/api";
import FileTreeNode, { type TreeNode } from "@/components/FileTreeNode.vue";

const route = useRoute();
const auth = useAuthStore();
const projectStore = useProjectStore();
const activityStore = useActivityStore();

const projectId = computed(() => route.params.id as string);
const branch = ref((route.query.branch as string) || "");
const files = ref<string[]>([]);
const currentFile = ref<string | null>(null);

// build a nested tree from the flat list of file paths
const tree = computed<TreeNode>(() => {
  const root: TreeNode = { name: "", fullPath: "", isFile: false, children: [] };
  for (const path of files.value) {
    const parts = path.split("/");
    let cur = root;
    for (let i = 0; i < parts.length; i++) {
      const segment = parts[i];
      const isLeaf = i === parts.length - 1;
      let child = cur.children.find((c) => c.name === segment && c.isFile === isLeaf);
      if (!child) {
        child = {
          name: segment,
          fullPath: isLeaf ? path : "",
          isFile: isLeaf,
          children: [],
        };
        cur.children.push(child);
      }
      cur = child;
    }
  }
  // folders before files at each level, alphabetical within
  const sortRec = (n: TreeNode) => {
    n.children.sort((a, b) => {
      if (a.isFile !== b.isFile) return a.isFile ? 1 : -1;
      return a.name.localeCompare(b.name);
    });
    n.children.forEach(sortRec);
  };
  sortRec(root);
  return root;
});

const baseContent = ref("");
const others = ref<OverlayUserView[]>([]);
const error = ref<string | null>(null);
const loadingTree = ref(false);

// project-wide active edits across all branches, used to dot the file tree.
// sourced from the shared activity store so the WS doesnt thrash on view changes.
const projectEdits = computed<ActiveEdit[]>(() => activityStore.edits);
const editedFileSet = computed(() => new Set(projectEdits.value.map((e) => e.file)));

// best-guess branch for a user editing the current file (from project activity feed)
function branchOf(userId: string): string {
  const e = projectEdits.value.find((x) => x.file === currentFile.value && x.user_id === userId);
  return e?.branch ?? "?";
}

// stable color per user_id so the same person is the same color in the main pane and the sidebar.
// keeping text/bg variants as literal arrays so tailwind picks them up at build time.
const textColors = ["text-amber-300", "text-emerald-300", "text-sky-300", "text-fuchsia-300", "text-orange-300"];
const bgColors = ["bg-amber-300", "bg-emerald-300", "bg-sky-300", "bg-fuchsia-300", "bg-orange-300"];
function colorIndex(userId: string): number {
  let h = 0;
  for (let i = 0; i < userId.length; i++) h = (h * 31 + userId.charCodeAt(i)) >>> 0;
  return h % textColors.length;
}
function colorFor(userId: string): string {
  return textColors[colorIndex(userId)];
}
function bgColorFor(userId: string): string {
  return bgColors[colorIndex(userId)];
}

// merge projection: line-by-line diff against base. only lines whose content
// actually differs from base get colored. edited_sections from VSCode covers
// the whole file so we cant trust it for the visual diff.
interface ProjLine {
  text: string;
  userId?: string;
  conflict?: boolean;
}
const projectedLines = computed<ProjLine[]>(() => {
  const baseLines = (baseContent.value || "").split("\n");
  const out: ProjLine[] = baseLines.map((t) => ({ text: t }));
  for (const u of others.value) {
    const overlay = u.content.split("\n");
    while (out.length < overlay.length) out.push({ text: "" });
    for (let i = 0; i < overlay.length; i++) {
      const baseLine = baseLines[i] ?? "";
      const overlayLine = overlay[i];
      if (baseLine === overlayLine) continue;
      if (out[i].userId && out[i].userId !== u.user_id) {
        out[i] = { text: overlayLine, conflict: true };
      } else {
        out[i] = { text: overlayLine, userId: u.user_id };
      }
    }
  }
  return out;
});

function lineClass(line: ProjLine): string {
  if (line.conflict) return "bg-red-900/40 text-red-200";
  if (line.userId) return colorFor(line.userId);
  return "text-zinc-300";
}

let ws: OverlayWebSocket | null = null;

onMounted(() => {
  projectStore.fetchMembers(projectId.value).catch(() => undefined);
  activityStore.ensure(projectId.value);
});

async function loadTree() {
  if (!branch.value) return;
  error.value = null;
  loadingTree.value = true;
  try {
    const { data } = await api.get<string[]>(
      `/api/projects/${projectId.value}/tree?branch=${encodeURIComponent(branch.value)}`,
    );
    files.value = data;
  } catch {
    error.value = "Failed to load file tree";
    files.value = [];
  } finally {
    loadingTree.value = false;
  }
}

async function openFile(path: string) {
  if (!auth.user || !auth.token) return;
  // clicking the same file again should be a no-op — tearing down a healthy WS
  // and re-fetching the file just to land in the same state caused flicker and
  // a spurious "reconnect" on a connection that was already live.
  if (currentFile.value === path) return;
  error.value = null;
  currentFile.value = path;
  others.value = [];
  ws?.dispose();
  ws = null;

  // view mode: just fetch the current branch content. no overlay involvement.
  try {
    const { data } = await api.get<string>(
      `/api/projects/${projectId.value}/file?branch=${encodeURIComponent(branch.value)}&path=${encodeURIComponent(path)}`,
      { responseType: "text", transformResponse: [(v) => v] },
    );
    baseContent.value = data;
  } catch {
    error.value = "Failed to read file";
    baseContent.value = "";
  }

  // live mode is opened lazily by the activity-driven watcher below, only when
  // someone is actually editing this file. otherwise no per-file ws connects.
}

async function connectFileLive(path: string) {
  if (!auth.user || !auth.token || ws) return;
  // assign `ws` synchronously so a re-firing watcher cant race past the
  // `!ws` guard during the await below and spawn a second socket.
  ws = new OverlayWebSocket({
    projectId: projectId.value,
    userId: auth.user.id,
    fileName: path,
    token: auth.token,
  });
  ws.onMessage(handleMessage);
  ws.connect();

  // seed `others` with the current overlay state
  try {
    const encoded = path.split("/").map(encodeURIComponent).join("/");
    const { data } = await api.get(
      `/api/overlay/${projectId.value}/${auth.user.id}/${encoded}`,
    );
    // include every active user — projectedLines does the per-line diff
    // against baseContent and skips lines that already match, so this wont
    // create spurious colour but DOES surface the main-vs-branch distinction
    // before the user has typed anything.
    others.value = data.all_user_contents as OverlayUserView[];
  } catch {
    // overlay may exist as a broadcast channel without an HTTP-readable
    // snapshot if no one has PUT yet; the WS stream will fill `others` in.
  }
}

// connect the per-file WS only when activity feed reports this file has editors,
// and tear it down when activity stops. avoids noisy 404s on idle files.
watch(
  () => ({ file: currentFile.value, edits: editedFileSet.value }),
  ({ file, edits }) => {
    if (!file) return;
    if (edits.has(file)) {
      if (!ws) void connectFileLive(file);
    } else if (ws) {
      ws.dispose();
      ws = null;
      others.value = [];
    }
  },
  { deep: true },
);

function handleMessage(msg: OverlayWsMsg) {
  if (msg.kind !== "change") return;
  const change = msg.payload;
  // a Change broadcast means the sender is editing, not just viewing
  const idx = others.value.findIndex((u) => u.user_id === change.user_id);
  const entry: OverlayUserView = {
    user_id: change.user_id,
    content: change.content,
    edited_sections: change.line_section,
    updated_at_secs: 0,
    updated_at_nanos: 0,
  };
  if (idx >= 0) others.value[idx] = entry;
  else others.value.push(entry);
}

// load tree on mount; auto-open file once if linked from activity view.
let autoOpened = false;
watch(
  branch,
  async () => {
    await loadTree();
    const queryFile = route.query.file as string | undefined;
    if (!autoOpened && queryFile && files.value.includes(queryFile)) {
      autoOpened = true;
      await openFile(queryFile);
      return;
    }
    if (currentFile.value) {
      if (files.value.includes(currentFile.value)) {
        await openFile(currentFile.value);
      } else {
        currentFile.value = null;
        baseContent.value = "";
        others.value = [];
        ws?.dispose();
        ws = null;
      }
    }
  },
  { immediate: true },
);

onUnmounted(() => {
  ws?.dispose();
  // intentionally NOT disposing activityStore — its WS is shared and persists
  // across the ProjectView <-> OverlayView navigation.
});
</script>

<template>
  <div class="min-h-screen bg-zinc-950 text-zinc-100 px-6 py-6">
    <header class="mb-4 flex items-center gap-4">
      <RouterLink :to="`/projects/${projectId}`" class="text-sm text-zinc-500 hover:text-zinc-300">
        &larr; Project
      </RouterLink>
      <h1 class="text-xl font-semibold">Live view</h1>
    </header>

    <p v-if="error" class="text-sm text-red-400 mb-3">{{ error }}</p>

    <div class="grid grid-cols-12 gap-4">
      <aside class="col-span-3 border border-zinc-800 rounded p-3">
        <h2 class="text-sm font-medium text-zinc-300 mb-2">Files</h2>
        <p v-if="loadingTree" class="text-xs text-zinc-500">Loading...</p>
        <p v-else-if="!branch" class="text-xs text-zinc-500">No branch in URL.</p>
        <p v-else-if="files.length === 0" class="text-xs text-zinc-500">No files.</p>
        <ul class="space-y-0.5 text-sm">
          <FileTreeNode
            v-for="child in tree.children"
            :key="child.name + child.fullPath"
            :node="child"
            :selected="currentFile"
            :edited-files="editedFileSet"
            @open="openFile"
          />
        </ul>
      </aside>

      <main class="col-span-6">
        <p class="text-xs text-zinc-500 mb-2">
          {{ currentFile ? `${branch}:${currentFile}` : "Select a file to view the overlay" }}
          <span v-if="others.length > 0" class="text-zinc-600">— merge projection</span>
        </p>
        <div
          v-if="currentFile"
          class="bg-zinc-900 border border-zinc-800 rounded p-3 text-sm font-mono overflow-auto max-h-[80vh] min-h-[200px]"
        >
          <div
            v-for="(line, i) in projectedLines"
            :key="i"
            :class="lineClass(line)"
            class="whitespace-pre leading-5"
          >{{ line.text || " " }}</div>
        </div>
      </main>

      <aside class="col-span-3">
        <h2 class="text-sm font-medium text-zinc-300 mb-2">
          Active editors
          <span class="text-zinc-500">({{ others.length }})</span>
        </h2>
        <p v-if="others.length === 0" class="text-xs text-zinc-500">
          No one editing this file right now.
        </p>
        <ul class="space-y-2">
          <li
            v-for="u in others"
            :key="u.user_id"
            class="border border-zinc-800 rounded p-2 text-xs"
          >
            <p class="font-medium flex items-center gap-2">
              <span class="inline-block w-2 h-2 rounded-full" :class="bgColorFor(u.user_id)"></span>
              <span :class="colorFor(u.user_id)">
                {{ projectStore.memberName(u.user_id) }}{{ u.user_id === auth.user?.id ? " (you)" : "" }}
              </span>
            </p>
            <p class="text-zinc-500">
              {{ branchOf(u.user_id) }} · lines {{ u.edited_sections[0] }}-{{ u.edited_sections[1] }}
            </p>
          </li>
        </ul>
      </aside>
    </div>
  </div>
</template>
