# hf-lightning-git-frontend

The Vue 3 + TypeScript web surface for Lightning Git. Renders the Kanban,
the live OverlayView, organisation and project management, and the public
landing + pricing pages.

## What is Lightning Git?

A realtime visibility layer for Git. Two surfaces: this web app for the
Scrum Master and other non-coding stakeholders, and a VSCode extension for
developers. Both talk to the same Rust backend.

- "hf-lightning-git-backend" — Rust + Actix-Web
- "hf-lightning-git-frontend" (this one) — Vue 3 + TypeScript + Vite + Pinia
- "hf-lightning-git-vsc" — VSCode extension

## Stack

- **Vue 3** with "<script setup>" Composition API
- **Vite** for dev server and build
- **Pinia** stores for auth, org, project, activity (shared WS), toast
- **Vue Router** with "requiresAuth" / "requiresOrg" route meta guards
- **Tailwind CSS** with a small set of project-specific tokens ("lg-bg",
  "lg-surface", "lg-accent", etc.) and reusable component classes
  ("lg-btn-primary", "lg-card", "lg-input", "lg-scope", etc.) declared in
  "src/style.css"
- **Space Grotesk** loaded from Google Fonts

## Views

| Route                   | View               | Purpose                                        |
| ----------------------- | ------------------ | ---------------------------------------------- |
| "/"                     | LandingView        | Public landing with hero, features, principles |
| "/pricing"              | PricingView        | Static pricing tiers, SLA, FAQ                 |
| "/login", "/register"   | auth views         | Supabase auth via the backend                  |
| "/orgs"                 | OrgListView        | Organisation picker                            |
| "/orgs/new"             | OrgCreateView      | Create an org                                  |
| "/orgs/:id/members"     | OrgMembersView     | Org-level membership (owner-gated)             |
| "/dashboard"            | DashboardView      | Projects in the currently selected org         |
| "/projects/new"         | ProjectCreateView  | Connect a GitHub repository                    |
| "/projects/:id"         | ProjectView        | Four-column Kanban with drag, archive          |
| "/projects/:id/members" | ProjectMembersView | Project-level membership                       |
| "/projects/:id/overlay" | OverlayView        | Live code view with per-line comments          |

## State

- **"stores/auth.ts"** — JWT + refresh, exposes "isAuthenticated". Registers a
  refresh handler with "services/api.ts" so 401s trigger a single in-flight
  token refresh.
- **"stores/org.ts"** — list of orgs the user belongs to, currently selected
  org id in localStorage, member CRUD.
- **"stores/project.ts"** — list of projects in the current org, current
  project, members, tasks, archive toggling.
- **"stores/activity.ts"** — project-wide activity WebSocket. Single shared
  socket across ProjectView ↔ OverlayView so navigation does not thrash the
  connection.
- **"stores/toast.ts"** — top-right toast queue with auto-dismiss.

## Local development

```bash
# Backend must be running (see hf-lightning-git-backend)
cp .env.example .env  # set VITE_API_BASE_URL, VITE_WS_URL
npm install
npm run dev   # serves on http://localhost:5173
```

"npm run build" produces a static bundle in "dist/". The bundle is served
either by the Railway container alongside the backend or via any static host.
