# lightning-git-frontend

![Vue 3](https://img.shields.io/badge/Vue%203-1a1a1a?style=flat-square&logo=vuedotjs&logoColor=white) ![TypeScript](https://img.shields.io/badge/TypeScript-1a1a1a?style=flat-square&logo=typescript&logoColor=white) ![Vite](https://img.shields.io/badge/Vite-1a1a1a?style=flat-square&logo=vite&logoColor=white) ![live](https://img.shields.io/badge/live-lightning--git.com-9b2c2c?style=flat-square)

The Vue web surface for Lightning Git: a dashboard that renders the same realtime overlay data as the editor extension, plus the organization and project management screens and the public landing and pricing pages.

Lightning Git is a realtime visibility layer on top of Git. Between the moment a developer writes code and the moment they commit, nobody else can see the work in progress — Git deals in commits, not in the stream of edits between them. Lightning Git mirrors a repository read-only, holds each person's in-flight edits as ephemeral overlay state in the backend's RAM, and streams that state to the rest of the team. It is a Swiss HF diploma-thesis prototype. Live at [lightning-git.com](https://lightning-git.com).

The product is three repositories against one backend:

- [lightning-git-backend](https://github.com/191-iota/lightning-git-backend) (Rust / actix-web) — owns the read-only repo clones, the in-RAM overlay state, the WebSocket realtime layer, and merge-conflict prediction.
- `lightning-git-frontend` (this repo, Vue 3 + TypeScript + Vite + Pinia) — the web dashboard plus org/project management and the public marketing pages.
- [lightning-git-vsc](https://github.com/191-iota/lightning-git-vsc) (VS Code extension, TypeScript) — the developer surface inside the editor.

<p align="center">
  <img src="assets/system.png" alt="One backend, two surfaces: this web dashboard and the VS Code extension over the Rust backend, with a read-only git mirror and a Supabase metadata store" width="900">
</p>

## Why a web surface exists alongside the editor extension

The extension lives where developers work, so a developer already sees teammates' edits and predicted conflicts in their own editor. But not everyone on a team opens VS Code. A Scrum Master, a Product Owner, or anyone coordinating the work needs the same live picture: who is editing which file right now, where two branches are diverging, what comments sit on which line, all without installing an editor or touching a repository. The web app renders exactly that. It reads the same activity and overlay streams the extension reads and projects them into a browser dashboard, so a non-coding stakeholder watches the live state of the code without ever cloning it.

Projects, organizations, and members are created and managed here, not in the extension; the extension only links to a project that already exists by matching the git remote. So this repo is both the read-only window for stakeholders and the management console for the whole product, and it carries the public landing and pricing pages on top.

## How the realtime layer is wired

There are two kinds of socket. One is project-wide and shows the activity feed (who is editing what across the whole project). The other is per-file and carries the detailed overlay of a single file. The frontend treats them differently on purpose, because they have different lifetimes.

### One shared activity socket per project

<p align="center">
  <img src="assets/activity-socket.png" alt="One project-wide socket lives in the store and survives navigation; a generation counter stops a stale socket from reconnecting over the live one" width="900">
</p>

The project-wide activity socket is owned by a Pinia store (`stores/activity.ts`), not by a view. `ensure(projectId)` opens it only if the project changed or the socket is dead, and the store is reused across navigation between the Kanban board (`ProjectView`) and the live file view (`OverlayView`). `OverlayView`'s `onUnmounted` disposes its own per-file socket but deliberately does not dispose the activity store, because tearing the activity socket down and rebuilding it on every route change would make it thrash for no reason.

Keeping one long-lived socket across navigation creates the classic WebSocket race: a previous socket's delayed `onclose` can fire after a new socket has already opened and schedule a reconnect on top of it. The store guards against that with a monotonic `generation` counter. `open()` captures `const myGen = ++generation`, and every async handler starts with `if (myGen !== generation) return`, so a stale socket's `onopen`, `onmessage`, `onclose`, and `onerror` all short-circuit and cannot touch state or schedule a reconnect that races the live connection.

The store also drives reconnect from the browser's `online` and `offline` events, because a dropped network does not reliably fire `onclose` on an already-open socket. DevTools "Offline" in particular leaves the socket reporting `OPEN`, which would otherwise pin the status at `live` forever. `handleOffline` bumps the generation and closes the dead socket; `handleOnline` calls `open()` immediately rather than waiting for a timeout, and the backend pushes the current edits on the fresh connection.

### Per-file overlay sockets

Each opened file gets its own `OverlayWebSocket` (`services/ws.ts`). It auto-reconnects 3 seconds after an unexpected close, but never after the caller disposes it. The guard is a `disposed` flag set *before* `socket.close()` is called, so the async `onclose` reconnect path sees `disposed === true` and short-circuits. Without that ordering, `dispose()` would close the socket, the async `onclose` would then fire on an instance the view had already discarded, and an orphan socket would reconnect itself back to life.

The token rides in the query string (`?token=...&echo=true`) because browsers cannot set headers on a WebSocket handshake. `echo=true` is what makes the web viewer see its own edits reflected back; the VS Code side omits it so a typist there does not fight their own cursor. The backend reads that flag and filters overlay echoes per subscriber, while always delivering comment events so the originating client learns the server-assigned id. The message shape (`WsMessage`) is a tagged union mirroring the backend's `WsBroadcast`:

```ts
export type WsMessage =
  | { kind: "overlay"; user_id: string; content: string; line_section: [number, number] }
  | { kind: "comment_created"; id: string; user_id: string; line: number; text: string; created_at: number }
  | { kind: "comment_deleted"; id: string }
  | { kind: "snapshot"; comments: Comment[]; all_user_contents: OverlayUserView[] };
```

The `snapshot` message arrives on subscribe and seeds both the existing comments and the current teammate contents in one shot, so there is no HTTP follow-up to populate the view.

## Client-side overlay projection

<p align="center">
  <img src="assets/conflict-prediction.png" alt="The conflict algorithm this view ports from the backend: per-source diffs against main, clustered, judged by whether the sources disagree" width="860">
</p>

The live file view does not just paint the raw text of each teammate. It projects every contributor's content into a per-line view and computes a live conflict set, then merges that with the backend's authoritative answer.

`utils/overlay.ts` `computeProjectedLines` runs an LCS diff (`diffArrays` from the `diff` package) of each user's content against the base, so a single inserted line does not cascade-tag every line below it as that user's edit. Lines are tagged last-writer-wins but prefer the most recent non-empty contributor, because otherwise one teammate clearing their whole file would blank the projection for everyone. Pure deletions are surfaced as untagged base lines so the viewer still sees the file's structure rather than a hole.

`utils/merge.ts` is a direct port of the backend's conflict algorithm. `computeCombinedDiff` and `computeConflicts` decompose each source's line diff against `origin/main` into hunks, group overlapping base-line ranges, and drop any group that has fewer than two distinct sources or whose hunks all make the identical edit. `flattenConflicts(live, backend)` treats the backend's result as a superset and keeps a live-only conflict only at a range the backend's last poll did not cover. Because the live pass and the backend run the same algorithm, the two results line up at overlapping ranges, so the backend's slower answer slots in without flicker. The frontend gets sub-second feedback between polls without diverging from the server.

The conflict poll hits `/api/merge/{projectId}/{encodedFilePath}` every 60 seconds with an in-flight guard and a stale-file check, so a poll that returns after the user switched files is discarded. The base content for the diff is fetched from `origin/main` (`branch=main`), not the user's feature branch, so the client and backend diff against the same merge target. `mergeConflicts` is a `shallowRef` because the response is a wholesale-replaced JSON array, and deep-proxying every hunk on each 60 s tick caused page hitches. Rendering is capped at `MAX_RENDERED_LINES = 1500` to avoid freezing on huge files, while conflict detection still runs over the full content.

Per-user colors are assigned by position, not by hashing. `OverlayView` builds `knownUserIds` as a sorted array of every id seen in overlays, comments, and the current user, and `colorIndex(userId)` returns the array index modulo the palette length, with a char-hash fallback only for an id not yet in the set. Five users get five distinct colors, which a plain hash-mod could not guarantee on a small population, and the result is stable across reloads.

This algorithm lives in three places (the Rust backend, `utils/merge.ts` here, and `liveConflicts.ts` in the extension), and only the Rust variant has tests, so drift between the ports would not be caught automatically.

## Auth and the refresh interceptor

Auth uses JWT access and refresh tokens. The access token persists in `localStorage` under `token`, the refresh token under `refreshToken`, the user under `user`, and the selected org under `currentOrgId`. The request interceptor in `services/api.ts` attaches `Authorization: Bearer <token>` to every call.

Refresh is handled by inverting the dependency so the store and the axios instance do not import each other in a cycle. The api module exposes `onUnauthorized(handler)` and keeps a module-level `refreshHandler`; the auth store registers its `refresh` function through it at construction. On a 401, the response interceptor marks the failed config with `_retried`, skips the call if it is the `/refresh` call itself, awaits `refreshHandler()`, rewrites the `Authorization` header with the fresh token, and replays the request via `api.request`. If refresh returns `null` (no usable refresh token), the handler has already cleared auth and the interceptor redirects to /login with `window.location.replace('/login')` (unless the user is already there) so they are not stuck on a page they cannot load.

The refresh itself is single-flight. `auth.ts` `refresh()` guards with a module-scoped `inFlight` promise, so when a page fires many parallel requests that all 401 at once, they share one refresh call instead of each burning a refresh token.

## Routes

The router (`src/router/index.ts`) splits into public marketing pages, guest-only auth pages, and authenticated app pages scoped under an organization.

| path | name | view | meta | purpose |
|------|------|------|------|---------|
| `/` | landing | `LandingView.vue` | — | public marketing landing page |
| `/pricing` | pricing | `PricingView.vue` | — | public pricing + FAQ |
| `/login` | login | `LoginView.vue` | `requiresGuest` | sign in |
| `/register` | register | `RegisterView.vue` | `requiresGuest` | sign up |
| `/orgs` | orgs | `OrgListView.vue` | `requiresAuth` | list / select organizations |
| `/orgs/new` | orgs-new | `OrgCreateView.vue` | `requiresAuth` | create an organization |
| `/orgs/:id/members` | org-members | `OrgMembersView.vue` | `requiresAuth` | manage org members |
| `/dashboard` | dashboard | `DashboardView.vue` | `requiresAuth, requiresOrg` | projects board for the selected org |
| `/projects/new` | projects-new | `ProjectCreateView.vue` | `requiresAuth, requiresOrg` | create a project |
| `/projects/:id` | project | `ProjectView.vue` | `requiresAuth, requiresOrg` | Kanban board for the project |
| `/projects/:id/overlay` | overlay | `OverlayView.vue` | `requiresAuth, requiresOrg` | live file overlay view |
| `/projects/:id/members` | project-members | `ProjectMembersView.vue` | `requiresAuth, requiresOrg` | manage project members |

The guard (`router.beforeEach`) is three rules: a `requiresAuth` route redirects to `login` when the user is not authenticated, a `requiresGuest` route redirects to `dashboard` when they already are, and a `requiresOrg` route redirects to `/orgs` when no org is selected.

## Pinia stores

State lives in five stores under `src/stores/`:

- `auth` — user, token, and refreshToken hydrated from `localStorage`; `isAuthenticated`; `login` / `register` / `logout`; the single-flight `refresh` registered into the api layer via `onUnauthorized`.
- `org` — the org list and `currentOrgId` (persisted under `currentOrgId`); fetch / create / rename / `transferOwnership` / remove / select / clear; member management; `roleIn(orgId)` returning `'owner' | 'member' | null`.
- `project` — projects, current project, tasks, and members; fetch and CRUD; `wipeMyOverlays` (the Notbremse); `setTaskArchived` / `setTaskColumn`; `myRole` returning `'admin' | 'member' | null`.
- `activity` — the shared project-wide WebSocket described above, exposing `edits`, a `state` of `'closed' | 'connecting' | 'live'`, `ensure(projectId)`, and `dispose()`.
- `toast` — the toast list with `push` / `dismiss` plus `info` / `success` / `error` helpers, default TTL 4500 ms.

## The board and the Notbremse

The Kanban board (`ProjectView`) tracks branches as tasks across four columns (`todo`, `in_progress`, `review`, `merged`), dragged with `vuedraggable`. Remote branches register as tasks automatically; columns are moved by hand. Archiving toggles via `PATCH /api/tasks/{id}/archive` and a column move via `PATCH /api/tasks/{id}/column`.

The Notbremse (kept as the German coinage for "emergency brake") resets the calling user's in-flight overlays in a project back to the committed git base, via `DELETE /api/overlay/me/{projectId}`. It affects only the caller's own overlays and is gated server-side by project membership. Its reason for existing is credential safety: if a secret lands in a live edit, one click discards that user's uncommitted typing across every file at once. It is a reactive control — it only helps if the user notices, and it cannot recall edits that already streamed out to teammates before the wipe.

## Local development

```bash
npm install
npm run dev          # vite dev server on :5173
npm run build        # vue-tsc -b && vite build
npm run preview      # serve the production build
npm run test         # vitest run
npm run test:watch   # vitest in watch mode
```

The backend's CORS in the prototype hard-wires `http://localhost:5173`, so the dev server is expected on that port. Two environment variables point the app at the backend; both have localhost defaults and live in `.env.example`:

| var | default | purpose |
|-----|---------|---------|
| `VITE_API_BASE_URL` | `http://localhost:8080` | REST base for the axios client |
| `VITE_WS_URL` | `ws://localhost:8080` | base for both the activity and per-file overlay sockets |

Run the [lightning-git-backend](https://github.com/191-iota/lightning-git-backend) locally first; it exposes the `/api` routes and the two WebSocket endpoints this app connects to.

## Testing

Tests run under Vitest with `@vue/test-utils` and `happy-dom` as the environment. There are 22 tests across two spec files: `src/utils/overlay.spec.ts` (15 — 6 for `buildTree`, 9 for `computeProjectedLines`) and `src/components/FileTreeNode.spec.ts` (7). Twenty-one pass and one fails on purpose.

The failing test is `computeProjectedLines > 'only tags the deleted region, not the lines that shifted up'`. It expects `['a','b','d','e']` but the implementation keeps the deleted line `c`, returning `['a','b','c','d','e']`, because `computeProjectedLines` surfaces pure deletions as untagged base lines so a stakeholder still sees the file's structure rather than a gap. The test is left red to mark that the deletion-projection behaviour is an open design question, not settled.

## Project layout

```
src/
  views/        route components incl. OverlayView (live file), ProjectView (Kanban),
                Landing/Pricing marketing pages, org & project management screens
  stores/       five Pinia stores: auth, org, project, activity, toast
  services/     api.ts (axios + refresh interceptor), ws.ts (per-file OverlayWebSocket)
  utils/        overlay.ts (projection, LCS diff), merge.ts (conflict algorithm port)
  components/   NavBar, FileTreeNode, dialogs, ToastHost, TabStrip, icons
  router/       route table + the auth/guest/org guard
  types/        shared API types
```

## Scope and limitations

This is a diploma-thesis prototype built under a fixed time budget, not production software, and a few things are deferred on purpose.

The conflict algorithm is ported by hand into `utils/merge.ts` from the Rust backend, and only the Rust copy is tested, so a divergence between the two would not be caught. Conflicts and comments use a 60-second poll for the authoritative answer while live overlays already arrive over WebSocket; moving both fully to push would cut latency. Comments in the backend are in-memory only and are lost on restart. The backend keeps overlay state in a single in-memory instance with an in-process broadcast, so running more than one backend would need shared state. The backend's CORS is hard-wired to the localhost dev origin and would need to be environment-driven for a real deployment, and the WebSocket connect path on the backend checks a valid JWT but does not verify project membership on the socket itself.

The pricing tiers on `PricingView` (Free CHF 0 for public repos and up to 3 projects, Team CHF 4 with private repos and unlimited projects, Organization CHF 8 with a self-hosted option) are forward-looking product framing; the page states plainly that Lightning Git is a research prototype developed as a Swiss HF diploma project.

---

Repositories live under [github.com/191-iota](https://github.com/191-iota): `lightning-git-backend`, `lightning-git-frontend`, `lightning-git-vsc`.
