<script setup lang="ts">
import { RouterLink } from "vue-router";
import { useAuthStore } from "@/stores/auth";
import BoltIcon from "@/components/BoltIcon.vue";
import NavBar from "@/components/NavBar.vue";

const auth = useAuthStore();

interface Plan {
  name: string;
  price: string;
  cadence: string;
  blurb: string;
  features: string[];
  cta: { label: string; to: string };
  highlight?: boolean;
}

const plans: Plan[] = [
  {
    name: "Free",
    price: "CHF 0",
    cadence: "per user / month",
    blurb: "For solo developers and open-source projects.",
    features: [
      "Public GitHub repos only",
      "Up to 3 projects",
      "Live overlay + conflict detection",
      "Kanban board with branch sync",
      "Community support",
    ],
    cta: { label: "Get started", to: "/register" },
  },
  {
    name: "Team",
    price: "CHF 4",
    cadence: "per user / month",
    blurb: "For small teams shipping together every day.",
    features: [
      "Public and private repos",
      "Unlimited projects per organization",
      "Live overlay + conflict detection",
      "Inline comments synced across IDE and web",
      "Email support (Mon to Fri, 08 to 18 CET)",
      "99.5% monthly uptime SLA",
    ],
    cta: { label: "Start a Team plan", to: "/register" },
    highlight: true,
  },
  {
    name: "Organization",
    price: "CHF 8",
    cadence: "per user / month",
    blurb: "For organizations that need extended support and self-hosting.",
    features: [
      "Everything in Team",
      "Extended support window (Mon to Sat, 06 to 22 CET)",
      "Priority incident response (1h critical)",
      "Self-hosted deployment option",
      "Dedicated onboarding session",
    ],
    cta: { label: "Talk to sales", to: "/register" },
  },
];

const faqs = [
  {
    q: "Does Lightning Git store my source code?",
    a: "A read-only mirror of your GitHub repository lives on the backend container so we can compute conflicts and serve the live view. The mirror is ephemeral, re-cloned on every deploy, and never written to. Your in-flight edits stay in server memory only and are dropped on restart or when you trigger the Notbremse. GitHub remains your source of truth; nothing persists in our database besides project metadata.",
  },
  {
    q: "Do I need to install anything?",
    a: "Only the VSCode extension. The web view runs in the browser and works for non-coding team members like project leads.",
  },
  {
    q: "Can I cancel any time?",
    a: "Yes. Billing is monthly. Your Git history stays in GitHub, so leaving Lightning Git does not put data at risk.",
  },
  {
    q: "What is included in the SLA?",
    a: "99.5% monthly uptime on the API, WebSocket and web frontend. Credits up to 50% of the monthly fee apply if the target is missed.",
  },
];
</script>

<template>
  <div class="min-h-screen bg-lg-bg text-lg-text">
    <NavBar :brand-to="auth.isAuthenticated ? '/dashboard' : '/'">
      <RouterLink to="/pricing" class="text-lg-accent-bright">Pricing</RouterLink>
      <template v-if="auth.isAuthenticated">
        <RouterLink to="/dashboard" class="lg-link">Dashboard</RouterLink>
      </template>
      <template v-else>
        <RouterLink to="/login" class="lg-link">Log in</RouterLink>
        <RouterLink to="/register" class="lg-btn-primary">Sign up</RouterLink>
      </template>
    </NavBar>

    <section class="lg-container pt-24 pb-12 text-center">
      <div
        class="inline-flex items-center justify-center w-20 h-20 rounded-3xl mb-8 bg-lg-surface border border-lg-border"
      >
        <BoltIcon class="text-lg-text-sec" :size="40" />
      </div>
      <h1 class="text-5xl md:text-6xl font-semibold leading-tight">
        Pricing
      </h1>
      <p class="text-lg-text-sec max-w-[540px] mx-auto mt-5 text-base">
        Pick a plan that fits how your team ships.
      </p>
    </section>

    <section class="lg-container pb-20 grid grid-cols-1 md:grid-cols-3 gap-5">
      <article
        v-for="plan in plans"
        :key="plan.name"
        :class="[
          'rounded-lg border p-6 flex flex-col transition-all duration-300',
          plan.highlight
            ? 'border-lg-ink bg-lg-bg shadow-[0_4px_24px_rgba(0,0,0,0.08)]'
            : 'border-lg-border bg-lg-bg hover:shadow-[0_2px_12px_rgba(0,0,0,0.04)]',
        ]"
      >
        <header class="mb-5">
          <p
            v-if="plan.highlight"
            class="inline-block uppercase tracking-wider text-[0.7rem] font-semibold text-lg-accent-bright mb-3"
          >Most popular</p>
          <h2 class="text-lg font-semibold">{{ plan.name }}</h2>
          <p class="mt-3">
            <span class="text-4xl font-bold">{{ plan.price }}</span>
            <span class="text-lg-text-muted text-sm ml-1">{{ plan.cadence }}</span>
          </p>
          <p class="text-sm text-lg-text-sec mt-3 leading-relaxed">{{ plan.blurb }}</p>
        </header>
        <ul class="space-y-2.5 text-sm text-lg-text-sec flex-1">
          <li v-for="f in plan.features" :key="f" class="flex items-start gap-2.5">
            <span class="text-lg-accent-bright flex-shrink-0 mt-[0.4rem]">
              <span class="block h-1 w-1 rounded-full bg-current"></span>
            </span>
            <span class="leading-snug">{{ f }}</span>
          </li>
        </ul>
        <RouterLink
          :to="plan.cta.to"
          :class="plan.highlight ? 'lg-btn-primary mt-6' : 'lg-btn-secondary mt-6'"
        >{{ plan.cta.label }}</RouterLink>
      </article>
    </section>

    <section
      class="border-t border-b border-lg-border bg-lg-surface"
    >
      <div class="lg-container py-20">
        <h2 class="text-2xl md:text-3xl font-semibold text-center mb-3">
          Support & SLA
        </h2>
        <p class="text-center text-lg-text-sec text-[0.975rem] mb-12">
          Predictable response times that scale with your plan.
        </p>
        <div class="grid grid-cols-1 md:grid-cols-3 gap-5">
          <div class="rounded-xl border border-lg-border bg-lg-bg p-6">
            <h3 class="font-semibold text-lg-text mb-2">Support windows</h3>
            <p class="text-sm text-lg-text-sec leading-relaxed">
              Standard: Mon to Fri, 08 to 18 CET.<br />
              Extended: Mon to Sat, 06 to 22 CET.
            </p>
          </div>
          <div class="rounded-xl border border-lg-border bg-lg-bg p-6">
            <h3 class="font-semibold text-lg-text mb-2">Uptime target</h3>
            <p class="text-sm text-lg-text-sec leading-relaxed">
              99.5% monthly across the REST API, WebSocket gateway and web frontend.
            </p>
          </div>
          <div class="rounded-xl border border-lg-border bg-lg-bg p-6">
            <h3 class="font-semibold text-lg-text mb-2">Status & reports</h3>
            <p class="text-sm text-lg-text-sec leading-relaxed">
              Uptime monitored every 60 seconds. Monthly reports on request.
            </p>
          </div>
        </div>
      </div>
    </section>

    <section class="max-w-[800px] mx-auto px-5 py-20 lg-container">
      <h2 class="text-2xl md:text-3xl font-semibold text-center mb-12">
        Frequently asked questions
      </h2>
      <div class="space-y-4">
        <div
          v-for="item in faqs"
          :key="item.q"
          class="rounded-xl border border-lg-border bg-lg-surface p-5"
        >
          <h3 class="font-semibold text-lg-text">{{ item.q }}</h3>
          <p class="text-sm text-lg-text-sec mt-2 leading-relaxed">{{ item.a }}</p>
        </div>
      </div>
    </section>

    <footer class="border-t border-lg-border">
      <div
        class="lg-container py-10 flex flex-col md:flex-row gap-3 justify-between text-xs text-lg-text-muted"
      >
        <p>Lightning Git is a research prototype developed as a Swiss HF diploma project.</p>
        <p>All prices in Swiss Francs (CHF). Taxes may apply.</p>
      </div>
    </footer>
  </div>
</template>
