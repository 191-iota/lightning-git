<script setup lang="ts">
import { ref } from "vue";
import { useRouter, RouterLink } from "vue-router";
import { useOrgStore } from "@/stores/org";
import { useProjectStore } from "@/stores/project";

const name = ref("");
const repoUrl = ref("");
const error = ref<string | null>(null);
const submitting = ref(false);

const orgStore = useOrgStore();
const projectStore = useProjectStore();
const router = useRouter();

async function onSubmit() {
  if (!orgStore.currentOrgId) {
    error.value = "No org selected";
    return;
  }
  error.value = null;
  submitting.value = true;
  try {
    await projectStore.create({
      name: name.value,
      repo_url: repoUrl.value,
      org_id: orgStore.currentOrgId,
      create_tasks_retroactively: false,
    });
    await router.push({ name: "dashboard" });
  } catch {
    error.value = "Failed to create project";
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
      <h1 class="text-xl font-semibold">New project</h1>

      <label class="block">
        <span class="text-sm text-zinc-400">Name</span>
        <input
          v-model="name"
          type="text"
          required
          minlength="3"
          maxlength="255"
          class="mt-1 w-full bg-zinc-800 border border-zinc-700 rounded px-3 py-2 focus:outline-none focus:border-zinc-500"
        />
      </label>

      <label class="block">
        <span class="text-sm text-zinc-400">Repository URL</span>
        <input
          v-model="repoUrl"
          type="url"
          required
          placeholder="https://github.com/owner/repo.git"
          class="mt-1 w-full bg-zinc-800 border border-zinc-700 rounded px-3 py-2 focus:outline-none focus:border-zinc-500"
        />
        <span class="block text-xs text-zinc-500 mt-1">
          Use the HTTPS clone URL from GitHub (not SSH).
        </span>
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
        <RouterLink to="/dashboard" class="text-zinc-300 underline">Back to dashboard</RouterLink>
      </p>
    </form>
  </div>
</template>
