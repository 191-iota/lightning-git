import * as vscode from "vscode";
import type { MergeConflict } from "./client";

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
  private latest: MergeConflict[] = [];
  // local disposal flag because the WebviewPanel's own dispose semantics
  // are async — if reveal/update gets called between user-clicks-X and
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
    this.latest = conflicts;
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
    const labelFor = (id: string | null | undefined): string => {
      if (!id) return "committed";
      const name = this.host.authorLabelFor(id);
      return id === me ? `${name} (you, live)` : `${name} (live)`;
    };

    const blocks = conflicts.length === 0
      ? `<p class="empty">No conflicts on this file.</p>`
      : conflicts
          .map((c) => {
            // joint pad length: keep all version <pre>s visually equal-height
            // even when one contributor's overlay is shorter than another's.
            let padTo = Math.max(0, c.base_end - c.base_start);
            for (const b of c.branches) {
              for (const v of b.versions) {
                if (v.content.length > padTo) padTo = v.content.length;
              }
            }
            const renderContent = (lines: string[]): string => {
              if (lines.length === 0 && padTo === 0) return "(removed)";
              let body = lines.join("\n");
              if (lines.length < padTo) {
                const pad = Array(padTo - lines.length).fill("(no content)").join("\n");
                body = body.length > 0 ? `${body}\n${pad}` : pad;
              }
              return body;
            };
            const branchesHtml = c.branches
              .map((b) => {
                const versionsHtml = b.versions
                  .map((v) => {
                    const labels = v.contributors.map(labelFor).join(", ");
                    const display = renderContent(v.content);
                    return `
                      <div class="version">
                        <div class="row">
                          <span class="labels">${escapeHtml(labels)}</span>
                        </div>
                        <pre>${escapeHtml(display)}</pre>
                      </div>
                    `;
                  })
                  .join("");
                return `
                  <div class="branch">
                    <div class="branch-head">
                      <code>${escapeHtml(b.branch)}</code>
                      <span class="meta">${b.versions.length} version${b.versions.length === 1 ? "" : "s"}</span>
                    </div>
                    ${versionsHtml}
                  </div>
                `;
              })
              .join("");
            return `
              <section class="conflict">
                <h2>Lines ${c.base_start + 1}-${c.base_end}</h2>
                ${branchesHtml}
              </section>
            `;
          })
          .join("");

    return `<!doctype html>
<html>
<head>
  <meta charset="utf-8" />
  <style>
    body {
      font-family: var(--vscode-font-family);
      color: var(--vscode-foreground);
      background: var(--vscode-editor-background);
      padding: 12px 18px;
      font-size: 13px;
    }
    h1 { font-size: 14px; margin-bottom: 4px; }
    .file { font-family: var(--vscode-editor-font-family); color: var(--vscode-descriptionForeground); margin-bottom: 14px; }
    .empty { color: var(--vscode-descriptionForeground); }
    .conflict {
      border: 1px solid color-mix(in srgb, var(--vscode-errorForeground) 50%, transparent);
      border-radius: 6px;
      padding: 10px 12px;
      margin-bottom: 16px;
    }
    h2 { font-size: 12px; text-transform: uppercase; letter-spacing: 0.08em; color: var(--vscode-errorForeground); margin: 0 0 8px 0; }
    .branch {
      margin: 10px 0;
      padding-left: 8px;
      border-left: 2px solid var(--vscode-editorWidget-border);
    }
    .branch-head { display: flex; align-items: center; gap: 8px; margin-bottom: 6px; }
    .branch-head code { background: var(--vscode-textBlockQuote-background); padding: 2px 6px; border-radius: 3px; }
    .meta { font-size: 11px; color: var(--vscode-descriptionForeground); }
    .version { margin: 8px 0 12px 0; }
    .row { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin-bottom: 4px; }
    .labels { font-size: 11px; color: var(--vscode-descriptionForeground); }
    pre {
      font-family: var(--vscode-editor-font-family);
      background: var(--vscode-textBlockQuote-background);
      padding: 8px 10px;
      border-radius: 4px;
      margin: 0;
      white-space: pre-wrap;
      overflow-x: auto;
    }
  </style>
</head>
<body>
  <h1>Merge conflicts</h1>
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
