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

    <div v-if="orgStore.orgs.length === 0 && !error" class="text-zinc-500 mb-4">
      You're not in any organization yet.
    </div>

    <ul class="space-y-2 mb-6">
      <li
        v-for="org in orgStore.orgs"
        :key="org.id"
        class="border border-zinc-800 rounded p-3 hover:bg-zinc-900 cursor-pointer"
        @click="pick(org.id)"
      >
        <p class="font-medium">{{ org.name }}</p>
        <p class="text-xs text-zinc-500">{{ org.id }}</p>
      </li>
    </ul>

    <RouterLink
      to="/orgs/new"
      class="inline-block bg-zinc-100 text-zinc-900 rounded px-4 py-2 font-medium"
    >
      Create organization
    </RouterLink>
  </div>
</template>
