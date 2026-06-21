# Lightning Git

![CI](https://img.shields.io/badge/CI-path--filtered-1a1a1a?style=flat-square) ![live](https://img.shields.io/badge/live-lightning--git.com-9b2c2c?style=flat-square)

Lightning Git is a realtime visibility layer on top of Git. Between the moment a developer writes code and the moment they commit, nobody else can see the work in progress — Git deals in commits, not in the stream of edits between them. Lightning Git mirrors a repository read-only, holds each person's in-flight edits as ephemeral overlay state in the backend's RAM, and streams that state to the rest of the team, so teammates see who is editing which file, where two branches are diverging, and the conflicts that divergence will cause before anyone opens a pull request. It is an early-stage, self-hostable project. Live at [lightning-git.com](https://lightning-git.com).

This repository holds the whole product. It used to be four separate repositories; they are now subfolders here, each still an independent, independently deployable unit.

- [`backend/`](backend) — Rust / actix-web. Owns the read-only repo clones, the in-RAM overlay state, the WebSocket realtime layer, and merge-conflict prediction.
- [`frontend/`](frontend) — Vue 3 + TypeScript + Vite + Pinia. The web dashboard that renders the same realtime overlay data as the editor extension, plus org and project management.
- [`extension/`](extension) — the VS Code extension, where a developer sees teammates' edits and predicted conflicts inside their own editor.
- [`landing/`](landing) — the standalone marketing page served at lightning-git.com, a single self-contained `index.html`.

The backend is the one server; the frontend, the extension, and the landing page all sit in front of it. The conflict-prediction algorithm runs only in the backend, which recomputes the set on every overlay edit and pushes it over the per-file WebSocket; the frontend and the extension render what the backend sends rather than each carrying their own copy of the algorithm.

## Layout and why it is flat

Each package keeps its own toolchain, its own lockfile, and its own build. There is no root workspace and no shared `node_modules`. A deploy host points at one subfolder and never has to understand the rest of the repo, and the VS Code extension is packaged without the dependency-hoisting quirks a workspace would introduce. The cost is that the two Node packages install separately; for four packages that deploy to four different places, that is the cheaper side of the trade.

## Running it locally

Each package has its own README with the details. In short:

```bash
# backend (needs Rust + git, plus Supabase/JWT env — see backend/.env.example)
cd backend && cargo run

# frontend (needs Node; VITE_API_BASE_URL points at the backend)
cd frontend && npm ci && npm run dev

# extension: open extension/ in VS Code and press F5 to launch an Extension Host

# landing: open landing/index.html, or `npx serve landing`
```

## Deployment

Every package deploys from its own subfolder. [DEPLOYMENT.md](DEPLOYMENT.md) has the per-package host settings (Docker context for the backend, base directory and output for the frontend and landing, vsce for the extension), and the note about repointing the live deploys after the four standalone repos were archived.

## CI

[`.github/workflows/ci.yml`](.github/workflows/ci.yml) checks only the packages a change touches. The backend builds and runs its tests, the frontend type-checks and builds and tests, the extension compiles and lints and tests, and the landing page is validated as HTML.
