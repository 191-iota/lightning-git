<script setup lang="ts">
import { RouterLink } from "vue-router";
import { useAuthStore } from "@/stores/auth";
import BoltIcon from "@/components/BoltIcon.vue";
import NavBar from "@/components/NavBar.vue";

const auth = useAuthStore();

interface Feature {
  title: string;
  body: string;
}

const features: Feature[] = [
  {
    title: "Live code overlays",
    body: "See teammates' uncommitted edits per file, colour-coded by author, as they type. No more 'who is touching this?' in Slack.",
  },
  {
    title: "Pre-merge conflict detection",
    body: "Conflicts surface in the editor the moment two branches diverge on the same lines, not at merge time. Fix them while the context is still warm.",
  },
  {
    title: "Branches as tasks",
    body: "Your Kanban is derived from your branches. Active sessions light up the board automatically. No ticket double-entry, no stale status fields.",
  },
];

const principles = [
  { label: "Read-only", body: "Lightning Git never writes to your repository. Git history stays untouched." },
  { label: "In-memory", body: "Live edits live in server RAM and are gone on restart. Nothing about your in-flight work is persisted." },
  { label: "Project-isolated", body: "Every WS message is membership-checked. A user in project A cannot observe project B." },
];
</script>

<template>
  <div class="min-h-screen bg-lg-bg text-lg-text">
    <NavBar :brand-to="auth.isAuthenticated ? '/dashboard' : '/'">
      <RouterLink to="/pricing" class="lg-link">Pricing</RouterLink>
      <template v-if="auth.isAuthenticated">
        <RouterLink to="/dashboard" class="lg-link">Dashboard</RouterLink>
      </template>
      <template v-else>
        <RouterLink to="/login" class="lg-link">Log in</RouterLink>
        <RouterLink to="/register" class="lg-btn-primary">Sign up</RouterLink>
      </template>
    </NavBar>

    <section class="lg-container pt-24 pb-16 text-center relative">
      <div
        class="inline-flex items-center justify-center w-16 h-16 rounded-2xl mb-8"
        style="background: rgba(139,92,246,0.1); border: 1px solid rgba(139,92,246,0.3)"
      >
        <BoltIcon class="text-lg-accent-bright" :size="34" />
      </div>
      <h1 class="text-5xl md:text-6xl font-semibold leading-[1.05] tracking-[-0.02em]">
        See your team's code.
        <br />
        <span class="text-lg-accent-bright">In real time.</span>
      </h1>
      <p class="text-lg-text-sec text-base md:text-lg max-w-[600px] mx-auto mt-6 leading-relaxed">
        A realtime visibility layer for Git. Watch teammates edit, catch merge conflicts
        before they happen, and drive the Kanban from your branches.
      </p>
      <div class="flex flex-wrap gap-3 justify-center mt-10">
        <RouterLink :to="auth.isAuthenticated ? '/dashboard' : '/register'" class="lg-btn-primary">
          {{ auth.isAuthenticated ? 'Go to dashboard' : 'Get started' }}
        </RouterLink>
        <RouterLink to="/pricing" class="lg-btn-secondary">View pricing</RouterLink>
      </div>
    </section>

    <section class="border-t border-b border-lg-border bg-lg-surface">
      <div class="lg-container py-20">
        <h2 class="text-2xl md:text-3xl font-semibold text-center mb-3">What you get</h2>
        <p class="text-center text-lg-text-sec text-sm md:text-[0.975rem] mb-12 max-w-[540px] mx-auto">
          Three primitives. Each one solves a problem that Slack and Jira do not.
        </p>
        <div class="grid grid-cols-1 md:grid-cols-3 gap-5">
          <article
            v-for="f in features"
            :key="f.title"
            class="rounded-xl border border-lg-border bg-lg-bg p-6"
          >
            <h3 class="font-semibold text-lg-text">{{ f.title }}</h3>
            <p class="text-sm text-lg-text-sec mt-3 leading-relaxed">{{ f.body }}</p>
          </article>
        </div>
      </div>
    </section>

    <section class="lg-container py-20">
      <h2 class="text-2xl md:text-3xl font-semibold text-center mb-3">Principles</h2>
      <p class="text-center text-lg-text-sec text-sm md:text-[0.975rem] mb-12 max-w-[560px] mx-auto">
        The product follows three constraints and they are not negotiable.
      </p>
      <div class="grid grid-cols-1 md:grid-cols-3 gap-5">
        <div
          v-for="p in principles"
          :key="p.label"
          class="rounded-xl border border-lg-border bg-lg-surface p-6"
        >
          <p class="text-[0.7rem] uppercase tracking-[0.18em] font-medium text-lg-accent-bright mb-3">
            {{ p.label }}
          </p>
          <p class="text-sm text-lg-text-sec leading-relaxed">{{ p.body }}</p>
        </div>
      </div>
    </section>

    <section class="border-t border-lg-border bg-lg-surface">
      <div class="lg-container py-16 text-center">
        <h2 class="text-2xl md:text-3xl font-semibold mb-4">Ready to ship together?</h2>
        <p class="text-lg-text-sec text-sm md:text-base mb-8 max-w-[480px] mx-auto leading-relaxed">
          Connect a GitHub repo in under a minute. Free for solo devs and open source.
        </p>
        <div class="flex flex-wrap gap-3 justify-center">
          <RouterLink :to="auth.isAuthenticated ? '/dashboard' : '/register'" class="lg-btn-primary">
            {{ auth.isAuthenticated ? 'Open dashboard' : 'Create your account' }}
          </RouterLink>
          <RouterLink to="/pricing" class="lg-btn-secondary">See plans</RouterLink>
        </div>
      </div>
    </section>

    <footer class="border-t border-lg-border">
      <div
        class="lg-container py-10 flex flex-col md:flex-row gap-3 justify-between text-xs text-lg-text-muted"
      >
        <p>Lightning Git is a research prototype developed as a Swiss HF diploma project.</p>
        <p>Built on Rust, Vue, and Supabase.</p>
        <p>
          Contact:
          <a href="mailto:decayorbit@proton.me" class="lg-link">decayorbit@proton.me</a>
        </p>
      </div>
    </footer>
  </div>
</template>
