import { describe, expect, it } from "vitest";
import { conflictsEqual } from "./conflictsEqual";
import type { ConflictHunk, MergeConflict } from "./client";

function hunk(over: Partial<ConflictHunk> = {}): ConflictHunk {
  return {
    branch: "main",
    user_id: "u1",
    base_start: 0,
    base_end: 2,
    content: ["a", "b"],
    ...over,
  };
}

function conflict(over: Partial<MergeConflict> = {}): MergeConflict {
  return {
    base_start: 0,
    base_end: 2,
    hunks: [hunk()],
    ...over,
  };
}

describe("conflictsEqual", () => {
  it("returns true for two structurally identical lists", () => {
    expect(conflictsEqual([conflict()], [conflict()])).toBe(true);
  });

  it("returns true for two independently built deep-equal lists", () => {
    const a: MergeConflict[] = [
      conflict({ hunks: [hunk({ branch: "feat", content: ["x"] }), hunk({ branch: "main" })] }),
    ];
    const b: MergeConflict[] = [
      conflict({ hunks: [hunk({ branch: "feat", content: ["x"] }), hunk({ branch: "main" })] }),
    ];
    expect(conflictsEqual(a, b)).toBe(true);
  });

  it("returns true for two empty lists", () => {
    expect(conflictsEqual([], [])).toBe(true);
  });

  it("returns false when the lists differ in length", () => {
    expect(conflictsEqual([conflict()], [conflict(), conflict()])).toBe(false);
    expect(conflictsEqual([], [conflict()])).toBe(false);
  });

  it("returns false when a conflict's base range differs", () => {
    expect(conflictsEqual([conflict({ base_start: 0 })], [conflict({ base_start: 1 })])).toBe(false);
    expect(conflictsEqual([conflict({ base_end: 2 })], [conflict({ base_end: 3 })])).toBe(false);
  });

  it("returns false when the hunk count differs", () => {
    expect(
      conflictsEqual([conflict({ hunks: [hunk()] })], [conflict({ hunks: [hunk(), hunk()] })]),
    ).toBe(false);
  });

  it("returns false when a hunk's branch differs", () => {
    expect(
      conflictsEqual([conflict({ hunks: [hunk({ branch: "a" })] })], [conflict({ hunks: [hunk({ branch: "b" })] })]),
    ).toBe(false);
  });

  it("returns false when a hunk's user_id differs", () => {
    expect(
      conflictsEqual(
        [conflict({ hunks: [hunk({ user_id: "u1" })] })],
        [conflict({ hunks: [hunk({ user_id: "u2" })] })],
      ),
    ).toBe(false);
  });

  it("returns false when a hunk's base range differs", () => {
    expect(
      conflictsEqual(
        [conflict({ hunks: [hunk({ base_start: 0 })] })],
        [conflict({ hunks: [hunk({ base_start: 5 })] })],
      ),
    ).toBe(false);
  });

  it("returns false when content lines differ", () => {
    expect(
      conflictsEqual(
        [conflict({ hunks: [hunk({ content: ["a", "b"] })] })],
        [conflict({ hunks: [hunk({ content: ["a", "c"] })] })],
      ),
    ).toBe(false);
  });

  it("returns false when content length differs", () => {
    expect(
      conflictsEqual(
        [conflict({ hunks: [hunk({ content: ["a"] })] })],
        [conflict({ hunks: [hunk({ content: ["a", "b"] })] })],
      ),
    ).toBe(false);
  });

  it("treats null and undefined user_id as equal (nullish-coalesced)", () => {
    expect(
      conflictsEqual(
        [conflict({ hunks: [hunk({ user_id: null })] })],
        [conflict({ hunks: [hunk({ user_id: undefined })] })],
      ),
    ).toBe(true);
  });

  it("distinguishes a real user_id from null/undefined", () => {
    expect(
      conflictsEqual(
        [conflict({ hunks: [hunk({ user_id: "u1" })] })],
        [conflict({ hunks: [hunk({ user_id: null })] })],
      ),
    ).toBe(false);
  });
});
