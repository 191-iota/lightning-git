export interface OverlayChangeMsg {
  user_id: string;
  content: string;
  line_section: [number, number];
}

export interface OverlayUserView {
  user_id: string;
  content: string;
  edited_sections: [number, number];
  updated_at_secs: number;
  updated_at_nanos: number;
}

export interface OverlayViewMsg {
  content: string;
  original_content: string;
  all_user_contents: OverlayUserView[];
}

export type OverlayWsMsg =
  | { kind: "change"; payload: OverlayChangeMsg }
  | { kind: "view"; payload: OverlayViewMsg };

export interface OverlayWsOpts {
  projectId: string;
  userId: string;
  fileName: string;
  token: string;
}

type MessageHandler = (msg: OverlayWsMsg) => void;

export class OverlayWebSocket {
  private socket: WebSocket | null = null;
  private handlers: MessageHandler[] = [];
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private readonly opts: OverlayWsOpts;

  constructor(opts: OverlayWsOpts) {
    this.opts = opts;
  }

  connect(): void {
    if (this.socket?.readyState === WebSocket.OPEN) return;
    const base = import.meta.env.VITE_WS_URL || "ws://localhost:8080";
    const path = `/api/overlay/ws/${this.opts.projectId}/${this.opts.userId}/${encodeURIComponent(this.opts.fileName)}`;
    // token goes in query because browsers cant set headers on WS handshake
    this.socket = new WebSocket(`${base}${path}?token=${encodeURIComponent(this.opts.token)}`);

    this.socket.onmessage = (event) => {
      try {
        // backend sends serde external-tag form: { Change: ... } or { View: ... }
        const obj = JSON.parse(event.data) as Record<string, unknown>;
        if (obj.Change) {
          for (const h of this.handlers) h({ kind: "change", payload: obj.Change as OverlayChangeMsg });
        } else if (obj.View) {
          for (const h of this.handlers) h({ kind: "view", payload: obj.View as OverlayViewMsg });
        }
      } catch {
        // ignore malformed
      }
    };

    this.socket.onclose = () => {
      this.reconnectTimer = setTimeout(() => this.connect(), 3000);
    };
  }

  onMessage(handler: MessageHandler): void {
    this.handlers.push(handler);
  }

  send(change: OverlayChangeMsg): void {
    this.socket?.send(JSON.stringify(change));
  }

  dispose(): void {
    if (this.reconnectTimer) clearTimeout(this.reconnectTimer);
    this.socket?.close();
    this.socket = null;
  }
}
