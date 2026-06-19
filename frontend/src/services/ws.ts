import type { Comment, MergeConflict } from "@/types/api";

export interface OverlayUserView {
  user_id: string;
  content: string;
  edited_sections: [number, number];
  updated_at_secs: number;
  updated_at_nanos: number;
}

/// Tagged WS payload matching the backend's WsBroadcast enum. The per-file
/// overlay channel carries typing updates, comment lifecycle, and the
/// initial snapshot pushed on subscription open.
export type WsMessage =
  | { kind: "overlay"; user_id: string; content: string; line_section: [number, number] }
  | { kind: "comment_created"; id: string; user_id: string; line: number; text: string; created_at: number }
  | { kind: "comment_deleted"; id: string }
  | { kind: "snapshot"; comments: Comment[]; all_user_contents: OverlayUserView[] }
  | { kind: "conflicts"; file: string; conflicts: MergeConflict[] };

export interface OverlayWsOpts {
  projectId: string;
  userId: string;
  fileName: string;
  token: string;
}

type MessageHandler = (msg: WsMessage) => void;

export class OverlayWebSocket {
  private socket: WebSocket | null = null;
  private handlers: MessageHandler[] = [];
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private readonly opts: OverlayWsOpts;
  // dispose() runs before the socket fires its async onclose, so without this
  // flag the onclose handler would schedule a reconnect on an instance that
  // the caller has already thrown away , leaving an orphan socket alive.
  private disposed = false;

  constructor(opts: OverlayWsOpts) {
    this.opts = opts;
  }

  connect(): void {
    if (this.disposed) return;
    if (this.socket?.readyState === WebSocket.OPEN) return;
    const base = import.meta.env.VITE_WS_URL || "ws://localhost:8080";
    const path = `/api/overlay/ws/${this.opts.projectId}/${this.opts.userId}/${encodeURIComponent(this.opts.fileName)}`;
    // token goes in query because browsers cant set headers on WS handshake.
    // echo=true so the viewer also sees its own changes (vscode side omits this).
    this.socket = new WebSocket(`${base}${path}?token=${encodeURIComponent(this.opts.token)}&echo=true`);

    this.socket.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data) as WsMessage;
        for (const h of this.handlers) h(msg);
      } catch {
        // ignore malformed
      }
    };

    this.socket.onclose = () => {
      if (this.disposed) return;
      this.reconnectTimer = setTimeout(() => this.connect(), 3000);
    };
  }

  onMessage(handler: MessageHandler): void {
    this.handlers.push(handler);
  }

  send(msg: WsMessage): void {
    this.socket?.send(JSON.stringify(msg));
  }

  dispose(): void {
    // set disposed *before* close() so the async onclose short-circuits
    this.disposed = true;
    if (this.reconnectTimer) clearTimeout(this.reconnectTimer);
    this.reconnectTimer = null;
    this.socket?.close();
    this.socket = null;
    this.handlers = [];
  }
}
