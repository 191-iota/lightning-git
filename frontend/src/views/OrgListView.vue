<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useRouter, RouterLink } from "vue-router";
import { useOrgStore } from "@/stores/org";

const orgStore = useOrgStore();
const router = useRouter();
const error = ref<string | null>(null);

onMounted(async () => {
  try {
    await orgStore.fetch();
  } catch {
    error.value = "Failed to load orgs";
  }
});

function pick(id: string) {
  orgStore.select(id);
  router.push({ name: "dashboard" });
}
</script>

<template>
  <div class="min-h-screen bg-zinc-950 text-zinc-100 px-6 py-8">
    <header class="mb-6">
      <h1 class="text-xl font-semibold">Your organizations</h1>
      <p class="text-sm text-zinc-500">Pick one to continue, or create a new one.</p>
    </header>

    <p v-if="error" class="text-sm text-red-400 mb-4">{{ error }}</p>

    <div
      v-if="orgStore.orgs.length === 0 && !error"
      class="border border-dashed border-zinc-800 rounded-lg p-8 text-center mb-6"
    >
      <p class="text-zinc-300 mb-1">No organizations yet</p>
      <p class="text-sm text-zinc-500 mb-4">
        Create one to start adding projects and team members.
      </p>
      <RouterLink
        to="/orgs/new"
        class="inline-block bg-zinc-100 text-zinc-900 rounded px-4 py-2 font-medium"
      >
        Create your first organization
      </RouterLink>
    </div>

    <ul v-if="orgStore.orgs.length > 0" class="space-y-2 mb-6">
      <li
        v-for="org in orgStore.orgs"
        :key="org.id"
        class="border border-zinc-800 rounded p-3 hover:bg-zinc-900 cursor-pointer"
        @click="pick(org.id)"
      >
        <p class="font-medium flex items-center gap-2">
          {{ org.name }}
          <span
            class="text-xs px-1.5 py-0.5 rounded"
            :class="org.role === 'owner' ? 'bg-amber-900/50 text-amber-300' : 'bg-zinc-800 text-zinc-400'"
          >
            {{ org.role }}
          </span>
        </p>
        <p class="text-xs text-zinc-500">{{ org.id }}</p>
      </li>
    </ul>

    <RouterLink
      v-if="orgStore.orgs.length > 0"
      to="/orgs/new"
      class="inline-block bg-zinc-100 text-zinc-900 rounded px-4 py-2 font-medium"
    >
      Create organization
    </RouterLink>
  </div>
</template>
