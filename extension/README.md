# hf-lightning-git-vsc

The VSCode extension for Lightning Git. Streams a developer's edits to the
backend, peeks at teammates' in-flight changes inline, surfaces predicted
merge conflicts, renders line comments, and exposes the Notbremse.

## What is Lightning Git?

A realtime visibility layer for Git. Three repos:

- "hf-lightning-git-backend" — Rust + Actix-Web backend
- "hf-lightning-git-frontend" — Vue 3 web app for non-coding stakeholders
- "hf-lightning-git-vsc" (this one) — VSCode extension for developers

## Commands

| Command | What it does |
|---|---|
| Lightning Git: Login | Email + password against the Supabase-backed API |
| Lightning Git: Logout | Clear stored credentials |
| Lightning Git: Register | Create a new account |
| Lightning Git: Create Project | Picks an org, posts to the backend |
| Lightning Git: Start Session | Resolves the workspace repo URL to a project, opens the per-file overlay WS for the active editor |
| Lightning Git: Stop Session | Closes all open WS connections |
| Lightning Git: Peek Teammate Changes | Inline decorations of other developers' content on the current file |
| Lightning Git: Peek Merge Conflicts | Inline decorations on lines predicted to conflict on merge |
| Lightning Git: View Teammate Change | Diff your file against a chosen teammate's version |
| Lightning Git: Comment on Current Line | Add a comment scoped to the line under the cursor |
| Lightning Git: Notbremse | Reset your server-side overlay on every file back to the committed branch state |

## Status bar

- **Notbremse** (always visible, top-left): emergency reset of your live
  state. Survives session start and stop, never goes away.
- **Teammates** (during a session): number of teammates editing the current
  file. Click to peek.
- **Conflicts** (during a session): number of predicted conflicts on the
  current file. Click to peek.
- Activity flash on the teammate item when a new edit comes in.

## Session lifecycle

"OverlaySession" opens when the user runs Start Session. On every active
editor change it:

1. Reads the current branch via "git rev-parse --abbrev-ref HEAD".
2. Calls "PUT /api/overlay/..." on the backend to register or refresh the
   overlay for the file.
3. Connects to "/api/overlay/ws/..." for that file.
4. Polls "/api/merge/..." every 30 seconds for conflicts.
5. Polls "/api/comments/..." every 5 seconds for new comments.
6. Watches "onDidChangeTextDocument" and debounces outbound updates by
   "lightningGit.debounceMs" (default 250ms).

The debounce default of 250ms is tuned for sub-second perceived liveness
while still coalescing a fast burst of keystrokes into a single broadcast.
If you want a wider window to clear accidentally typed credentials with the
Notbremse before they sync, raise this value in your settings.

## Settings

```jsonc
{
  "lightningGit.apiUrl": "http://localhost:8080",
  "lightningGit.wsUrl": "ws://localhost:8080",
  "lightningGit.debounceMs": 250
}
```

## Local development

```bash
npm install
npm run compile
# F5 in VSCode to launch an Extension Development Host
```

The Extension Development Host opens with the extension loaded. Open a Git
workspace, run "Lightning Git: Login", then "Lightning Git: Start Session".
The status bar items appear; conflicts and teammates light up as soon as
another developer edits the same file.
