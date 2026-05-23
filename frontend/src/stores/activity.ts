import { defineStore } from "pinia";
import { ref } from "vue";
import type { ActiveEdit } from "@/types/api";
import { useAuthStore } from "@/stores/auth";

// Project-scoped activity WS. Persists across in-app navigation so
// ProjectView <-> OverlayView dont tear it down on every view change.
// No handshake timeout , that races with the actual connection and produces
// the "closed before established" abort cycle. A slow handshake just stays in
// "connecting" until it either opens or the backend drops it.
export const useActivityStore = defineStore("activity", () => {
  const edits = ref<ActiveEdit[]>([]);
  const state = ref<"closed" | "connecting" | "live">("closed");

  let ws: WebSocket | null = null;
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  let currentProjectId: string | null = null;
  // bumped on every fresh open() call so a stale onclose from a prior socket
  // cant schedule a reconnect that races the new one.
  let generation = 0;

  function ensure(projectId: string) {
    if (
      currentProjectId === projectId &&
      ws &&
      (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING)
    ) {
      return;
    }
    currentProjectId = projectId;
    open();
  }

  function open() {
    if (!currentProjectId) return;
    const auth = useAuthStore();
    if (!auth.token) {
      state.value = "closed";
      return;
    }
    if (reconnectTimer) {
      clearTimeout(reconnectTimer);
      reconnectTimer = null;
    }
    if (ws) {
      ws.close();
      ws = null;
    }
    const myGen = ++generation;
    const base = import.meta.env.VITE_WS_URL || "ws://localhost:8080";
    const url = `${base}/api/projects/${currentProjectId}/activity/ws?token=${encodeURIComponent(auth.token)}`;
    state.value = "connecting";
    const sock = new WebSocket(url);
    ws = sock;

    sock.onopen = () => {
      if (myGen !== generation) return;
      state.value = "live";
    };
    sock.onmessage = (event) => {
      if (myGen !== generation) return;
      try {
        edits.value = JSON.parse(event.data) as ActiveEdit[];
      } catch {
        // ignore malformed
      }
    };
    sock.onerror = (e) => {
      if (myGen !== generation) return;
      console.error("[activity ws] error", e);
    };
    sock.onclose = () => {
      if (myGen !== generation) return;
      state.value = "closed";
      ws = null;
      if (currentProjectId) {
        reconnectTimer = setTimeout(open, 3000);
      }
    };
  }

  function dispose() {
    if (reconnectTimer) clearTimeout(reconnectTimer);
    reconnectTimer = null;
    currentProjectId = null;
    generation++;
    ws?.close();
    ws = null;
    edits.value = [];
    state.value = "closed";
  }

  return { edits, state, ensure, dispose };
});
