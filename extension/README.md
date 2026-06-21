# lightning-git-vsc

![VS Code](https://img.shields.io/badge/VS%20Code-extension-1a1a1a?style=flat-square&logo=visualstudiocode&logoColor=white) ![TypeScript](https://img.shields.io/badge/TypeScript-1a1a1a?style=flat-square&logo=typescript&logoColor=white) ![self-hosted](https://img.shields.io/badge/self--hosted-9b2c2c?style=flat-square)

The Lightning Git VS Code extension. It is the developer's surface for Lightning Git: a live session that paints teammates' uncommitted edits and predicted merge conflicts directly into the editor, and an always-present Notbremse to wipe your live state if you typed something you shouldn't have.

Lightning Git is a real-time visibility layer on top of Git. Git tracks finished states (commits) but says nothing about the hours or days between them, when someone is mid-edit on a file you are also touching and a conflict is quietly forming. Lightning Git fills that gap by mirroring each repo read-only on a backend, holding everyone's in-flight edits as ephemeral overlay state in RAM, and streaming that state to the people who need to see it. This extension is where the code actually lives, so it is where that information is most useful.

The product is three repos against one backend:

- [lightning-git-backend](../backend): the Rust/actix-web server that owns the read-only repo clones, the in-RAM overlay state, the WebSocket realtime layer, and merge-conflict prediction.
- [lightning-git-frontend](../frontend): the Vue 3 web dashboard for non-coding stakeholders: Kanban, the live overlay view, org and project management.
- `lightning-git-vsc` (this repo): the VS Code extension.

It is an HF diploma-thesis prototype. Live site: [lightning-git.com](https://lightning-git.com).

<p align="center">
  <img src="assets/conflict-live.gif" alt="Two developers edit the same line; the conflict is detected live, before any commit, then cleared by the Notbremse" width="780">
</p>

## What a developer sees

<p align="center">
  <img src="assets/developer-view.png" alt="A VS Code window showing a teammate's edit as a line tint, a predicted conflict in the gutter, and the Notbremse in the status bar" width="900">
</p>

Start a session and the extension begins watching your active editor. As teammates type on the same file from their own machines, their changes arrive over a WebSocket and the extension peeks them inline, a whole-line tint and an italic label showing which teammate changed which line. Another developer in the same file becomes visible without anyone having to ask. A status bar item on the right shows `N editing` and flashes a warning background each time a teammate's keystrokes land, so activity is visible even when you are not looking at the decorated lines. If you want the full picture of a teammate's version, `Lightning Git: View Teammate Change` opens a normal VS Code diff between your file and theirs.

Predicted merge conflicts paint in the gutter before anyone commits. When two people change the same lines in incompatible ways, those line ranges get an orange left border and an overview-ruler tick, and a second status bar item shows `N conflict(s)`. The conflict panel lists each region, grouped by branch and author. A forming conflict shows up while it is still cheap to talk about, hours before either branch reaches a pull request.

The Notbremse (German for emergency brake, kept as-is) sits at the far left of the status bar as a warning-colored `$(zap) Notbremse` button. It is shown the moment the extension activates and stays there whether or not a session is running, because the moment you need it is exactly when something went wrong. Clicking it shows a modal confirm and then resets your server-side overlay on every file in the project back to the committed base. Its purpose is credential safety: if you pasted a token into a file, the brake clears it from the backend's memory and from teammates' views before it spreads further. It is a reactive control. It only helps if you notice, and it cannot recall edits that already went out. The `lightningGit.debounceMs` setting is the other half of the same safety property, a larger debounce delays broadcasting your local edits, which widens the window to hit the brake before a secret syncs to anyone.

## Conflict prediction, two ways at once

<p align="center">
  <img src="assets/conflict-fusion.png" alt="A 60-second authoritative backend poll fused with per-keystroke client synthesis; the same algorithm on both sides, so they merge without flicker" width="900">
</p>

The genuinely tricky part of this extension is showing conflicts that are both fast and correct, because those two goals pull in opposite directions.

The authoritative answer comes from the backend. `OverlaySession` polls `GET /api/merge/{projectId}/{file}` every 60 seconds (`CONFLICT_POLL_INTERVAL_MS = 60_000`). That poll is the source of truth because the backend can diff against committed branches the editor cannot see, branches no one in this session has open. But a minute is a long time to wait when two people are actively typing, so the extension also computes conflicts on the client.

`src/liveConflicts.ts` is a line-for-line port of the backend's `merge_service.rs`. Its `computeCombinedDiff` walks a `diffLines` diff of each source against the base, builds per-source hunks tagged with branch and `user_id`, and `computeConflicts` groups overlapping base-line ranges and drops the trivial groups, single-source clusters and clusters where every hunk made the identical change. On every teammate keystroke, `doRecomputeAndRender` assembles the overlay set (your local document text plus each teammate's content), calls `synthesizeLiveConflicts`, and fuses the result with the last backend poll through `mergeWithBackend`.

Fusion works because both sides run the same algorithm, so their ranges align exactly. A backend conflict replaces any live conflict whose range overlaps it, using a plain interval test:

```ts
const overlaps = out.some(
  (c) => !(lc.base_end < c.base_start || lc.base_start > c.base_end),
);
```

Live conflicts at ranges the backend has not yet reported are kept as a fallback. When the next 60-second poll lands, its result is a strict superset and slots in without flicker, there is no jump between two differently-computed conflict sets, because there are not two different computations.

Recompute fires constantly, so the render path is guarded against thrashing VS Code's IPC. Recomputes are coalesced through a 60ms `renderTimer`. Then `conflictsEqual` does a cheap deep fingerprint of the new conflict set against the current one (lengths, each conflict's range, each hunk's branch, user, range and content lines), and `doRecomputeAndRender` stores the new set but returns before touching `setDecorations`, the gutter, or the webview when nothing actually changed. Two teammates typing at once used to fire a synchronous render per keystroke and saturate the extension-host channel until VS Code stopped responding to commands; the equality short-circuit plus the debounce keeps the channel idle when there is nothing to repaint.

One more class of bug shaped the code: focus. When the conflict-panel webview takes focus, `vscode.window.activeTextEditor` is `undefined`, which would silently drop your own content from the synthesis and flip the panel to "No conflicts" mid-divergence. So the session never reads `activeTextEditor`. `findVisibleEditor` scans `visibleTextEditors` for the tracked file, `applyConflictGutter` iterates the visible editors, and `doRecomputeAndRender` resolves the local document by path via `findOpenDocument` over `workspace.textDocuments`. Editor resolution is by tracked path, not by focus.

## Session lifecycle

A session is keyed to a project, and the extension finds the project for you rather than asking. On start it reads the workspace's `git remote.origin.url`, normalizes it to `owner/repo`, and matches it against the projects in your organizations, caching the result in `workspaceState` under `lightningGit.projectId`. Projects themselves are created only in the web frontend; the extension links to one that already exists.

Once a session is running, the active editor drives everything. On each active-editor change, `openDocumentOverlay` reads the current branch with `git rev-parse --abbrev-ref HEAD`, registers the file with the backend over `PUT /api/overlay/{projectId}/{userId}/{file}?branch={branch}` (a 400 is treated as already-registered), fetches the base content once from `origin/main`, deliberately main, not your feature branch, so client synthesis diffs against the same target the backend does, and opens the per-file WebSocket at `{wsUrl}/api/overlay/ws/{projectId}/{userId}/{file}?token={jwt}`. The token rides in the query string because the backend accepts the JWT either as an `Authorization` header or as a `?token=` query param, the fallback that lets browser WebSockets authenticate, and the extension uses the query-param form to share one code path with the web client.

Your own edits flow the other way. `onDidChangeTextDocument` queues a send debounced by `debounceMs` (default 250ms); `sendDocumentChange` then transmits the full document text with `line_section: [0, lineCount-1]` over the same socket. Teammate edits, comments, and the initial snapshot all arrive on that one socket, there is no separate poll for comments. The 60-second `/api/merge` poll is the only recurring backend poll besides the local edit debounce; the only other clocks are short one-shot UI timers (the 60ms render coalesce, the peek auto-hide, the status-bar flash).

```mermaid
sequenceDiagram
    participant Dev as Developer (editor)
    participant Ext as OverlaySession
    participant BE as Backend
    participant Mate as Teammate session

    Dev->>Ext: open / switch active file
    Ext->>BE: PUT /api/overlay/.../{file}?branch=...
    Ext->>BE: GET /api/projects/{id}/file?branch=main (base)
    Ext->>BE: open WS /api/overlay/ws/.../{file}?token=...
    BE-->>Ext: snapshot (comments + all_user_contents)

    loop every 60s
        Ext->>BE: GET /api/merge/{id}/{file}
        BE-->>Ext: authoritative conflicts (incl. committed branches)
    end

    Dev->>Ext: type (debounced ~250ms)
    Ext->>BE: WS overlay { content, line_section }
    BE-->>Mate: WS overlay (teammate sees the edit)
    Mate-->>BE: WS overlay (their edit)
    BE-->>Ext: WS overlay
    Ext->>Ext: synthesize live conflicts, fuse with last poll
    Ext->>Dev: peek decorations + gutter + status bar

    Dev->>Ext: Notbremse
    Ext->>BE: DELETE /api/overlay/me/{projectId}
    BE-->>Mate: WS overlay reset to base
    BE-->>Ext: { reset: n }
```

On the backend side, the Notbremse is a single in-memory pass: `reset_user_overlays` walks every file overlay in the project, resets the calling user's content to the cached base, and broadcasts the reset on each file's channel so teammates' views snap back. It never touches git, so recovery is instant and cannot damage history.

## Commands

All command ids live under the `lightning-git.*` namespace. Note that the settings namespace is `lightningGit` instead, the two differ.

| Command id | Title |
|---|---|
| `lightning-git.register` | Lightning Git: Register |
| `lightning-git.login` | Lightning Git: Login |
| `lightning-git.logout` | Lightning Git: Logout |
| `lightning-git.startSession` | Lightning Git: Start Session |
| `lightning-git.stopSession` | Lightning Git: Stop Session |
| `lightning-git.viewChange` | Lightning Git: View Teammate Change |
| `lightning-git.peekTeammates` | Lightning Git: Peek Teammate Changes |
| `lightning-git.peekConflicts` | Lightning Git: Peek Merge Conflicts |
| `lightning-git.notbremse` | Lightning Git: Notbremse (Wipe Live Session) |

`register`, `login`, `logout`, `startSession`, `stopSession`, `viewChange`, and `notbremse` register at activation. `peekTeammates` and `peekConflicts` register inside `OverlaySession.start()`, so they only do anything during a live session and bind to the two status bar items.

## Settings

Configured under `lightningGit` (Settings UI title "Lightning Git"):

| Setting | Type | Default | Description |
|---|---|---|---|
| `lightningGit.apiUrl` | string | `http://localhost:8080` | Backend API endpoint |
| `lightningGit.wsUrl` | string | `ws://localhost:8080` | WebSocket server URL |
| `lightningGit.debounceMs` | number | `250` | Delay before broadcasting local edits to teammates. Lower values feel more live; higher values give a wider window to clear accidentally typed credentials with the Notbremse before sync. |

## How it talks to the backend

The extension never reaches Supabase or GitHub directly. Everything goes through the backend over one axios instance with `baseURL = apiUrl`. A request interceptor injects the `Bearer` token; a response interceptor handles `401` with a single-flight refresh and one retry.

Authentication uses `POST /register`, `POST /login` (returns `access_token`, `refresh_token`, `user_id`), and `POST /refresh`. `AuthManager.refresh()` memoizes an `inFlightRefresh` promise so that when several requests `401` at once they share one `/refresh` call rather than each burning a refresh token, and an `_retried` flag on the original request stops infinite retry loops. Tokens are stored in `context.secrets` (VS Code SecretStorage); email and `userId` go in `globalState`.

The endpoints the session uses:

| Method | Path | Purpose |
|---|---|---|
| `PUT` | `/api/overlay/{projectId}/{userId}/{file}?branch={branch}` | register/refresh this file's overlay |
| `GET` | `/api/overlay/ws/{projectId}/{userId}/{file}?token={jwt}` | per-file overlay WebSocket |
| `GET` | `/api/projects/{id}/file?branch=main&path=...` | base content for live synthesis |
| `GET` | `/api/merge/{projectId}/{file}?user_id={userId}` | authoritative conflict poll (errors return `[]`) |
| `DELETE` | `/api/overlay/me/{projectId}` | Notbremse; returns `{ reset: n }` |
| `GET` | `/api/projects/{id}/members` | resolve `user_id` to display name for labels |

Best-effort getters such as `getMergeConflicts` and `listProjectMembers` swallow errors and return `[]`, because a missing conflict poll or member list should degrade the UI rather than break the session.

## Local development

```bash
npm ci
npm run compile      # tsc -p ./
npm run watch        # tsc -watch -p ./ during development
npm run lint         # eslint src
```

Press F5 in VS Code to launch an Extension Development Host. Point `lightningGit.apiUrl` and `lightningGit.wsUrl` at a running [backend](../backend) (defaults assume `localhost:8080`). You will need an account, register from the command palette, and the workspace must be a git checkout whose `origin` matches a project that already exists in the web frontend.

Runtime dependencies are `axios`, `ws`, and `diff` (plus its types). The extension targets VS Code `^1.74.0` and compiles to `./out/extension.js`.

## Project layout

```
src/
  extension.ts        activation, command registration, project auto-detect, Notbremse item
  overlaySession.ts   the session: WS lifecycle, decorations, status bars, recompute/render
  liveConflicts.ts    client-side port of the backend merge algorithm
  conflictsEqual.ts   conflict-set fingerprint for repaint suppression
  conflictPanel.ts    the conflict webview panel
  client.ts           axios instance, endpoints, 401 single-flight refresh + retry
  auth.ts             SecretStorage tokens, register/login/refresh
```

CI (`.github/workflows/ci.yml`) runs on push to `main`/`master` and on pull requests, on Node 18 and 20: checkout, `npm ci`, `npm run compile`, `npm run lint`.

## Scope and limitations

This extension is a diploma-thesis prototype, and a few things are deliberately out of scope.

There are no automated tests. There is no test framework, no test script, and no `*.test.ts` files; CI compiles and lints only. The conflict algorithm is the most exposed gap here, because it exists in three implementations, the Rust backend, the frontend's `utils/merge.ts`, and this extension's `liveConflicts.ts`, and only the Rust copy has tests. If the ports drift, nothing catches it.

The backend holds all overlay state in RAM on a single instance, so live state is lost on restart and the system does not scale past one backend process without shared state and cross-instance broadcast. Conflicts come from a 60-second poll rather than a push, which is why the client-side synthesis exists at all; moving that path fully to push is future work. The Notbremse is reactive by nature, as noted above.

For the why behind the design, and for the conflict-prediction algorithm in full, see the [backend](../backend) and [lightning-git.com](https://lightning-git.com).
