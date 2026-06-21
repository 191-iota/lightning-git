# Lightning Git

![CI](https://img.shields.io/badge/CI-path--filtered-1a1a1a?style=flat-square) ![tests](https://img.shields.io/badge/tests-passing-2d6a4f?style=flat-square) ![self-hosted](https://img.shields.io/badge/self--hosted-9b2c2c?style=flat-square)

Git only shows you a teammate's work once they commit and push. Everything before that, the file they have open right now, the function they are halfway through rewriting, the line two of you are both editing on different branches, is invisible until it lands as a conflict in a pull request. Lightning Git closes that gap. It mirrors a repository read-only, holds each person's uncommitted edits as live overlay state in the backend's RAM, and streams that state to the rest of the team, so you can see who is editing which file, where two branches are diverging, and which merge conflict is forming, while everyone is still typing and before any commit exists.

Nobody changes how they work. You keep committing, branching, and merging in your own Git exactly as before. Lightning Git never writes to your repository; it only watches and reports.

<p align="center">
  <img src="backend/assets/system.png" alt="One backend, two surfaces: a VS Code extension and a web dashboard over the Rust backend, with a read-only git mirror and a Supabase metadata store" width="900">
</p>

## What is in this repository

This used to be four separate repositories. They are now one, each kept as a self-contained subfolder with its own toolchain, build, and full history.

- [`backend/`](backend) (Rust, actix-web) is the engine. It owns the read-only mirror clones, the in-RAM overlay state, the per-file WebSocket layer, and the conflict prediction. It is the only place the conflict algorithm runs.
- [`frontend/`](frontend) (Vue 3, TypeScript, Vite, Pinia) is the web dashboard. It renders the same realtime overlay the editor shows, so a non-coding stakeholder like a Scrum Master or Product Owner can watch the live state of the code in a browser without cloning anything.
- [`extension/`](extension) (VS Code, TypeScript) is the developer's surface. It shows teammates' edits and the predicted conflicts inside the editor where the work actually happens.
- [`landing/`](landing) is the static marketing page, a single self-contained `index.html`.

The backend computes conflicts and pushes them over the WebSocket; the frontend and the extension render what the backend sends rather than each carrying their own copy of the algorithm, so the two clients can never drift from the server or from each other.

## Why one flat repository

Each package keeps its own lockfile and builds on its own. There is no root workspace and no shared `node_modules`. A deploy host points at a single subfolder and never has to understand the rest of the repo, and the VS Code extension is packaged without the dependency-hoisting surprises a shared install would cause. The cost is that the two Node packages install separately; for four pieces that ship to four different places, that is the cheaper trade.

## Running it locally

Each package has its own README with the real detail. In short:

```bash
# backend: needs Rust + git on the host and a Supabase project (see backend/.env.example)
cd backend && cargo run

# frontend: needs Node; VITE_API_BASE_URL points at the backend
cd frontend && npm ci && npm run dev

# extension: open extension/ in VS Code and press F5 to launch an Extension Host

# landing: just open landing/index.html, or `npx serve landing`
```

## Deployment

Every package deploys from its own subfolder. [DEPLOYMENT.md](DEPLOYMENT.md) has the per-package host settings: the Docker build context for the backend, the base directory and output for the frontend and the landing page, and `vsce` for the extension.

There is no public instance you can sign into. [lightning-git.com](https://lightning-git.com) serves the `landing/` page only, not a running product. To use Lightning Git you host it yourself from these subfolders.

## CI

[`.github/workflows/ci.yml`](.github/workflows/ci.yml) checks only the packages a change touches. The backend builds and runs its tests, the frontend type-checks and builds and tests, the extension compiles and lints and tests, and the landing page is validated as HTML.

## Authorship

This repository carries its complete development history, every commit from all four original packages, which is itself the record of how it was built. Releases are published as signed tags; see [AUTHORSHIP.md](AUTHORSHIP.md) for how to verify them and what the signature proves. Licensed under [MIT](backend/LICENSE), Copyright (c) 2026 191-iota.
