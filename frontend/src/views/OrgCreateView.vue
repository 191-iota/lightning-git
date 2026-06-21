<script setup lang="ts">
import { ref } from "vue";
import { useRouter, RouterLink } from "vue-router";
import { useOrgStore } from "@/stores/org";
import { useToastStore } from "@/stores/toast";
import NavBar from "@/components/NavBar.vue";

const name = ref("");
const submitting = ref(false);

const orgStore = useOrgStore();
const toast = useToastStore();
const router = useRouter();

async function onSubmit() {
  submitting.value = true;
  try {
    const orgId = await orgStore.create({ name: name.value });
    orgStore.select(orgId);
    toast.success(`Organization "${name.value}" created.`);
    await router.push({ name: "dashboard" });
  } catch {
    toast.error("Failed to create organization. Try a different name.");
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <div class="min-h-screen flex flex-col bg-lg-bg text-lg-text">
    <NavBar brand-to="/orgs" />

    <main class="flex-1 flex flex-col items-center px-4 py-16">
      <div class="w-full max-w-md mb-4">
        <RouterLink to="/orgs" class="lg-breadcrumb">&larr; Back to organizations</RouterLink>
      </div>
      <form class="w-full max-w-md lg-card p-8 space-y-5" @submit.prevent="onSubmit">
        <h1 class="text-2xl font-semibold">New organization</h1>

        <label class="block">
          <span class="text-xs uppercase tracking-wider text-lg-text-sec font-medium">Name</span>
          <input
            v-model="name"
            type="text"
            required
            minlength="2"
            maxlength="64"
            class="lg-input mt-1.5"
            placeholder="Acme Corp"
          />
        </label>

        <button type="submit" :disabled="submitting" class="lg-btn-primary w-full disabled:opacity-50">
          {{ submitting ? "Creating..." : "Create organization" }}
        </button>
      </form>
    </main>
  </div>
</template>
