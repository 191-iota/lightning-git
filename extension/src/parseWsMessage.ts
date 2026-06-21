import type { Comment, MergeConflict } from "./client";

/// Tagged WS message matching the backend's WsBroadcast enum. All payloads
/// (live typing, comment lifecycle, conflict set, initial snapshot) flow
/// through the per-file overlay channel.
export type WsMessage =
  | { kind: "overlay"; user_id: string; content: string; line_section: [number, number] }
  | { kind: "conflicts"; file: string; conflicts: MergeConflict[] }
  | { kind: "comment_created"; id: string; user_id: string; line: number; text: string; created_at: number }
  | { kind: "comment_deleted"; id: string }
  | {
      kind: "snapshot";
      comments: Comment[];
      all_user_contents: Array<{
        user_id: string;
        content: string;
        edited_sections: [number, number];
        updated_at_secs: number;
        updated_at_nanos: number;
      }>;
    };

function isObject(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

function isLineSection(v: unknown): v is [number, number] {
  return Array.isArray(v) && v.length === 2 && typeof v[0] === "number" && typeof v[1] === "number";
}

/// Parses and structurally validates a raw WS frame against the discriminated
/// union. Returns null on invalid JSON or any shape that does not match a
/// known message kind, so callers never have to defend against a half-formed
/// payload. This is the stable WS contract the extension and frontend share.
export function parseWsMessage(raw: string): WsMessage | null {
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return null;
  }
  if (!isObject(parsed)) return null;

  switch (parsed.kind) {
    case "overlay":
      if (
        typeof parsed.user_id === "string"
        && typeof parsed.content === "string"
        && isLineSection(parsed.line_section)
      ) {
        return {
          kind: "overlay",
          user_id: parsed.user_id,
          content: parsed.content,
          line_section: parsed.line_section,
        };
      }
      return null;

    case "conflicts":
      if (typeof parsed.file === "string" && Array.isArray(parsed.conflicts)) {
        return {
          kind: "conflicts",
          file: parsed.file,
          conflicts: parsed.conflicts as MergeConflict[],
        };
      }
      return null;

    case "comment_created":
      if (
        typeof parsed.id === "string"
        && typeof parsed.user_id === "string"
        && typeof parsed.line === "number"
        && typeof parsed.text === "string"
        && typeof parsed.created_at === "number"
      ) {
        return {
          kind: "comment_created",
          id: parsed.id,
          user_id: parsed.user_id,
          line: parsed.line,
          text: parsed.text,
          created_at: parsed.created_at,
        };
      }
      return null;

    case "comment_deleted":
      if (typeof parsed.id === "string") {
        return { kind: "comment_deleted", id: parsed.id };
      }
      return null;

    case "snapshot":
      if (Array.isArray(parsed.comments) && Array.isArray(parsed.all_user_contents)) {
        return {
          kind: "snapshot",
          comments: parsed.comments as Comment[],
          all_user_contents:
            parsed.all_user_contents as Extract<WsMessage, { kind: "snapshot" }>["all_user_contents"],
        };
      }
      return null;

    default:
      return null;
  }
}
