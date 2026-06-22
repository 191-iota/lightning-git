// Per-teammate visual identity. Colours are assigned by a user's stable
// sorted position within the known set, mirroring the web frontend's
// colorIndex, so the same person keeps the same colour across renders instead
// of flickering between repaints. A char-hash fallback covers an id that is
// not in the known set yet (e.g. a hover firing a frame before the snapshot).

export const TEAMMATE_PALETTE = [
  "#4a9eff", // blue
  "#e0af68", // amber
  "#9ece6a", // green
  "#bb9af7", // violet
  "#f7768e", // rose
  "#7dcfff", // cyan
  "#ff9e64", // orange
  "#73daca", // teal
] as const;

export function colorForIndex(index: number): string {
  const len = TEAMMATE_PALETTE.length;
  return TEAMMATE_PALETTE[((index % len) + len) % len];
}

function hashIndex(value: string): number {
  let hash = 0;
  for (let i = 0; i < value.length; i++) {
    hash = (hash * 31 + value.charCodeAt(i)) | 0;
  }
  return Math.abs(hash);
}

// Stable colour for a user id given the full set of known ids. Sorting makes a
// small team's colours deterministic and distinct; an unknown id falls back to
// a hash so it still gets a consistent colour.
export function colorForUser(userId: string, knownUserIds: readonly string[]): string {
  const sorted = Array.from(new Set(knownUserIds)).sort();
  const idx = sorted.indexOf(userId);
  return colorForIndex(idx >= 0 ? idx : hashIndex(userId));
}

// One or two uppercase letters for an avatar-style badge: initials of a
// two-word name, otherwise the first two characters. Used as the gutter glyph
// stand-in and in the teammate hover.
export function initialsFor(name: string): string {
  const parts = name.trim().split(/\s+/).filter((p) => p.length > 0);
  if (parts.length === 0) {
    return "?";
  }
  if (parts.length === 1) {
    return parts[0].slice(0, 2).toUpperCase();
  }
  return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase();
}

// An ARGB-ish hex with an alpha suffix appended, for low-opacity line tints
// (VS Code accepts #rrggbbaa). alpha is 0..255.
export function withAlpha(hexColor: string, alpha: number): string {
  const clamped = Math.max(0, Math.min(255, Math.round(alpha)));
  return `${hexColor}${clamped.toString(16).padStart(2, "0")}`;
}
