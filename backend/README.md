# lightning-git-backend

In order for this repository to work, git needs to be installed on the machine.

# What is Lightning Git?

Lightning Git mirrors a Git repository without owning it with the use of webhooks. It adds collaborative features such as live coding views, merge detections, and instant comments directly in the code.

# Why?

Traditional tools like Jira create overhead. Developers and managers often communicate using separate systems that don't reflect the code in real time. Lightning Git gives teams transparency and instant feedback by bringing project management directly into the codebase.

---

# Conceptual Overview

Lightning Git is a **realtime visibility layer** built on top of Git. It allows teams to see code changes as they happen, track progress through branches, and manage projects without leaving the development environment.

## Core Concepts

### Projects

A **Project** in Lightning Git represents a mirrored Git repository. When you create a project, Lightning Git:
- Connects to your remote repository via webhooks
- Creates a local mirror that stays synchronized
- Sets up realtime channels for live updates

### Tasks from Branches

Lightning Git automatically creates **Tasks** based on branches in your repository. This means:
- Each feature branch becomes a trackable task
- Branch activity (commits, merges) updates task status automatically
- Teams can see who is working on what without manual status updates

### Realtime Overlay

The **Overlay** is a live view layer that displays uncommitted changes across the team. This is purely informational—it shows what others are working on without interfering with their work. There is no cross-overwriting; each developer maintains full control of their own changes.

### Git Mirror

The backend maintains a local clone of the remote repository. This mirror:
- Stays in sync through webhook events
- Serves as the data source for overlay computations
- Fetches updates to reflect the latest repository state

## How It Works

1. **Create a Project**: Connect a Git repository to Lightning Git
2. **Automatic Task Creation**: Branches become tasks, commits update progress
3. **Realtime Visibility**: Team members see live updates in their IDE or dashboard
4. **Stay Informed**: View what others are working on without any interference

---
