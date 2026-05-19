import { defineStore } from "pinia";
import { ref } from "vue";
import api from "@/services/api";
import type { Comment, CommentEvent } from "@/types/api";
import { useAuthStore } from "@/stores/auth";

export const useCommentsStore = defineStore("comments", () => {
  // file path -> comments. Snapshots arrive project-wide on WS connect; per-file
  // GETs lazily backfill when a user opens a file before the WS has connected.
  const byFile = ref<Map<string, Comment[]>>(new Map());
  let ws: WebSocket | null = null;

  function get(file: string): Comment[] {
    return byFile.value.get(file) ?? [];
  }

  function setFile(file: string, comments: Comment[]) {
    byFile.value.set(file, comments);
    // trigger reactivity since Map mutation doesnt
    byFile.value = new Map(byFile.value);
  }

  function applyEvent(event: CommentEvent) {
    if (event.kind === "snapshot") {
      const next = new Map<string, Comment[]>();
      for (const c of event.comments) {
        if (!next.has(c.file)) next.set(c.file, []);
        next.get(c.file)!.push(c);
      }
      byFile.value = next;
    } else if (event.kind === "created") {
      // event has kind + Comment fields flattened
      const { kind: _kind, ...rest } = event;
      void _kind;
      const c = rest as Comment;
      const list = byFile.value.get(c.file) ?? [];
      if (!list.some((x) => x.id === c.id)) {
        list.push(c);
        list.sort((a, b) => a.line - b.line || a.created_at_secs - b.created_at_secs);
        setFile(c.file, list);
      }
    } else if (event.kind === "deleted") {
      const list = byFile.value.get(event.file);
      if (list) {
        const next = list.filter((c) => c.id !== event.id);
        setFile(event.file, next);
      }
    }
  }

  async function fetchFile(projectId: string, file: string) {
    const { data } = await api.get<Comment[]>(
      `/api/projects/${projectId}/comments?file=${encodeURIComponent(file)}`,
    );
    setFile(file, data);
  }

  async function create(projectId: string, file: string, line: number, body: string) {
    const { data } = await api.post<Comment>(`/api/projects/${projectId}/comments`, {
      file,
      line,
      body,
    });
    applyEvent({ kind: "created", ...data });
  }

  async function remove(projectId: string, commentId: string) {
    await api.delete(`/api/projects/${projectId}/comments/${commentId}`);
    // WS broadcast will update the local map; nothing else to do here.
  }

  function connectWs(projectId: string) {
    const auth = useAuthStore();
    if (!auth.token) return;
    const base = import.meta.env.VITE_WS_URL || "ws://localhost:8080";
    const url = `${base}/api/projects/${projectId}/comments/ws?token=${encodeURIComponent(auth.token)}`;
    ws = new WebSocket(url);
    ws.onmessage = (event) => {
      try {
        applyEvent(JSON.parse(event.data) as CommentEvent);
      } catch {
        // ignore malformed
      }
    };
  }

  function disconnect() {
    ws?.close();
    ws = null;
    byFile.value = new Map();
  }

  return { byFile, get, fetchFile, create, remove, connectWs, disconnect };
});
