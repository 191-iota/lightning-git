<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, shallowRef, watch } from "vue";
import { useRoute, RouterLink } from "vue-router";
import api from "@/services/api";
import { OverlayWebSocket, type OverlayUserView, type WsMessage } from "@/services/ws";
import { useAuthStore } from "@/stores/auth";
import { useProjectStore } from "@/stores/project";
import { useActivityStore } from "@/stores/activity";
import type { ActiveEdit, Comment, MergeConflict, ProjectTree } from "@/types/api";
import FileTreeNode from "@/components/FileTreeNode.vue";
import { buildTree, computeProjectedLines, type ProjLine } from "@/utils/overlay";
import { computeCombinedDiff, computeConflicts, flattenConflicts } from "@/utils/merge";
import NavBar from "@/components/NavBar.vue";
import Skeleton from "@/components/Skeleton.vue";
import TabStrip, { type Tab } from "@/components/TabStrip.vue";
import { useToastStore } from "@/stores/toast";
import { useOrgStore } from "@/stores/org";
import { useRouter } from "vue-router";

const orgStore = useOrgStore();
const toast = useToastStore();
const fileLoading = ref(false);
const router = useRouter();

async function onLogout() {
  auth.logout();
  orgStore.clear();
  projectStore.clear();
  await router.push({ name: "login" });
}

const projectTabs = computed<Tab[]>(() => [
  { label: "Board", to: { name: "project", params: { id: projectId.value } } },
  { label: "Live view", to: { name: "overlay", params: { id: projectId.value }, query: { branch: branch.value || "main" } } },
  { label: "Members", to: { name: "project-members", params: { id: projectId.value } } },
]);

const parentOrg = computed(() => {
  const orgId = projectStore.current?.org_id;
  if (!orgId) return null;
  const o = orgStore.orgs.find((x) => x.id === orgId);
  return o ? { id: o.id, name: o.name } : null;
});

const projectName = computed(() => projectStore.current?.name ?? "Project");

const route = useRoute();
const auth = useAuthStore();
const projectStore = useProjectStore();
const activityStore = useActivityStore();

const projectId = computed(() => route.params.id as string);
const branch = ref((route.query.branch as string) || "");
const files = ref<string[]>([]);
const drafts = ref<string[]>([]);
const currentFile = ref<string | null>(null);

// build a nested tree from the flat list of file paths
const tree = computed(() => buildTree(files.value));

const baseContent = ref("");
const others = ref<OverlayUserView[]>([]);
const loadingTree = ref(false);
const comments = ref<Comment[]>([]);
const draftLine = ref<number | null>(null);
const draftText = ref("");
let conflictsTimer: ReturnType<typeof setInterval> | null = null;

// backend-computed merge conflicts for the current file. polled, not pushed.
// the backend already runs the hunk overlap algorithm across every branch's
// content (committed and live overlays) against origin/main, so we just
// surface its result instead of recomputing locally.
// shallowRef because the response is a freshly-decoded JSON array we always
// replace wholesale, and deep-proxying every hunk + content array on each
// 60s tick was making the page hitch when many branches diverged.
const mergeConflicts = shallowRef<MergeConflict[]>([]);

// Live conflicts run the SAME algorithm as the backend's merge_service
// (computeCombinedDiff + computeConflicts mirror compute_combined_diff +
// compute_conflicts). The live pass only sees live overlay sources; the
// backend's 60s poll sees those PLUS committed branches and writes its
// authoritative result into mergeConflicts. Because both use the same
// algorithm, the two results align at overlapping ranges and the backend
// just adds any extra conflicts it found on committed branches.
const liveConflicts = computed<MergeConflict[]>(() => {
  const file = currentFile.value;
  const branchLookup = new Map<string, string>();
  for (const e of projectEdits.value) {
    if (e.file === file) branchLookup.set(e.user_id, e.branch);
  }
  const sources = others.value.map((u) => ({
    branch: branchLookup.get(u.user_id) ?? "?",
    userId: u.user_id,
    content: u.content,
  }));
  const hunks = computeCombinedDiff(baseContent.value, sources);
  return computeConflicts(hunks);
});

const effectiveConflicts = computed<MergeConflict[]>(() =>
  flattenConflicts(liveConflicts.value, mergeConflicts.value),
);

const conflictByLine = computed(() => {
  const map = new Map<number, MergeConflict>();
  for (const c of effectiveConflicts.value) {
    for (let line = c.base_start + 1; line <= c.base_end; line++) {
      map.set(line, c);
    }
    if (c.base_start === c.base_end) {
      map.set(c.base_start + 1, c);
    }
  }
  return map;
});

interface GroupedVersion {
  contributors: { user_id?: string | null }[];
  content: string[];
}
interface GroupedBranch {
  branch: string;
  versions: GroupedVersion[];
}

// group a conflict's hunks by branch, then by content within the branch.
// two users on the same branch with identical content collapse into one
// version with both credited as contributors, so the panel no longer reads
// "2 versions" when there's actually one shared version.
function groupedHunks(conflict: MergeConflict): GroupedBranch[] {
  const byBranch = new Map<string, Map<string, GroupedVersion>>();
  for (const h of conflict.hunks) {
    // key by normalized content so trailing-empty differences dont split
    // identical-looking versions into separate cards.
    const key = normalizeContent(h.content);
    let versions = byBranch.get(h.branch);
    if (!versions) {
      versions = new Map();
      byBranch.set(h.branch, versions);
    }
    const existing = versions.get(key);
    if (existing) {
      existing.contributors.push({ user_id: h.user_id });
    } else {
      versions.set(key, { contributors: [{ user_id: h.user_id }], content: h.content });
    }
  }
  return Array.from(byBranch.entries()).map(([branch, versions]) => ({
    branch,
    versions: Array.from(versions.values()),
  }));
}

function contributorLabel(c: { user_id?: string | null }): string {
  if (!c.user_id) return "committed";
  const me = c.user_id === auth.user?.id;
  return projectStore.memberName(c.user_id) + (me ? " (you, live)" : " (live)");
}

// conflicts default to expanded; the user clicks "hide" to collapse a card
// they've already inspected. tracking the inverse set means the default
// matches the user's "live visible" expectation without seeding state every
// time a new conflict appears.
const collapsedConflicts = ref<Set<number>>(new Set());
function toggleConflictCollapse(line: number) {
  const s = new Set(collapsedConflicts.value);
  if (s.has(line)) s.delete(line);
  else s.add(line);
  collapsedConflicts.value = s;
}

// give empty hunk content a label that says what it means: an empty content
// vec is a deletion of the base range, not "the branch has nothing".
function describeContent(content: string[]): string {
  if (content.length === 0) return "(removed)";
  const joined = content.join("\n");
  if (joined.trim().length === 0) return "(empty lines)";
  return joined;
}

// collapse whitespace differences for the dedupe signature. two versions
// that render visually identical (e.g. both as just "huh") sometimes carry
// different array shapes — one ["huh"], another ["", "huh"] or
// ["huh", "", ""] from split-on-newline quirks — and would otherwise be
// emitted as a fake "2 versions, both huh" conflict. trim() of the
// joined string normalizes leading/trailing whitespace including newlines.
function normalizeContent(content: string[]): string {
  return content.join("\n").trim();
}

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
// picked for visual distinctness rather than aesthetic harmony: amber + orange
// + emerald all looked too similar at small sizes, so the palette spreads
// across yellow / green / cyan / magenta / violet hues that are easy to tell
// apart even on a single line.
const textColors = ["text-amber-700", "text-emerald-700", "text-sky-700", "text-rose-700", "text-violet-700"];
const bgColors = ["bg-amber-500", "bg-emerald-500", "bg-sky-500", "bg-rose-500", "bg-violet-500"];
// full badge class strings (bg + text + border) per color slot. tailwind has
// to see the literal classes to include them in the build, so they live
// as static array entries rather than being templated together at runtime.
const badgeClasses = [
  "bg-amber-50 text-amber-800 border border-amber-200",
  "bg-emerald-50 text-emerald-800 border border-emerald-200",
  "bg-sky-50 text-sky-800 border border-sky-200",
  "bg-rose-50 text-rose-800 border border-rose-200",
  "bg-violet-50 text-violet-800 border border-violet-200",
];
// position-based color assignment over a stable sort of every known user id
// in this session. five users always get five distinct colors; the hash-mod
// approach this replaced could (and did) collide on small populations.
// sort is over uuid strings so it's deterministic between reloads.
const knownUserIds = computed<string[]>(() => {
  const ids = new Set<string>();
  for (const u of others.value) ids.add(u.user_id);
  for (const c of comments.value) ids.add(c.user_id);
  if (auth.user?.id) ids.add(auth.user.id);
  return Array.from(ids).sort();
});
function colorIndex(userId: string): number {
  const idx = knownUserIds.value.indexOf(userId);
  if (idx >= 0) return idx % textColors.length;
  // fallback for users not yet seen in others / comments (e.g. members
  // panel before their first edit lands).
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
function badgeClassFor(userId: string | null | undefined): string {
  if (!userId) return "bg-lg-surface-2 text-lg-text-muted border border-lg-border";
  return badgeClasses[colorIndex(userId)];
}

// merge projection: line-by-line diff against base. only lines whose content
// actually differs from base get colored. edited_sections from VSCode covers
// the whole file so we cant trust it for the visual diff.
const projectedLines = computed<ProjLine[]>(() =>
  computeProjectedLines(baseContent.value, others.value),
);

// rendering thousands of reactive line divs (each with a hover button +
// nested v-fors for comments and drafts) freezes the page. cap the rendered
// slice and show a notice when truncated; live edits still come through the
// WS and conflict detection still runs over the full content.
const MAX_RENDERED_LINES = 1500;
const renderableLines = computed<ProjLine[]>(() =>
  projectedLines.value.slice(0, MAX_RENDERED_LINES),
);
const isTruncated = computed(() => projectedLines.value.length > MAX_RENDERED_LINES);

function lineClass(line: ProjLine): string {
  if (line.userId) return colorFor(line.userId);
  return "text-lg-text-sec";
}

let ws: OverlayWebSocket | null = null;

onMounted(() => {
  projectStore.fetchMembers(projectId.value).catch(() => undefined);
  activityStore.ensure(projectId.value);
  void initBranchView();
  // backend is the slow source of truth (commits, cross-branch). live
  // overlap synthesis on the client bridges the gap so we dont need a fast
  // poll. the in-flight guard in refreshConflicts keeps slow ticks from
  // stacking.
  conflictsTimer = setInterval(() => void refreshConflicts(), 60000);
});

async function loadTree() {
  if (!branch.value) return;
  loadingTree.value = true;
  try {
    const { data } = await api.get<ProjectTree>(
      `/api/projects/${projectId.value}/tree?branch=${encodeURIComponent(branch.value)}`,
    );
    files.value = data.committed;
    drafts.value = data.drafts;
  } catch {
    toast.error("Failed to load file tree.");
    files.value = [];
    drafts.value = [];
  } finally {
    loadingTree.value = false;
  }
}

// activity feed is a hint, not a source of truth: when it mentions a file on
// the current branch that isnt in our committed list or drafts list yet,
// refetch the tree once. avoids polling and avoids client-side unioning.
watch(
  () => activityStore.edits.map((e) => `${e.branch}:${e.file}`).sort().join("|"),
  () => {
    if (!branch.value) return;
    const known = new Set<string>([...files.value, ...drafts.value]);
    const missing = activityStore.edits.some(
      (e) => e.branch === branch.value && !known.has(e.file),
    );
    if (missing) void loadTree();
  },
);

async function openFile(path: string) {
  if (!auth.user || !auth.token) return;
  if (currentFile.value === path) return;

  // tear everything down BEFORE opening the new WS so the new snapshot lands
  // on clean state. a watcher trying to do the same thing was racing with
  // this function and flickering.
  ws?.dispose();
  ws = null;
  others.value = [];
  comments.value = [];
  draftLine.value = null;
  draftText.value = "";
  baseContent.value = "";
  mergeConflicts.value = [];
  fileLoading.value = true;
  currentFile.value = path;

  // fetch base FIRST so projectedLines computes against real content as
  // soon as the WS snapshot lands. otherwise the LCS diff briefly treats
  // every overlay line as an "addition" and flashes the wrong colors.
  try {
    const { data } = await api.get<string>(
      // base content for merge analysis is origin/main (the merge target),
      // NOT the user's feature branch. otherwise commits already on the
      // feature branch look like "matching base" client-side while the
      // backend (which always diffs vs main) sees them as divergent — and
      // the live conflict ends up smaller than the 60s-poll conflict.
      `/api/projects/${projectId.value}/file?branch=main&path=${encodeURIComponent(path)}`,
      { responseType: "text", transformResponse: [(v) => v] },
    );
    baseContent.value = data;
  } catch {
    toast.error("Failed to read file.");
    baseContent.value = "";
  } finally {
    fileLoading.value = false;
  }

  // open the per-file WS. the backend pushes a Snapshot immediately so
  // others + comments populate without a separate fetch.
  connectFileLive(path);
  void refreshConflicts();
}

function encodeFilePath(path: string): string {
  return path.split("/").map(encodeURIComponent).join("/");
}

// in-flight guard. without this, when the merge endpoint takes >60s (many
// branches serializing through the git fetch_lock on the backend), the
// setInterval keeps firing new requests on top of pending ones; when they
// all eventually return their responses land in a burst, and each wholesale
// replace of mergeConflicts churns the render. one at a time is enough.
let conflictsInFlight = false;
async function refreshConflicts() {
  const file = currentFile.value;
  if (!file) {
    mergeConflicts.value = [];
    return;
  }
  if (conflictsInFlight) return;
  conflictsInFlight = true;
  try {
    const { data } = await api.get<MergeConflict[]>(
      `/api/merge/${projectId.value}/${encodeFilePath(file)}`,
    );
    // file may have changed while we were awaiting; only commit if its still
    // the one the user is looking at.
    if (currentFile.value === file) mergeConflicts.value = data;
  } catch {
    if (currentFile.value === file) mergeConflicts.value = [];
  } finally {
    conflictsInFlight = false;
  }
}

function commentsForLine(line: number): Comment[] {
  return comments.value.filter((c) => c.line === line);
}

function startDraft(line: number) {
  draftLine.value = line;
  draftText.value = "";
}

// open the comment composer pre-filled with an "@username " mention so the
// recipient can scan a conflict panel and immediately reply to a specific
// contributor's version. comments stay line-keyed at the conflict's start
// line on the backend; the mention is the addressing convention.
function startReplyToUser(line: number, userId: string | null | undefined) {
  draftLine.value = line;
  const name = userId ? projectStore.memberName(userId) : "committed";
  draftText.value = `@${name} `;
}

// @mention autocomplete state. when the cursor sits right after an "@"
// followed by non-space chars, we show a dropdown of project members
// filtered by that prefix. clicking an item replaces the partial mention
// with "@FullName ".
const mentionOpen = ref(false);
const mentionQuery = ref("");
const mentionActiveIndex = ref(0);

const projectMembers = computed(() => projectStore.members ?? []);

const mentionCandidates = computed(() => {
  const q = mentionQuery.value.toLowerCase();
  return projectMembers.value
    .filter((m) => m.display_name.toLowerCase().includes(q))
    .slice(0, 6);
});

function onDraftInput(event: Event) {
  const ta = event.target as HTMLTextAreaElement;
  draftText.value = ta.value;
  const caret = ta.selectionStart ?? ta.value.length;
  // walk back from the caret to find the last "@" without an intervening
  // whitespace or newline. if found, open the menu with the chars after it
  // as the filter query.
  const upto = ta.value.slice(0, caret);
  const atIdx = upto.lastIndexOf("@");
  if (atIdx < 0) {
    mentionOpen.value = false;
    return;
  }
  const tail = upto.slice(atIdx + 1);
  if (/[\s\n]/.test(tail)) {
    mentionOpen.value = false;
    return;
  }
  mentionQuery.value = tail;
  mentionActiveIndex.value = 0;
  mentionOpen.value = true;
}

function pickMention(name: string, target: HTMLTextAreaElement | null) {
  if (!target) {
    mentionOpen.value = false;
    return;
  }
  const caret = target.selectionStart ?? target.value.length;
  const upto = target.value.slice(0, caret);
  const atIdx = upto.lastIndexOf("@");
  if (atIdx < 0) {
    mentionOpen.value = false;
    return;
  }
  const before = target.value.slice(0, atIdx);
  const after = target.value.slice(caret);
  const inserted = `@${name} `;
  const next = before + inserted + after;
  draftText.value = next;
  mentionOpen.value = false;
  // restore caret right after the inserted mention on next tick
  void Promise.resolve().then(() => {
    target.focus();
    const pos = before.length + inserted.length;
    target.setSelectionRange(pos, pos);
  });
}

function onDraftKeydown(event: KeyboardEvent) {
  if (!mentionOpen.value || mentionCandidates.value.length === 0) return;
  if (event.key === "ArrowDown") {
    event.preventDefault();
    mentionActiveIndex.value =
      (mentionActiveIndex.value + 1) % mentionCandidates.value.length;
  } else if (event.key === "ArrowUp") {
    event.preventDefault();
    mentionActiveIndex.value =
      (mentionActiveIndex.value - 1 + mentionCandidates.value.length)
      % mentionCandidates.value.length;
  } else if (event.key === "Enter" || event.key === "Tab") {
    event.preventDefault();
    const pick = mentionCandidates.value[mentionActiveIndex.value];
    pickMention(pick.display_name, event.target as HTMLTextAreaElement);
  } else if (event.key === "Escape") {
    mentionOpen.value = false;
  }
}

function cancelDraft() {
  draftLine.value = null;
  draftText.value = "";
}

function submitDraft() {
  const file = currentFile.value;
  const line = draftLine.value;
  const text = draftText.value.trim();
  if (!file || line === null || !text || !auth.user) return;
  if (!ws) {
    toast.error("Not connected to live channel; cannot post comment.");
    return;
  }
  // server fills id + created_at when it broadcasts back. our local list
  // updates via the comment_created event in handleMessage.
  ws.send({
    kind: "comment_created",
    id: "00000000-0000-0000-0000-000000000000",
    user_id: auth.user.id,
    line,
    text,
    created_at: 0,
  });
  draftLine.value = null;
  draftText.value = "";
}

function deleteOwnComment(c: Comment) {
  if (!ws) {
    toast.error("Not connected to live channel; cannot delete comment.");
    return;
  }
  ws.send({ kind: "comment_deleted", id: c.id });
}

function fmtCommentTime(unixSecs: number): string {
  const ageSec = Math.max(0, Math.floor(Date.now() / 1000 - unixSecs));
  if (ageSec < 60) return `${ageSec}s ago`;
  if (ageSec < 3600) return `${Math.floor(ageSec / 60)}m ago`;
  if (ageSec < 86400) return `${Math.floor(ageSec / 3600)}h ago`;
  return `${Math.floor(ageSec / 86400)}d ago`;
}

function connectFileLive(path: string) {
  if (!auth.user || !auth.token || ws) return;
  // assign ws synchronously so a re-firing watcher cant race past the
  // !ws guard and spawn a second socket. backend pushes a Snapshot message
  // on subscribe, so no HTTP follow-up is needed to seed others / comments.
  ws = new OverlayWebSocket({
    projectId: projectId.value,
    userId: auth.user.id,
    fileName: path,
    token: auth.token,
  });
  ws.onMessage(handleMessage);
  ws.connect();
}

function handleMessage(msg: WsMessage) {
  switch (msg.kind) {
    case "snapshot":
      // initial state pushed by the backend right after subscribe; populate
      // both comments and active editors so no HTTP follow-up is needed.
      // immutable replace so Vue picks up the change reliably.
      comments.value = [...msg.comments];
      others.value = [...msg.all_user_contents];
      return;
    case "overlay": {
      const entry: OverlayUserView = {
        user_id: msg.user_id,
        content: msg.content,
        edited_sections: msg.line_section,
        updated_at_secs: 0,
        updated_at_nanos: 0,
      };
      // immutable update: filter the previous entry for this user_id out
      // and append the new one. avoids in-place .push / index-assignment
      // that didn't reliably trigger downstream computeds in some cases.
      others.value = [
        ...others.value.filter((u) => u.user_id !== msg.user_id),
        entry,
      ];
      return;
    }
    case "comment_created":
      comments.value = [
        ...comments.value.filter((c) => c.id !== msg.id),
        {
          id: msg.id,
          user_id: msg.user_id,
          line: msg.line,
          text: msg.text,
          created_at: msg.created_at,
        },
      ];
      return;
    case "comment_deleted":
      comments.value = comments.value.filter((c) => c.id !== msg.id);
      return;
  }
}

async function initBranchView() {
  await loadTree();
  const queryFile = route.query.file as string | undefined;
  if (queryFile && files.value.includes(queryFile)) {
    await openFile(queryFile);
  }
}

onUnmounted(() => {
  ws?.dispose();
  if (conflictsTimer) clearInterval(conflictsTimer);
  // intentionally NOT disposing activityStore, its WS is shared and persists
  // across the ProjectView <-> OverlayView navigation.
});
</script>

<template>
  <div class="min-h-screen bg-lg-bg text-lg-text">
    <NavBar :wide="true">
      <RouterLink to="/orgs" class="lg-link">Switch org</RouterLink>
      <span class="text-lg-text-muted hidden sm:inline">{{ auth.user?.email }}</span>
      <button class="lg-link" @click="onLogout">Sign out</button>
    </NavBar>

    <main class="lg-container-wide py-10">
      <RouterLink to="/dashboard" class="lg-breadcrumb mb-3">
        &larr; {{ parentOrg?.name ? `${parentOrg.name} / Projects` : "Projects" }}
      </RouterLink>

      <header class="mt-3 mb-6 min-h-[6rem] flex items-center">
        <div class="flex flex-col gap-2 items-start">
          <span class="lg-scope lg-scope-project">Project</span>
          <h1 class="text-3xl font-bold">{{ projectName }}</h1>
          <p class="text-[0.7rem] text-lg-text-muted/80 font-mono min-h-[1rem]">
            {{ branch ? `${branch}${currentFile ? `:${currentFile}` : ""}` : " " }}
          </p>
        </div>
      </header>

      <TabStrip :tabs="projectTabs" class="mb-6" />
      <div class="grid grid-cols-12 gap-4">
        <aside class="col-span-2 lg-card p-4 space-y-5 h-[70vh] overflow-auto lg-scroll-dark">
          <section>
            <h2 class="text-xs text-lg-text-sec font-medium mb-3">Files</h2>

            <div v-if="loadingTree" class="space-y-2">
              <Skeleton v-for="i in 8" :key="i" :width="i % 2 ? '85%' : '65%'" height="0.625rem" />
            </div>
            <p v-else-if="!branch" class="text-xs text-lg-text-muted">No branch in URL.</p>
            <p v-else-if="files.length === 0" class="text-xs text-lg-text-muted">No files.</p>
            <ul v-else class="space-y-0.5 text-sm">
              <FileTreeNode
                v-for="child in tree.children"
                :key="child.name + child.fullPath"
                :node="child"
                :selected="currentFile"
                :edited-files="editedFileSet"
                @open="openFile"
              />
            </ul>
          </section>

          <section v-if="drafts.length > 0">
            <h2 class="text-xs text-lg-text-sec font-medium mb-3">
              Live drafts
            </h2>
            <ul class="space-y-0.5 text-sm">
              <li
                v-for="path in drafts"
                :key="path"
                class="cursor-pointer italic truncate transition-colors px-1 py-0.5 rounded"
                :class="
                  path === currentFile
                    ? 'text-lg-accent-bright font-medium'
                    : 'text-lg-accent-bright/70 hover:text-lg-accent-bright'
                "
                @click="openFile(path)"
              >{{ path }}</li>
            </ul>
          </section>
        </aside>

        <section class="col-span-8">
          <div
            v-if="!currentFile"
            class="rounded-xl border border-lg-border bg-lg-surface h-[70vh] flex items-center justify-center text-sm text-lg-text-muted"
          >
            Select a file to view the overlay
          </div>
          <div
            v-else-if="currentFile && fileLoading"
            class="rounded-xl border border-lg-border bg-lg-surface p-4 space-y-2 min-h-[200px]"
          >
            <Skeleton
              v-for="i in 14"
              :key="i"
              :width="`${30 + ((i * 13) % 60)}%`"
              height="0.625rem"
            />
          </div>
          <div
            v-else-if="currentFile"
            class="rounded-xl border border-lg-border bg-lg-surface text-sm font-mono overflow-auto h-[70vh] lg-scroll-dark"
          >
            <div
              v-if="isTruncated"
              class="sticky top-0 z-10 bg-lg-surface-2 border-b border-lg-border px-3 py-1.5 text-xs text-lg-text-muted"
            >
              File too long to render fully. Showing first {{ MAX_RENDERED_LINES }} of {{ projectedLines.length }} lines.
            </div>
            <template v-for="(line, i) in renderableLines" :key="i">
              <div
                class="group flex items-start gap-2 pr-3 leading-5"
                :class="
                  conflictByLine.get(i + 1)
                    ? 'bg-lg-rose/10 hover:bg-lg-rose/15'
                    : 'hover:bg-lg-surface-2'
                "
              >
                <span class="select-none pl-2 w-10 text-right text-lg-text-muted text-xs pt-0.5">{{ i + 1 }}</span>
                <!-- the gutter dot was visually noisy; the per-user color on
                     the line text itself is enough to attribute authorship. -->
                <span class="w-1.5 flex-shrink-0"></span>
                <button
                  v-if="line.userId"
                  type="button"
                  class="opacity-0 group-hover:opacity-100 text-xs text-lg-accent-bright hover:text-lg-accent-hover px-1 transition-opacity"
                  @click="startDraft(i + 1)"
                  :title="'Comment on line ' + (i + 1)"
                >+</button>
                <span v-else class="w-[1.25rem] flex-shrink-0"></span>
                <span :class="lineClass(line)" class="whitespace-pre flex-1">{{ line.text || " " }}</span>
                <span
                  v-if="line.liveContributors"
                  class="flex items-center gap-1 flex-shrink-0"
                  :title="
                    line.liveContributors
                      .map((c) => projectStore.memberName(c.userId) + ': ' + c.content.split('\\n')[0])
                      .join('\\n')
                  "
                >
                  <span
                    v-for="c in line.liveContributors"
                    :key="c.userId"
                    class="inline-block w-2 h-2 rounded-full border border-black/40"
                    :class="bgColorFor(c.userId)"
                  ></span>
                </span>
              </div>
              <!-- merge conflict: default expanded with a hide/show toggle. -->
              <template v-if="conflictByLine.get(i + 1) && (conflictByLine.get(i + 1)!.base_start + 1 === i + 1)">
                <button
                  type="button"
                  class="w-full text-left ml-12 mr-3 my-1.5 flex items-center gap-2 px-3 py-1.5 border-l-2 border-lg-rose bg-lg-rose/10 hover:bg-lg-rose/15 rounded-r-md text-xs transition-colors"
                  :style="{ width: 'calc(100% - 3.75rem)' }"
                  @click="toggleConflictCollapse(i + 1)"
                >
                  <span class="text-lg-rose font-medium text-xs">
                    Merge conflict
                  </span>
                  <span class="text-lg-text-sec">
                    <template v-if="conflictByLine.get(i + 1)!.base_start === conflictByLine.get(i + 1)!.base_end">
                      insert before line {{ conflictByLine.get(i + 1)!.base_start + 1 }}
                    </template>
                    <template v-else>
                      lines {{ conflictByLine.get(i + 1)!.base_start + 1 }}-{{ conflictByLine.get(i + 1)!.base_end }}
                    </template>,
                    {{ groupedHunks(conflictByLine.get(i + 1)!).length }} branches
                  </span>
                  <span class="ml-auto text-lg-text-muted">
                    {{ collapsedConflicts.has(i + 1) ? "show ▾" : "hide ▴" }}
                  </span>
                </button>
                <div
                  v-if="!collapsedConflicts.has(i + 1)"
                  class="ml-12 mr-3 mb-2 border border-lg-rose/40 rounded-md bg-lg-bg font-display text-xs"
                  :style="{ width: 'calc(100% - 3.75rem)' }"
                >
                  <div
                    v-for="(group, gi) in groupedHunks(conflictByLine.get(i + 1)!)"
                    :key="group.branch"
                    class="p-3"
                    :class="gi > 0 ? 'border-t border-lg-rose/20' : ''"
                  >
                    <p class="font-semibold text-lg-text mb-3 flex items-center gap-2">
                      <span class="text-[0.65rem] font-mono px-2 py-0.5 rounded bg-lg-surface-2 text-lg-text-sec border border-lg-border">
                        {{ group.branch }}
                      </span>
                      <span class="text-[0.7rem] text-lg-text-muted">
                        {{ group.versions.length }} {{ group.versions.length === 1 ? "version" : "versions" }}
                      </span>
                    </p>
                    <div
                      v-for="(v, vi) in group.versions"
                      :key="vi"
                      class="mt-3 first:mt-0"
                    >
                      <div class="flex flex-wrap gap-1.5 mb-1.5 items-center">
                        <span
                          v-for="(c, ci) in v.contributors"
                          :key="ci"
                          class="inline-block text-xs font-mono px-2.5 py-1 rounded font-medium"
                          :class="badgeClassFor(c.user_id)"
                        >
                          {{ contributorLabel(c) }}
                        </span>
                        <button
                          type="button"
                          class="ml-auto text-[0.65rem] text-lg-text-muted hover:text-lg-accent-bright transition-colors"
                          @click="startReplyToUser(i + 1, v.contributors[0]?.user_id)"
                          :title="`Reply to ${v.contributors[0]?.user_id ? projectStore.memberName(v.contributors[0].user_id) : 'committed'}`"
                        >
                          reply
                        </button>
                      </div>
                      <pre class="whitespace-pre-wrap text-lg-text/90 font-mono bg-lg-surface rounded border border-lg-border p-2 text-[0.75rem] leading-snug">{{ describeContent(v.content) }}</pre>
                    </div>
                  </div>
                </div>
              </template>

              <div
                v-for="c in commentsForLine(i + 1)"
                :key="c.id"
                class="ml-12 mr-3 my-1.5 border-l-2 border-lg-accent bg-lg-surface-2 rounded-r-lg p-3 text-xs"
              >
                <p class="flex items-center gap-2 mb-1.5">
                  <span :class="colorFor(c.user_id)" class="font-semibold">
                    {{ projectStore.memberName(c.user_id) }}{{ c.user_id === auth.user?.id ? " (you)" : "" }}
                  </span>
                  <span class="text-lg-text-muted">{{ fmtCommentTime(c.created_at) }}</span>
                  <button
                    v-if="c.user_id === auth.user?.id"
                    type="button"
                    class="ml-auto text-lg-text-muted hover:text-lg-rose transition-colors"
                    @click="deleteOwnComment(c)"
                    title="Delete"
                  >×</button>
                </p>
                <p class="text-lg-text whitespace-pre-wrap font-display">{{ c.text }}</p>
              </div>
              <div
                v-if="draftLine === i + 1"
                class="ml-12 mr-3 my-1.5 border-l-2 border-lg-accent-bright bg-lg-surface-2 rounded-r-lg p-3 text-xs space-y-2"
              >
                <div class="relative">
                  <textarea
                    rows="2"
                    class="w-full bg-lg-bg border border-lg-border rounded-lg p-2 text-lg-text font-display focus:outline-none focus:border-lg-accent focus:ring-1 focus:ring-lg-accent transition-colors"
                    :placeholder="`Add a comment on line ${i + 1}`"
                    :value="draftText"
                    @input="onDraftInput($event)"
                    @keydown="onDraftKeydown($event)"
                    @keydown.esc.stop="cancelDraft"
                    @keydown.meta.enter.stop="submitDraft"
                    @keydown.ctrl.enter.stop="submitDraft"
                  ></textarea>
                  <div
                    v-if="mentionOpen && mentionCandidates.length > 0"
                    class="absolute left-2 z-20 mt-1 w-64 max-h-48 overflow-auto bg-lg-surface border border-lg-border rounded-lg shadow-lg"
                  >
                    <button
                      v-for="(m, mi) in mentionCandidates"
                      :key="m.id"
                      type="button"
                      class="block w-full text-left px-3 py-1.5 text-xs hover:bg-lg-surface-2 transition-colors"
                      :class="mi === mentionActiveIndex ? 'bg-lg-surface-2' : ''"
                      @mousedown.prevent="pickMention(m.display_name, ($event.currentTarget as HTMLElement).closest('.relative')?.querySelector('textarea') ?? null)"
                    >
                      <span :class="colorFor(m.id)" class="font-semibold">{{ m.display_name }}</span>
                    </button>
                  </div>
                </div>
                <div class="flex items-center gap-2">
                  <button
                    type="button"
                    class="lg-btn-primary py-1 px-3 text-xs disabled:opacity-40"
                    :disabled="!draftText.trim()"
                    @click="submitDraft"
                  >Post</button>
                  <button
                    type="button"
                    class="lg-btn-secondary py-1 px-3 text-xs"
                    @click="cancelDraft"
                  >Cancel</button>
                </div>
              </div>
            </template>
          </div>
        </section>

        <aside class="col-span-2 lg-card p-4 h-[70vh] overflow-auto lg-scroll-dark">
          <h2 class="text-xs text-lg-text-sec font-medium mb-3">
            Active editors
            <span class="text-lg-text-muted">({{ others.length }})</span>
          </h2>
          <p v-if="others.length === 0" class="text-xs text-lg-text-muted">
            No one editing this file right now.
          </p>
          <ul class="space-y-2">
            <li
              v-for="u in others"
              :key="u.user_id"
              class="rounded-lg border border-lg-border bg-lg-surface-2 p-3 text-xs"
            >
              <p class="font-medium flex items-center gap-2">
                <span class="inline-block w-1.5 h-1.5 rounded-full" :class="bgColorFor(u.user_id)"></span>
                <span :class="colorFor(u.user_id)">
                  {{ projectStore.memberName(u.user_id) }}{{ u.user_id === auth.user?.id ? " (you)" : "" }}
                </span>
              </p>
              <p class="text-lg-text-muted mt-1">
                {{ branchOf(u.user_id) }} · lines {{ u.edited_sections[0] }}-{{ u.edited_sections[1] }}
              </p>
            </li>
          </ul>
        </aside>
      </div>
    </main>
  </div>
</template>
