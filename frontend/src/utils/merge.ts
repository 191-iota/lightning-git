import { diffLines } from "diff";
import type { ConflictHunk, MergeConflict } from "@/types/api";

// Direct port of the backend's merge_service.rs:
//   compute_combined_diff  -> computeCombinedDiff
//   compute_conflicts      -> computeConflicts
//
// Walks a line-level diff of each source against base, builds per-source
// hunks tagged with (branch, user_id), then groups overlapping hunks into
// Conflict clusters and filters out trivial groups (single source, all
// identical changes). The frontend version runs over live overlays only
// (no committed branches), so the backend's 60s poll response — which uses
// the same algorithm but adds committed branches to its source list — is
// a strict superset and slots in seamlessly via flattenConflicts.

export interface SourceInput {
  branch: string;
  userId: string | null;
  content: string;
}

export function computeCombinedDiff(
  baseContent: string,
  sources: SourceInput[],
): ConflictHunk[] {
  const all: ConflictHunk[] = [];
  for (const src of sources) {
    if (src.content === baseContent) continue;
    if (src.content.trim() === "") continue;

    const parts = diffLines(baseContent, src.content);
    let baseLine = 0;
    let hunkStart: number | null = null;
    let changedLines: string[] = [];

    const flush = (endLine: number) => {
      if (hunkStart === null) return;
      all.push({
        branch: src.branch,
        user_id: src.userId,
        base_start: hunkStart,
        base_end: endLine,
        content: changedLines,
      });
      hunkStart = null;
      changedLines = [];
    };

    for (const part of parts) {
      const lines = part.value.split("\n");
      if (lines.length > 0 && lines[lines.length - 1] === "") lines.pop();
      const count = lines.length;
      if (!part.added && !part.removed) {
        flush(baseLine);
        baseLine += count;
      } else if (part.added) {
        if (hunkStart === null) hunkStart = baseLine;
        changedLines.push(...lines);
      } else {
        if (hunkStart === null) hunkStart = baseLine;
        baseLine += count;
      }
    }
    flush(baseLine);
  }
  return all;
}

export function computeConflicts(allHunks: ConflictHunk[]): MergeConflict[] {
  if (allHunks.length === 0) return [];
  const sorted = [...allHunks].sort(
    (a, b) => a.base_start - b.base_start || a.base_end - b.base_end,
  );
  const groups: { start: number; end: number; hunks: ConflictHunk[] }[] = [];
  for (const hunk of sorted) {
    const last = groups[groups.length - 1];
    if (last && hunk.base_start <= last.end) {
      last.end = Math.max(last.end, hunk.base_end);
      last.hunks.push(hunk);
    } else {
      groups.push({ start: hunk.base_start, end: hunk.base_end, hunks: [hunk] });
    }
  }
  const out: MergeConflict[] = [];
  for (const g of groups) {
    const sources = new Set(
      g.hunks.map((h) => `${h.branch}:${h.user_id ?? ""}`),
    );
    if (sources.size < 2) continue;
    const sig = (h: ConflictHunk) =>
      `${h.base_start}:${h.base_end}:${h.content.join("\n")}`;
    const firstSig = sig(g.hunks[0]);
    if (g.hunks.slice(1).every((h) => sig(h) === firstSig)) continue;
    out.push({ base_start: g.start, base_end: g.end, hunks: g.hunks });
  }
  return out;
}

// Merge backend conflicts into live ones. Since both run the SAME algorithm,
// conflicts at identical ranges have identical hunks for the live sources;
// the backend version is a superset (adds committed-branch hunks) and
// replaces the live entry at that range. Live conflicts at ranges the
// backend didn't cover (uncommon — backend sees more sources than live)
// are kept as fallback.
export function flattenConflicts(
  live: MergeConflict[],
  backend: MergeConflict[],
): MergeConflict[] {
  const out = [...backend];
  for (const lc of live) {
    const overlaps = out.some(
      (c) => !(lc.base_end < c.base_start || lc.base_start > c.base_end),
    );
    if (!overlaps) out.push(lc);
  }
  out.sort((a, b) => a.base_start - b.base_start);
  return out;
}
