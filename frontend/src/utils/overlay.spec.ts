import { describe, expect, it } from "vitest";
import { buildTree, computeProjectedLines } from "./overlay";
import type { OverlayUserView } from "@/services/ws";

function userView(user_id: string, content: string): OverlayUserView {
  return {
    user_id,
    content,
    edited_sections: [0, content.split("\n").length],
    updated_at_secs: 0,
    updated_at_nanos: 0,
  };
}

describe("buildTree", () => {
  it("returns an empty root for no paths", () => {
    const tree = buildTree([]);
    expect(tree.children).toEqual([]);
  });

  it("places single-segment files at the root", () => {
    const tree = buildTree(["README.md", "LICENSE"]);
    expect(tree.children.map((c) => c.name)).toEqual(["LICENSE", "README.md"]);
    expect(tree.children.every((c) => c.isFile)).toBe(true);
  });

  it("nests multi-segment paths into folders", () => {
    const tree = buildTree(["src/main.rs", "src/lib.rs"]);
    expect(tree.children).toHaveLength(1);
    const src = tree.children[0];
    expect(src.name).toBe("src");
    expect(src.isFile).toBe(false);
    expect(src.children.map((c) => c.name)).toEqual(["lib.rs", "main.rs"]);
  });

  it("orders folders before files at each level", () => {
    const tree = buildTree(["README.md", "src/main.rs"]);
    expect(tree.children.map((c) => ({ name: c.name, isFile: c.isFile }))).toEqual([
      { name: "src", isFile: false },
      { name: "README.md", isFile: true },
    ]);
  });

  it("preserves the full path on leaf nodes", () => {
    const tree = buildTree(["a/b/c.txt"]);
    const c = tree.children[0].children[0].children[0];
    expect(c.fullPath).toBe("a/b/c.txt");
    expect(c.isFile).toBe(true);
  });

  it("does not collide a file and a folder with the same name", () => {
    const tree = buildTree(["foo", "foo/bar.txt"]);
    // "foo" shows up twice: once as a folder (containing bar.txt), once as a file
    const fooEntries = tree.children.filter((c) => c.name === "foo");
    expect(fooEntries).toHaveLength(2);
    expect(fooEntries.some((c) => c.isFile)).toBe(true);
    expect(fooEntries.some((c) => !c.isFile && c.children.length > 0)).toBe(true);
  });
});

describe("computeProjectedLines", () => {
  it("returns base lines untouched when there are no overlays", () => {
    const lines = computeProjectedLines("a\nb\nc", []);
    expect(lines.map((l) => l.text)).toEqual(["a", "b", "c"]);
    expect(lines.every((l) => !l.userId)).toBe(true);
  });

  it("tags lines that diverge from base with the editing user", () => {
    const overlays = [userView("u1", "a\nB\nc")];
    const lines = computeProjectedLines("a\nb\nc", overlays);
    expect(lines[0]).toEqual({ text: "a" });
    expect(lines[1]).toEqual({ text: "B", userId: "u1" });
    expect(lines[2]).toEqual({ text: "c" });
  });

  it("does not mark a line as edited when overlay matches base", () => {
    const overlays = [userView("u1", "a\nb\nc")];
    const lines = computeProjectedLines("a\nb\nc", overlays);
    expect(lines.every((l) => !l.userId)).toBe(true);
  });

  it("last-writer-wins on display but reports liveContributors when two users diverge", () => {
    // local projection tags by author and surfaces a live overlay overlap so
    // the UI can warn before the next backend merge poll catches it.
    const overlays = [userView("u1", "a\nALICE\nc"), userView("u2", "a\nBOB\nc")];
    const lines = computeProjectedLines("a\nb\nc", overlays);
    expect(lines[1].text).toBe("BOB");
    expect(lines[1].userId).toBe("u2");
    expect(lines[1].liveContributors).toEqual([
      { userId: "u1", content: "ALICE" },
      { userId: "u2", content: "BOB" },
    ]);
  });

  it("tags each line with its sole editor when users edit different lines", () => {
    const overlays = [userView("u1", "a\nALICE\nc"), userView("u2", "a\nb\nC")];
    const lines = computeProjectedLines("a\nb\nc", overlays);
    expect(lines[1]).toEqual({ text: "ALICE", userId: "u1" });
    expect(lines[2]).toEqual({ text: "C", userId: "u2" });
  });

  it("extends past the base when an overlay adds new lines", () => {
    const overlays = [userView("u1", "a\nb\nc\nd")];
    const lines = computeProjectedLines("a\nb\nc", overlays);
    expect(lines).toHaveLength(4);
    expect(lines[3]).toEqual({ text: "d", userId: "u1" });
  });

  it("handles an empty base content gracefully", () => {
    const overlays = [userView("u1", "first line")];
    const lines = computeProjectedLines("", overlays);
    expect(lines[0]).toEqual({ text: "first line", userId: "u1" });
  });

  it("only tags the inserted line, not everything below it", () => {
    // base of 5 lines; user inserts X between line 2 and line 3.
    // the naive index comparison wrongly marked every line from the
    // insertion point onward as edited.
    const overlays = [userView("u1", "a\nb\nX\nc\nd\ne")];
    const lines = computeProjectedLines("a\nb\nc\nd\ne", overlays);
    expect(lines.map((l) => l.text)).toEqual(["a", "b", "X", "c", "d", "e"]);
    expect(lines[0].userId).toBeUndefined();
    expect(lines[1].userId).toBeUndefined();
    expect(lines[2]).toEqual({ text: "X", userId: "u1" });
    expect(lines[3].userId).toBeUndefined();
    expect(lines[4].userId).toBeUndefined();
    expect(lines[5].userId).toBeUndefined();
  });

  it("only tags the deleted region, not the lines that shifted up", () => {
    // user removes line 3 from the base
    const overlays = [userView("u1", "a\nb\nd\ne")];
    const lines = computeProjectedLines("a\nb\nc\nd\ne", overlays);
    expect(lines.map((l) => l.text)).toEqual(["a", "b", "d", "e"]);
    expect(lines[2].userId).toBeUndefined();
    expect(lines[3].userId).toBeUndefined();
  });
});
