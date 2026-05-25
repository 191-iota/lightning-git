<script setup lang="ts">
import { ref } from "vue";
import { useRouter, RouterLink } from "vue-router";
import { useOrgStore } from "@/stores/org";
import { useProjectStore } from "@/stores/project";
import { useAuthStore } from "@/stores/auth";
import { useToastStore } from "@/stores/toast";
import NavBar from "@/components/NavBar.vue";

const name = ref("");
const repoUrl = ref("");
const submitting = ref(false);
const isPrivate = ref(false);

const orgStore = useOrgStore();
const projectStore = useProjectStore();
const auth = useAuthStore();
const toast = useToastStore();
const router = useRouter();

// open the backend's GitHub OAuth start endpoint in a new tab. the backend
// stores the resulting token under the caller's user_id and uses it for the
// clone, so all the frontend has to do is kick off the redirect and let the
// user confirm when they're back.
function authorizeGithub() {
  if (!auth.user?.id) {
    toast.error("Sign in again before authorizing GitHub.");
    return;
  }
  const apiUrl = import.meta.env.VITE_API_BASE_URL || "http://localhost:8080";
  window.open(`${apiUrl}/auth/github/${auth.user.id}`, "_blank", "noopener");
}

async function onSubmit() {
  if (!orgStore.currentOrgId) {
    toast.error("Pick an organization first.");
    return;
  }
  submitting.value = true;
  try {
    await projectStore.create({
      name: name.value,
      repo_url: repoUrl.value,
      org_id: orgStore.currentOrgId,
      create_tasks_retroactively: false,
    });
    toast.success(`Project "${name.value}" created.`);
    await router.push({ name: "dashboard" });
  } catch {
    toast.error("Failed to create project. Check the repository URL and permissions.");
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <div class="min-h-screen flex flex-col bg-lg-bg text-lg-text">
    <NavBar />

    <main class="flex-1 flex flex-col items-center px-4 py-16">
      <div class="w-full max-w-md mb-4">
        <RouterLink to="/dashboard" class="lg-breadcrumb">&larr; Back to dashboard</RouterLink>
      </div>
      <form class="w-full max-w-md lg-card p-8 space-y-5" @submit.prevent="onSubmit">
        <h1 class="text-2xl font-semibold">New project</h1>

        <label class="block">
          <span class="text-xs uppercase tracking-wider text-lg-text-sec font-medium">Name</span>
          <input
            v-model="name"
            type="text"
            required
            minlength="3"
            maxlength="255"
            class="lg-input mt-1.5"
            placeholder="my-awesome-project"
          />
        </label>

        <label class="block">
          <span class="text-xs uppercase tracking-wider text-lg-text-sec font-medium">Repository URL</span>
          <input
            v-model="repoUrl"
            type="url"
            required
            placeholder="https://github.com/owner/repo.git"
            class="lg-input mt-1.5 font-mono text-xs"
          />
          <span class="block text-xs text-lg-text-muted mt-1.5">
            HTTPS clone URL only.
          </span>
        </label>

        <label class="flex items-center gap-2 text-sm">
          <input v-model="isPrivate" type="checkbox" class="accent-lg-accent" />
          <span>This is a private repository</span>
        </label>

        <div v-if="isPrivate" class="rounded-md border border-lg-border bg-lg-surface-2 p-3 text-xs space-y-2">
          <p class="text-lg-text-sec">
            Authorize Lightning Git on GitHub so the backend can clone this repo on your behalf. Open the link, finish the OAuth flow, then come back and submit.
          </p>
          <button type="button" class="lg-btn-secondary text-xs py-1.5 px-3" @click="authorizeGithub">
            Authorize on GitHub
          </button>
        </div>

        <button type="submit" :disabled="submitting" class="lg-btn-primary w-full disabled:opacity-50">
          {{ submitting ? "Cloning..." : "Create project" }}
        </button>
      </form>
    </main>
  </div>
</template>
