import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { OverlayWebSocket, type WsMessage } from "./ws";
import type { MergeConflict } from "@/types/api";

// Minimal stand-in for the browser WebSocket. We don't assert on the URL or
// query params (those are expected to change) - we only need a handle to
// drive onmessage and observe what the OverlayWebSocket does with payloads.
class FakeWebSocket {
  static OPEN = 1;
  static instances: FakeWebSocket[] = [];
  readyState = 0;
  onmessage: ((event: { data: string }) => void) | null = null;
  onclose: (() => void) | null = null;
  sent: string[] = [];
  url: string;

  constructor(url: string) {
    this.url = url;
    FakeWebSocket.instances.push(this);
  }

  // simulate a frame arriving from the backend
  receive(payload: unknown) {
    this.onmessage?.({ data: JSON.stringify(payload) });
  }

  send(data: string) {
    this.sent.push(data);
  }

  close() {
    /* no-op */
  }
}

function conflict(base_start: number, base_end: number): MergeConflict {
  return {
    base_start,
    base_end,
    hunks: [
      { branch: "feature", base_start, base_end, content: ["x"] },
    ],
  };
}

function newSocket() {
  const ws = new OverlayWebSocket({
    projectId: "p1",
    userId: "u1",
    fileName: "src/main.rs",
    token: "tok",
  });
  ws.connect();
  return ws;
}

function lastFake(): FakeWebSocket {
  return FakeWebSocket.instances[FakeWebSocket.instances.length - 1];
}

describe("OverlayWebSocket message handling", () => {
  beforeEach(() => {
    FakeWebSocket.instances = [];
    vi.stubGlobal("WebSocket", FakeWebSocket);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("parses an incoming conflicts frame and hands the typed message to subscribers", () => {
    const ws = newSocket();
    const received: WsMessage[] = [];
    ws.onMessage((m) => received.push(m));

    const conflicts = [conflict(0, 1), conflict(3, 4)];
    lastFake().receive({ kind: "conflicts", file: "src/main.rs", conflicts });

    expect(received).toHaveLength(1);
    const msg = received[0];
    expect(msg.kind).toBe("conflicts");
    if (msg.kind === "conflicts") {
      expect(msg.file).toBe("src/main.rs");
      expect(msg.conflicts).toEqual(conflicts);
    }
  });

  it("replaces the conflict set wholesale on each frame (no client-side union/merge)", () => {
    const ws = newSocket();
    // a consumer that mirrors the documented contract: the latest frame's
    // conflicts ARE the conflict set; the client must not accumulate.
    let current: MergeConflict[] = [];
    ws.onMessage((m) => {
      if (m.kind === "conflicts") current = m.conflicts;
    });

    const first = [conflict(0, 1), conflict(3, 4)];
    lastFake().receive({ kind: "conflicts", file: "src/main.rs", conflicts: first });
    expect(current).toEqual(first);

    // a later frame carrying a single conflict must fully replace the prior
    // two - not be unioned/merged into them.
    const second = [conflict(7, 8)];
    lastFake().receive({ kind: "conflicts", file: "src/main.rs", conflicts: second });
    expect(current).toEqual(second);
    expect(current).toHaveLength(1);

    // an empty conflicts frame clears the set entirely.
    lastFake().receive({ kind: "conflicts", file: "src/main.rs", conflicts: [] });
    expect(current).toEqual([]);
  });

  it("fans a frame out to every registered handler", () => {
    const ws = newSocket();
    const a: WsMessage[] = [];
    const b: WsMessage[] = [];
    ws.onMessage((m) => a.push(m));
    ws.onMessage((m) => b.push(m));

    lastFake().receive({ kind: "conflicts", file: "src/main.rs", conflicts: [conflict(0, 1)] });

    expect(a).toHaveLength(1);
    expect(b).toHaveLength(1);
    expect(a[0]).toEqual(b[0]);
  });

  it("silently ignores a malformed frame instead of throwing", () => {
    const ws = newSocket();
    const received: WsMessage[] = [];
    ws.onMessage((m) => received.push(m));

    expect(() => {
      lastFake().onmessage?.({ data: "{ not valid json" });
    }).not.toThrow();
    expect(received).toHaveLength(0);
  });
});
