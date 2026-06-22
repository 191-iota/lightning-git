import * as vscode from "vscode";
import type { MergeConflict } from "./client";
import { colorForUser } from "./palette";

// Minimal surface the panel needs from its owning session. Keeping this as
// a structural interface (rather than importing OverlaySession) avoids a
// circular module dependency and makes the panel trivially testable.
export interface ConflictPanelHost {
  getCurrentUserId(): string;
  getActiveFile(): string | undefined;
  authorLabelFor(userId: string): string;
}

interface PanelVersion {
  contributors: (string | null | undefined)[];
  content: string[];
}
interface PanelBranch {
  branch: string;
  versions: PanelVersion[];
}
interface PanelConflict {
  base_start: number;
  base_end: number;
  branches: PanelBranch[];
}

// Webview that mirrors the frontend's expanded conflict panel: each
// conflict region shows its branches, each branch its distinct content
// versions, each version a content block grouped by branch + content sig.
export class ConflictPanel {
  private readonly panel: vscode.WebviewPanel;
  // local disposal flag because the WebviewPanel's own dispose semantics
  // are async, if reveal/update gets called between user-clicks-X and
  // onDidDispose firing, calls on the underlying panel throw silently
  // and the session is left holding a dead reference. with this flag the
  // outer toggle path can detect and recreate.
  private disposed = false;

  constructor(
    private readonly host: ConflictPanelHost,
    onDispose: () => void,
  ) {
    this.panel = vscode.window.createWebviewPanel(
      "lightningGitConflicts",
      "Lightning Git: Conflicts",
      { viewColumn: vscode.ViewColumn.Beside, preserveFocus: false },
      { enableScripts: false, retainContextWhenHidden: true },
    );
    this.panel.onDidDispose(() => {
      this.disposed = true;
      onDispose();
    });
  }

  isDisposed(): boolean {
    return this.disposed;
  }

  dispose(): void {
    if (this.disposed) return;
    this.panel.dispose();
  }

  reveal(): void {
    if (this.disposed) return;
    this.panel.reveal(vscode.ViewColumn.Beside, false);
  }

  update(conflicts: MergeConflict[]): void {
    if (this.disposed) return;
    const grouped = conflicts.map((c) => this.groupOne(c));
    this.panel.webview.html = this.renderHtml(grouped);
  }

  private groupOne(conflict: MergeConflict): PanelConflict {
    const byBranch = new Map<string, Map<string, PanelVersion>>();
    for (const h of conflict.hunks) {
      const key = h.content.join("\n");
      let versions = byBranch.get(h.branch);
      if (!versions) {
        versions = new Map();
        byBranch.set(h.branch, versions);
      }
      const existing = versions.get(key);
      if (existing) {
        existing.contributors.push(h.user_id ?? null);
      } else {
        versions.set(key, { contributors: [h.user_id ?? null], content: h.content });
      }
    }
    const branches: PanelBranch[] = [];
    for (const [branch, versions] of byBranch) {
      branches.push({ branch, versions: Array.from(versions.values()) });
    }
    return { base_start: conflict.base_start, base_end: conflict.base_end, branches };
  }

  private renderHtml(conflicts: PanelConflict[]): string {
    const me = this.host.getCurrentUserId();
    const file = this.host.getActiveFile() ?? "";

    // every user id appearing in any hunk, so contributor chips get the same
    // stable colour as the in-editor teammate tints.
    const knownIds: string[] = [];
    for (const c of conflicts) {
      for (const b of c.branches) {
        for (const v of b.versions) {
          for (const id of v.contributors) {
            if (id) {
              knownIds.push(id);
            }
          }
        }
      }
    }

    const chip = (id: string | null | undefined): string => {
      if (!id) {
        return `<span class="chip committed">committed</span>`;
      }
      const name = this.host.authorLabelFor(id);
      const label = id === me ? `${name} (you)` : name;
      return `<span class="chip live" style="--chip:${colorForUser(id, knownIds)}">${escapeHtml(label)}</span>`;
    };

    const blocks = conflicts.length === 0
      ? `<p class="empty"><span class="ok">✓</span> No predicted conflicts on this file. You are clear.</p>`
      : conflicts
          .map((c) => {
            const branchesHtml = c.branches
              .map((b) => {
                const versionsHtml = b.versions
                  .map((v) => {
                    const chips = v.contributors.map(chip).join("");
                    const body = v.content.length === 0
                      ? `<pre class="removed">(removed)</pre>`
                      : `<pre>${escapeHtml(v.content.join("\n"))}</pre>`;
                    return `<div class="version"><div class="chips">${chips}</div>${body}</div>`;
                  })
                  .join("");
                return `<div class="branch"><div class="branch-head"><code>${escapeHtml(b.branch)}</code><span class="meta">${b.versions.length} version${b.versions.length === 1 ? "" : "s"}</span></div>${versionsHtml}</div>`;
              })
              .join("");
            const rangeLabel = c.base_start === c.base_end
              ? `Line ${c.base_start + 1}`
              : `Lines ${c.base_start + 1}–${c.base_end}`;
            return `<section class="conflict"><div class="conflict-head"><span class="pill">${rangeLabel}</span></div>${branchesHtml}</section>`;
          })
          .join("");

    const count = conflicts.length;
    const countBadge = count === 0
      ? ""
      : `<span class="count">${count} region${count === 1 ? "" : "s"}</span>`;

    return `<!doctype html>
<html>
<head>
  <meta charset="utf-8" />
  <style>
    :root { color-scheme: light dark; }
    body {
      font-family: var(--vscode-font-family);
      color: var(--vscode-foreground);
      background: var(--vscode-editor-background);
      padding: 16px 20px 28px;
      font-size: 13px;
      line-height: 1.5;
    }
    header { display: flex; align-items: baseline; gap: 10px; margin: 0; }
    h1 { font-size: 15px; font-weight: 600; margin: 0; }
    .count {
      font-size: 11px; font-weight: 600;
      color: var(--vscode-errorForeground);
      border: 1px solid color-mix(in srgb, var(--vscode-errorForeground) 45%, transparent);
      border-radius: 999px; padding: 1px 9px;
    }
    .file { font-family: var(--vscode-editor-font-family); color: var(--vscode-descriptionForeground); margin: 4px 0 18px; font-size: 12px; }
    .empty { color: var(--vscode-descriptionForeground); }
    .ok { color: var(--vscode-testing-iconPassed, #3fb950); font-weight: 700; }
    .conflict {
      border: 1px solid color-mix(in srgb, var(--vscode-errorForeground) 35%, transparent);
      border-left-width: 3px;
      border-radius: 8px;
      padding: 12px 14px 14px;
      margin-bottom: 16px;
      background: color-mix(in srgb, var(--vscode-errorForeground) 4%, transparent);
    }
    .conflict-head { margin-bottom: 10px; }
    .pill {
      font-size: 11px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.06em;
      color: var(--vscode-errorForeground);
      background: color-mix(in srgb, var(--vscode-errorForeground) 14%, transparent);
      border-radius: 999px; padding: 2px 10px;
    }
    .branch { margin: 12px 0 0; padding-left: 10px; border-left: 2px solid var(--vscode-editorWidget-border); }
    .branch-head { display: flex; align-items: center; gap: 8px; margin-bottom: 8px; }
    .branch-head code { background: var(--vscode-textBlockQuote-background); padding: 2px 7px; border-radius: 4px; font-weight: 600; }
    .meta { font-size: 11px; color: var(--vscode-descriptionForeground); }
    .version { margin: 0 0 12px; }
    .chips { display: flex; flex-wrap: wrap; gap: 5px; margin-bottom: 5px; }
    .chip {
      font-size: 11px; line-height: 1.7; border-radius: 999px; padding: 0 9px;
      border: 1px solid transparent;
    }
    .chip.live {
      color: var(--chip);
      border-color: color-mix(in srgb, var(--chip) 50%, transparent);
      background: color-mix(in srgb, var(--chip) 14%, transparent);
    }
    .chip.committed { color: var(--vscode-descriptionForeground); background: var(--vscode-textBlockQuote-background); }
    pre {
      font-family: var(--vscode-editor-font-family);
      background: var(--vscode-textCodeBlock-background, var(--vscode-textBlockQuote-background));
      padding: 9px 11px; border-radius: 6px; margin: 0;
      white-space: pre-wrap; overflow-x: auto;
      border: 1px solid var(--vscode-editorWidget-border);
    }
    pre.removed { color: var(--vscode-descriptionForeground); font-style: italic; }
  </style>
</head>
<body>
  <header><h1>Merge conflicts</h1>${countBadge}</header>
  <p class="file">${escapeHtml(file)}</p>
  ${blocks}
</body>
</html>`;
  }
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}
