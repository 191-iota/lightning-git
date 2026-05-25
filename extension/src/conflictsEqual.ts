import type { MergeConflict } from "./client";

// Shallow content fingerprint: two conflict lists are "the same render" if
// every hunk's branch + user + range + content lines match. cheap enough
// to run on every recompute; saves the expensive setDecorations + webview
// html reset when nothing actually changed.
export function conflictsEqual(a: MergeConflict[], b: MergeConflict[]): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) {
    const x = a[i];
    const y = b[i];
    if (x.base_start !== y.base_start || x.base_end !== y.base_end) return false;
    if (x.hunks.length !== y.hunks.length) return false;
    for (let j = 0; j < x.hunks.length; j++) {
      const hx = x.hunks[j];
      const hy = y.hunks[j];
      if (
        hx.branch !== hy.branch
        || (hx.user_id ?? null) !== (hy.user_id ?? null)
        || hx.base_start !== hy.base_start
        || hx.base_end !== hy.base_end
        || hx.content.length !== hy.content.length
      ) {
        return false;
      }
      for (let k = 0; k < hx.content.length; k++) {
        if (hx.content[k] !== hy.content[k]) return false;
      }
    }
  }
  return true;
}
