<script setup lang="ts">
import { onMounted } from "vue";
import { useRouter, RouterLink } from "vue-router";
import { useOrgStore } from "@/stores/org";
import { useToastStore } from "@/stores/toast";
import NavBar from "@/components/NavBar.vue";

const orgStore = useOrgStore();
const toast = useToastStore();
const router = useRouter();

onMounted(async () => {
  try {
    await orgStore.fetch();
  } catch {
    toast.error("Could not load organizations.");
  }
});

function pick(id: string) {
  orgStore.select(id);
  router.push({ name: "dashboard" });
}
</script>

<template>
  <div class="min-h-screen bg-lg-bg text-lg-text">
    <NavBar brand-to="/orgs">
    </NavBar>

    <main class="lg-container py-12">
      <header class="mb-8">
        <h1 class="text-3xl font-semibold">Organizations</h1>
      </header>

      <div
        v-if="orgStore.orgs.length === 0"
        class="lg-card p-10 text-center"
      >
        <p class="text-lg-text-sec mb-5 text-sm">You don't belong to any organization yet.</p>
        <RouterLink to="/orgs/new" class="lg-btn-primary inline-flex">
          New organization
        </RouterLink>
      </div>

      <ul v-if="orgStore.orgs.length > 0" class="space-y-3 mb-6">
        <li
          v-for="org in orgStore.orgs"
          :key="org.id"
          class="lg-card p-4 hover:border-lg-border-strong hover:bg-lg-surface-2 cursor-pointer transition-colors"
          @click="pick(org.id)"
        >
          <p class="text-lg-text flex items-baseline gap-2.5">
            <span class="font-medium">{{ org.name }}</span>
            <span
              class="text-xs"
              :class="org.role === 'owner' ? 'text-lg-accent-bright' : 'text-lg-text-muted'"
            >{{ org.role }}</span>
          </p>
          <p class="text-xs text-lg-text-muted mt-1 font-mono truncate">{{ org.id }}</p>
        </li>
      </ul>

      <RouterLink
        v-if="orgStore.orgs.length > 0"
        to="/orgs/new"
        class="lg-btn-primary inline-flex"
      >
        New organization
      </RouterLink>
    </main>
  </div>
</template>
