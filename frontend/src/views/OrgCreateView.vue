<script setup lang="ts">
import { ref } from "vue";
import { useRouter, RouterLink } from "vue-router";
import { useOrgStore } from "@/stores/org";

const name = ref("");
const error = ref<string | null>(null);
const submitting = ref(false);

const orgStore = useOrgStore();
const router = useRouter();

async function onSubmit() {
  error.value = null;
  submitting.value = true;
  try {
    const orgId = await orgStore.create({ name: name.value });
    orgStore.select(orgId);
    await router.push({ name: "dashboard" });
  } catch {
    error.value = "Failed to create org";
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <div class="min-h-screen flex items-center justify-center bg-zinc-950 text-zinc-100 px-4">
    <form
      class="w-full max-w-sm bg-zinc-900 rounded-lg p-6 space-y-4 border border-zinc-800"
      @submit.prevent="onSubmit"
    >
      <h1 class="text-xl font-semibold">New organization</h1>

      <label class="block">
        <span class="text-sm text-zinc-400">Name</span>
        <input
          v-model="name"
          type="text"
          required
          minlength="2"
          maxlength="64"
          class="mt-1 w-full bg-zinc-800 border border-zinc-700 rounded px-3 py-2 focus:outline-none focus:border-zinc-500"
        />
      </label>

      <p v-if="error" class="text-sm text-red-400">{{ error }}</p>

      <button
        type="submit"
        :disabled="submitting"
        class="w-full bg-zinc-100 text-zinc-900 rounded py-2 font-medium disabled:opacity-50"
      >
        {{ submitting ? "Creating..." : "Create" }}
      </button>

      <p class="text-sm text-zinc-500">
        <RouterLink to="/orgs" class="text-zinc-300 underline">Back to list</RouterLink>
      </p>
    </form>
  </div>
</template>
