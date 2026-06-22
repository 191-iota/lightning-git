# Deployment

This is a polyglot monorepo. The three packages deploy independently, each from
its own subfolder, and nothing at the repo root needs to build for any single one
to ship. There is deliberately no root workspace or shared lockfile: each package
stays a self-contained, independently deployable unit, which keeps the deploy
configuration for each host trivial (point it at a subfolder) and avoids
dependency hoisting surprises for the VS Code extension.

```
lightning-git/
├── backend/     Rust / actix-web API + WebSocket hub (Docker)
├── frontend/    Vue 3 + Vite single-page app (static build)
└── extension/   VS Code extension (packaged with vsce)
```

The marketing page is not deployed from here. It is a separate repository,
[lightning-git-landing](https://github.com/191-iota/lightning-git-landing), and
answers on lightning-git.com on its own.

## backend/, Docker

The backend ships as a container. The `Dockerfile` lives in `backend/` and its
`COPY` paths are relative to that directory, so the only thing a host needs is the
right build context.

- Build context / root directory: `backend/`
- Dockerfile: `backend/Dockerfile`
- Exposes port `8080`; reads `HOST`, `PORT`, `GIT_REPO_DEV`, plus the Supabase and
  JWT settings from the environment (see `backend/.env.example`).

Build locally from the repo root:

```bash
docker build -t lightning-git-backend backend/
```

On a PaaS (Fly.io, Render, Railway): set the service's **root directory** to
`backend` and let it use the in-repo Dockerfile.

## frontend/, static SPA

A Vite build that produces a static bundle in `frontend/dist/`.

- Root / base directory: `frontend/`
- Install + build: `npm ci && npm run build`
- Publish directory: `frontend/dist`
- `VITE_API_BASE_URL` selects the backend at build time (see `frontend/.env.example`).

On Vercel / Netlify / Cloudflare Pages: set the project's **root directory** to
`frontend`, build command `npm run build`, output directory `dist`.

## extension/, VS Code Marketplace

Not a server deployment; packaged and published from its own folder.

```bash
cd extension
npm ci
npm run compile
npx vsce package      # -> a .vsix
npx vsce publish      # requires a publisher + PAT
```

## Migration note, repointing live deploys

These packages previously lived in standalone repositories, which are now
archived. Archiving makes a repo read-only; **a site that is already deployed
keeps serving its last build**, but any host integration that auto-deployed from
one of the old repos will no longer receive pushes. To resume continuous
deployment, point each host at this monorepo and set its root directory to the
matching subfolder (`backend/`, `frontend/`). Until that is done, the live site is
frozen at its last pre-migration build rather than broken.

## CI

`.github/workflows/ci.yml` runs only the jobs for packages a change actually
touches (path filters), so a backend-only change does not rebuild the frontend.
Each package is checked on its own terms: the backend builds and runs its test
suite, the frontend type-checks/builds/tests, and the extension compiles/lints/tests.
