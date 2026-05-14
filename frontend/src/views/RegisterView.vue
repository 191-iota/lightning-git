<script setup lang="ts">
import { ref } from "vue";
import { useRouter, RouterLink } from "vue-router";
import { useAuthStore } from "@/stores/auth";

const email = ref("");
const username = ref("");
const password = ref("");
const error = ref<string | null>(null);
const submitting = ref(false);

const authStore = useAuthStore();
const router = useRouter();

async function onSubmit() {
  error.value = null;
  submitting.value = true;
  try {
    await authStore.register({
      email: email.value,
      username: username.value,
      password: password.value,
    });
    await router.push({ name: "login" });
  } catch {
    error.value = "Registration failed";
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
      <h1 class="text-xl font-semibold">Create account</h1>

      <label class="block">
        <span class="text-sm text-zinc-400">Email</span>
        <input
          v-model="email"
          type="email"
          required
          autocomplete="email"
          class="mt-1 w-full bg-zinc-800 border border-zinc-700 rounded px-3 py-2 focus:outline-none focus:border-zinc-500"
        />
      </label>

      <label class="block">
        <span class="text-sm text-zinc-400">Username</span>
        <input
          v-model="username"
          type="text"
          required
          minlength="3"
          maxlength="32"
          autocomplete="username"
          class="mt-1 w-full bg-zinc-800 border border-zinc-700 rounded px-3 py-2 focus:outline-none focus:border-zinc-500"
        />
      </label>

      <label class="block">
        <span class="text-sm text-zinc-400">Password</span>
        <input
          v-model="password"
          type="password"
          required
          minlength="8"
          autocomplete="new-password"
          class="mt-1 w-full bg-zinc-800 border border-zinc-700 rounded px-3 py-2 focus:outline-none focus:border-zinc-500"
        />
      </label>

      <p v-if="error" class="text-sm text-red-400">{{ error }}</p>

      <button
        type="submit"
        :disabled="submitting"
        class="w-full bg-zinc-100 text-zinc-900 rounded py-2 font-medium disabled:opacity-50"
      >
        {{ submitting ? "Creating..." : "Create account" }}
      </button>

      <p class="text-sm text-zinc-500">
        Already have an account?
        <RouterLink to="/login" class="text-zinc-300 underline">Sign in</RouterLink>
      </p>
    </form>
  </div>
</template>
