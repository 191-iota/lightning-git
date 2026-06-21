import { diffArrays } from "diff";
import type { TreeNode } from "@/components/FileTreeNode.vue";
import type { OverlayUserView } from "@/services/ws";

export interface ProjLine {
  text: string;
  // last writer wins on display; the line is tagged with their id so the UI
  // can color and label it. real cross-branch conflict semantics live in the
  // backend's merge service.
  userId?: string;
  // a "live overlay overlap" exists when multiple users currently propose
  // different content at the same base position. it is NOT a merge conflict
  // by itself, but it usually predicts one before the next backend poll
  // surfaces it, so the UI surfaces it as a transient warning.
  liveContributors?: { userId: string; content: string }[];
}

export interface UserChange {
  startBaseIdx: number;
  endBaseIdx: number;
  content: string[];
  userId: string;
}

/// Build a nested tree from a flat list of file paths. Folders before files at
/// each level, alphabetical within. Extracted from OverlayView so it can be
/// unit-tested without mounting the view.
export function buildTree(paths: string[]): TreeNode {
  const root: TreeNode = { name: "", fullPath: "", isFile: false, children: [] };
  for (const path of paths) {
    const parts = path.split("/");
    let cur = root;
    for (let i = 0; i < parts.length; i++) {
      const segment = parts[i];
      const isLeaf = i === parts.length - 1;
      let child = cur.children.find((c) => c.name === segment && c.isFile === isLeaf);
      if (!child) {
        child = {
          name: segment,
          fullPath: isLeaf ? path : "",
          isFile: isLeaf,
          children: [],
        };
        cur.children.push(child);
      }
      cur = child;
    }
  }
  const sortRec = (n: TreeNode) => {
    n.children.sort((a, b) => {
      if (a.isFile !== b.isFile) return a.isFile ? 1 : -1;
      return a.name.localeCompare(b.name);
    });
    n.children.forEach(sortRec);
  };
  sortRec(root);
  return root;
}

/// Reduce a single user's overlay against the base into a list of changed
/// regions, expressed as (start_in_base, end_in_base, replacement_content).
/// Uses an LCS-based diff so a one-line insertion does not cascade-mark every
/// line below it as edited, which the naive index comparison would.
export function userChanges(baseLines: string[], user: OverlayUserView): UserChange[] {
  const userLines = user.content.split("\n");
  const parts = diffArrays(baseLines, userLines);
  const changes: UserChange[] = [];
  let baseIdx = 0;
  let pending: UserChange | null = null;

  const flush = () => {
    if (pending) {
      changes.push(pending);
      pending = null;
    }
  };

  for (const part of parts) {
    const count = part.count ?? part.value.length;
    if (!part.added && !part.removed) {
      flush();
      baseIdx += count;
      continue;
    }
    if (!pending) {
      pending = { startBaseIdx: baseIdx, endBaseIdx: baseIdx, content: [], userId: user.user_id };
    }
    if (part.removed) {
      pending.endBaseIdx += count;
      baseIdx += count;
    } else {
      pending.content.push(...part.value);
    }
  }
  flush();
  return changes;
}

/// Compute the per-line merge projection. Lines that match base stay
/// untagged; lines a user actually changed are tagged with that user. When
/// multiple users land on the same base position the latest writer's content
/// is displayed, last-writer-wins. Cross-branch conflict detection is the
/// backend's job, not this function's.
export function computeProjectedLines(
  baseContent: string,
  overlays: OverlayUserView[],
): ProjLine[] {
  const baseLines = (baseContent || "").split("\n");
  if (overlays.length === 0) {
    return baseLines.map((t) => ({ text: t }));
  }

  const allChanges = overlays.flatMap((u) => userChanges(baseLines, u));
  allChanges.sort((a, b) => a.startBaseIdx - b.startBaseIdx || a.endBaseIdx - b.endBaseIdx);

  const out: ProjLine[] = [];
  let baseIdx = 0;
  let i = 0;

  while (i < allChanges.length || baseIdx < baseLines.length) {
    // Drop any change overtaken by a wider earlier one: when two users' ranges
    // overlap, processing the wider range jumps baseIdx past a narrower range's
    // start, leaving it behind the cursor. Its base region is already emitted,
    // so skip it. Without this the loop can never reach `startBaseIdx === baseIdx`
    // for the stranded change and spins forever once the base lines run out.
    // Termination relies on the invariant that every UserChange from
    // `userChanges` has startBaseIdx <= baseLines.length (startBaseIdx is the
    // running base cursor, which only steps over real base lines). Callers must
    // build changes via `userChanges`, not hand-roll them past the base end.
    while (i < allChanges.length && allChanges[i].startBaseIdx < baseIdx) {
      i++;
    }

    const atIdx: UserChange[] = [];
    while (i < allChanges.length && allChanges[i].startBaseIdx === baseIdx) {
      atIdx.push(allChanges[i]);
      i++;
    }

    if (atIdx.length > 0) {
      const users = new Set(atIdx.map((c) => c.userId));
      const distinctContents = new Set(atIdx.map((c) => c.content.join("\n")));
      const overlapping = users.size >= 2 && distinctContents.size >= 2;
      const liveContributors = overlapping
        ? atIdx.map((c) => ({ userId: c.userId, content: c.content.join("\n") }))
        : undefined;
      // prefer the most-recent NON-EMPTY contributor as the displayed
      // version. otherwise a single teammate deleting their whole file
      // (UserChange with content=[] spanning all base lines) wins
      // last-writer-wins and wipes the viewer's projection blank, even
      // when other users still have content for the same range.
      let display = atIdx[atIdx.length - 1];
      if (display.content.length === 0) {
        for (let j = atIdx.length - 2; j >= 0; j--) {
          if (atIdx[j].content.length > 0) {
            display = atIdx[j];
            break;
          }
        }
      }
      const baseEnd = Math.max(...atIdx.map((c) => c.endBaseIdx));
      if (display.content.length === 0 && baseEnd > baseIdx) {
        // every contributor at this region is a pure deletion, but base
        // had lines here. surface them as untagged lines so the viewer
        // still sees the file's structure; the conflict panel separately
        // shows the deletion contributors.
        for (let k = baseIdx; k < baseEnd && k < baseLines.length; k++) {
          out.push({ text: baseLines[k], liveContributors });
        }
      } else {
        for (const line of display.content) {
          out.push({
            text: line,
            userId: display.userId,
            liveContributors,
          });
        }
      }
      baseIdx = baseEnd;
      continue;
    }

    if (baseIdx < baseLines.length) {
      out.push({ text: baseLines[baseIdx] });
      baseIdx++;
    }
  }

  return out;
}
