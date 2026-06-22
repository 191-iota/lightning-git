import { describe, it, expect } from "vitest";
import {
  TEAMMATE_PALETTE,
  colorForIndex,
  colorForUser,
  initialsFor,
  withAlpha,
} from "./palette";

describe("colorForIndex", () => {
  it("returns palette entries in order", () => {
    expect(colorForIndex(0)).toBe(TEAMMATE_PALETTE[0]);
    expect(colorForIndex(1)).toBe(TEAMMATE_PALETTE[1]);
  });

  it("wraps around the palette length", () => {
    expect(colorForIndex(TEAMMATE_PALETTE.length)).toBe(TEAMMATE_PALETTE[0]);
    expect(colorForIndex(TEAMMATE_PALETTE.length + 2)).toBe(TEAMMATE_PALETTE[2]);
  });

  it("handles a negative index without throwing", () => {
    expect(TEAMMATE_PALETTE).toContain(colorForIndex(-1));
  });
});

describe("colorForUser", () => {
  it("assigns distinct colours to a small team by sorted position", () => {
    const ids = ["ccc", "aaa", "bbb"];
    // sorted: aaa, bbb, ccc -> palette 0,1,2
    expect(colorForUser("aaa", ids)).toBe(TEAMMATE_PALETTE[0]);
    expect(colorForUser("bbb", ids)).toBe(TEAMMATE_PALETTE[1]);
    expect(colorForUser("ccc", ids)).toBe(TEAMMATE_PALETTE[2]);
  });

  it("is stable regardless of the order ids are passed in", () => {
    expect(colorForUser("bbb", ["aaa", "bbb", "ccc"])).toBe(
      colorForUser("bbb", ["ccc", "bbb", "aaa"]),
    );
  });

  it("falls back to a consistent colour for an unknown id", () => {
    const a = colorForUser("zzz", ["aaa"]);
    const b = colorForUser("zzz", ["aaa"]);
    expect(a).toBe(b);
    expect(TEAMMATE_PALETTE).toContain(a);
  });
});

describe("initialsFor", () => {
  it("takes initials of a two-word name", () => {
    expect(initialsFor("Ada Lovelace")).toBe("AL");
  });

  it("takes the first two letters of a single word", () => {
    expect(initialsFor("octocat")).toBe("OC");
  });

  it("uses first and last for three or more words", () => {
    expect(initialsFor("Jean Luc Picard")).toBe("JP");
  });

  it("returns a placeholder for an empty name", () => {
    expect(initialsFor("   ")).toBe("?");
  });
});

describe("withAlpha", () => {
  it("appends a two-digit hex alpha", () => {
    expect(withAlpha("#4a9eff", 255)).toBe("#4a9effff");
    expect(withAlpha("#4a9eff", 0)).toBe("#4a9eff00");
    expect(withAlpha("#4a9eff", 16)).toBe("#4a9eff10");
  });

  it("clamps out-of-range alpha", () => {
    expect(withAlpha("#000000", 999)).toBe("#000000ff");
    expect(withAlpha("#000000", -5)).toBe("#00000000");
  });
});
