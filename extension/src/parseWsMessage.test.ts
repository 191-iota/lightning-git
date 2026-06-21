import { describe, expect, it } from "vitest";
import { parseWsMessage } from "./parseWsMessage";

describe("parseWsMessage", () => {
  it("returns null for invalid JSON", () => {
    expect(parseWsMessage("not json")).toBeNull();
    expect(parseWsMessage("")).toBeNull();
    expect(parseWsMessage("{")).toBeNull();
  });

  it("returns null for valid JSON that is not an object", () => {
    expect(parseWsMessage("42")).toBeNull();
    expect(parseWsMessage('"a string"')).toBeNull();
    expect(parseWsMessage("[]")).toBeNull();
    expect(parseWsMessage("null")).toBeNull();
  });

  it("returns null for an unknown kind", () => {
    expect(parseWsMessage(JSON.stringify({ kind: "bogus" }))).toBeNull();
    expect(parseWsMessage(JSON.stringify({ foo: "bar" }))).toBeNull();
  });

  it("parses an overlay message", () => {
    const raw = JSON.stringify({
      kind: "overlay",
      user_id: "u1",
      content: "hello",
      line_section: [0, 3],
    });
    expect(parseWsMessage(raw)).toEqual({
      kind: "overlay",
      user_id: "u1",
      content: "hello",
      line_section: [0, 3],
    });
  });

  it("rejects an overlay missing a required field", () => {
    expect(parseWsMessage(JSON.stringify({ kind: "overlay", user_id: "u1", content: "x" }))).toBeNull();
    expect(
      parseWsMessage(JSON.stringify({ kind: "overlay", user_id: "u1", content: "x", line_section: [0] })),
    ).toBeNull();
  });

  it("parses a conflicts message and carries the conflict list through", () => {
    const conflicts = [{ base_start: 0, base_end: 2, hunks: [] }];
    const raw = JSON.stringify({ kind: "conflicts", file: "a.ts", conflicts });
    expect(parseWsMessage(raw)).toEqual({ kind: "conflicts", file: "a.ts", conflicts });
  });

  it("rejects a conflicts message without an array of conflicts", () => {
    expect(parseWsMessage(JSON.stringify({ kind: "conflicts", file: "a.ts" }))).toBeNull();
    expect(
      parseWsMessage(JSON.stringify({ kind: "conflicts", file: "a.ts", conflicts: "nope" })),
    ).toBeNull();
  });

  it("parses a comment_created message", () => {
    const raw = JSON.stringify({
      kind: "comment_created",
      id: "c1",
      user_id: "u1",
      line: 5,
      text: "hi",
      created_at: 1700000000,
    });
    expect(parseWsMessage(raw)).toEqual({
      kind: "comment_created",
      id: "c1",
      user_id: "u1",
      line: 5,
      text: "hi",
      created_at: 1700000000,
    });
  });

  it("rejects a comment_created with a non-numeric line", () => {
    expect(
      parseWsMessage(
        JSON.stringify({ kind: "comment_created", id: "c1", user_id: "u1", line: "5", text: "hi", created_at: 1 }),
      ),
    ).toBeNull();
  });

  it("parses a comment_deleted message", () => {
    const raw = JSON.stringify({ kind: "comment_deleted", id: "c1" });
    expect(parseWsMessage(raw)).toEqual({ kind: "comment_deleted", id: "c1" });
  });

  it("rejects a comment_deleted without an id", () => {
    expect(parseWsMessage(JSON.stringify({ kind: "comment_deleted" }))).toBeNull();
  });

  it("parses a snapshot message", () => {
    const snapshot = {
      kind: "snapshot",
      comments: [{ id: "c1", user_id: "u1", line: 1, text: "hi", created_at: 1 }],
      all_user_contents: [
        {
          user_id: "u2",
          content: "code",
          edited_sections: [0, 1],
          updated_at_secs: 10,
          updated_at_nanos: 20,
        },
      ],
    };
    expect(parseWsMessage(JSON.stringify(snapshot))).toEqual(snapshot);
  });

  it("rejects a snapshot missing its arrays", () => {
    expect(parseWsMessage(JSON.stringify({ kind: "snapshot", comments: [] }))).toBeNull();
    expect(parseWsMessage(JSON.stringify({ kind: "snapshot", all_user_contents: [] }))).toBeNull();
  });
});
