<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { useRoute, RouterLink } from "vue-router";
import api from "@/services/api";
import { OverlayWebSocket, type OverlayUserView, type OverlayWsMsg } from "@/services/ws";
import { useAuthStore } from "@/stores/auth";
import { useProjectStore } from "@/stores/project";
import type { ActiveEdit } from "@/types/api";
import FileTreeNode, { type TreeNode } from "@/components/FileTreeNode.vue";

const route = useRoute();
const auth = useAuthStore();
const projectStore = useProjectStore();

const projectId = computed(() => route.params.id as string);
const branch = ref((route.query.branch as string) || "");
const branches = ref<string[]>([]);
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

// project-wide active edits across all branches, used to dot the file tree
const projectEdits = ref<ActiveEdit[]>([]);
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
let activityWs: WebSocket | null = null;

function connectActivity() {
  if (!auth.token) return;
  const base = import.meta.env.VITE_WS_URL || "ws://localhost:8080";
  const url = `${base}/api/projects/${projectId.value}/activity/ws?token=${encodeURIComponent(auth.token)}`;
  activityWs = new WebSocket(url);
  activityWs.onmessage = (event) => {
    try {
      projectEdits.value = JSON.parse(event.data) as ActiveEdit[];
    } catch {
      // ignore malformed
    }
  };
}

async function loadBranches() {
  try {
    const { data } = await api.get<string[]>(`/api/projects/${projectId.value}/branches`);
    branches.value = data;
    if (!branch.value && data.includes("main")) branch.value = "main";
  } catch {
    // dropdown stays empty; user can still type a branch
  }
}

onMounted(() => {
  projectStore.fetchMembers(projectId.value).catch(() => undefined);
  loadBranches();
  connectActivity();
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
  error.value = null;
  currentFile.value = path;
  try {
    const encodedFile = path.split("/").map(encodeURIComponent).join("/");
    // PUT seeds the overlay so the WS channel exists. We register as the
    // current user, but other clients filter us out of the active-editors
    // list because our content hasnt diverged from the base.
    await api.put(
      `/api/overlay/${projectId.value}/${auth.user.id}/${encodedFile}?branch=${encodeURIComponent(branch.value)}`,
    );
    const { data } = await api.get(
      `/api/overlay/${projectId.value}/${auth.user.id}/${encodedFile}`,
    );
    baseContent.value = data.original_content;
    // every divergent editor counts (including the current user editing in vscode)
    others.value = (data.all_user_contents as OverlayUserView[]).filter(
      (u) => u.content !== data.original_content,
    );

    ws?.dispose();
    ws = new OverlayWebSocket({
      projectId: projectId.value,
      userId: auth.user.id,
      fileName: path,
      token: auth.token,
    });
    ws.onMessage(handleMessage);
    ws.connect();
  } catch {
    error.value = "Failed to open file";
  }
}

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

// load tree when branch becomes valid; auto-open file once if linked from activity view
let autoOpened = false;
watch(
  branch,
  async () => {
    await loadTree();
    const queryFile = route.query.file as string | undefined;
    if (!autoOpened && queryFile && files.value.includes(queryFile)) {
      autoOpened = true;
      await openFile(queryFile);
    }
  },
  { immediate: true },
);

onUnmounted(() => {
  ws?.dispose();
  activityWs?.close();
  activityWs = null;
});
</script>

<template>
  <div class="min-h-screen bg-zinc-950 text-zinc-100 px-6 py-6">
    <header class="mb-4 flex items-center gap-4">
      <RouterLink :to="`/projects/${projectId}`" class="text-sm text-zinc-500 hover:text-zinc-300">
        &larr; Project
      </RouterLink>
      <h1 class="text-xl font-semibold">Live view</h1>
      <label class="ml-auto flex items-center gap-2">
        <span class="text-xs text-zinc-500">Branch</span>
        <select
          v-model="branch"
          class="bg-zinc-800 border border-zinc-700 rounded px-2 py-1 text-sm w-48 focus:outline-none focus:border-zinc-500"
        >
          <option v-if="branches.length === 0" :value="branch">{{ branch || "(none loaded)" }}</option>
          <option v-for="b in branches" :key="b" :value="b">{{ b }}</option>
        </select>
      </label>
    </header>

    <p v-if="error" class="text-sm text-red-400 mb-3">{{ error }}</p>

    <div class="grid grid-cols-12 gap-4">
      <aside class="col-span-3 border border-zinc-800 rounded p-3">
        <h2 class="text-sm font-medium text-zinc-300 mb-2">Files</h2>
        <p v-if="loadingTree" class="text-xs text-zinc-500">Loading...</p>
        <p v-else-if="!branch" class="text-xs text-zinc-500">Pick a branch first.</p>
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
          class="bg-zinc-900 border border-zinc-800 rounded p-3 text-sm font-mono whitespace-pre overflow-auto max-h-[80vh] min-h-[200px]"
        >
          <div
            v-for="(line, i) in projectedLines"
            :key="i"
            :class="lineClass(line)"
          >{{ line.text || " " }}</div>
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
