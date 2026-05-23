<script setup lang="ts">
import { ref } from "vue";
import { useRouter, RouterLink } from "vue-router";
import { useAuthStore } from "@/stores/auth";
import { useToastStore } from "@/stores/toast";
import NavBar from "@/components/NavBar.vue";

const email = ref("");
const password = ref("");
const submitting = ref(false);

const authStore = useAuthStore();
const toast = useToastStore();
const router = useRouter();

async function onSubmit() {
  submitting.value = true;
  try {
    await authStore.login({ email: email.value, password: password.value });
    toast.success("Welcome back");
    await router.push({ name: "dashboard" });
  } catch {
    toast.error("Login failed. Check your email and password.");
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <div class="min-h-screen flex flex-col bg-lg-bg text-lg-text">
    <NavBar brand-to="/pricing">
      <RouterLink to="/pricing" class="lg-link">Pricing</RouterLink>
      <RouterLink to="/register" class="lg-link">Sign up</RouterLink>
    </NavBar>

    <main class="flex-1 flex items-center justify-center px-4 py-16">
      <form
        class="w-full max-w-md lg-card p-8 space-y-5"
        @submit.prevent="onSubmit"
      >
        <h1 class="text-2xl font-semibold">Sign in</h1>

        <label class="block">
          <span class="text-xs uppercase tracking-wider text-lg-text-sec font-medium">Email</span>
          <input
            v-model="email"
            type="email"
            required
            autocomplete="email"
            class="lg-input mt-1.5"
            placeholder="you@example.com"
          />
        </label>

        <label class="block">
          <span class="text-xs uppercase tracking-wider text-lg-text-sec font-medium">Password</span>
          <input
            v-model="password"
            type="password"
            required
            autocomplete="current-password"
            class="lg-input mt-1.5"
            placeholder="••••••••"
          />
        </label>

        <button
          type="submit"
          :disabled="submitting"
          class="lg-btn-primary w-full disabled:opacity-50"
        >
          {{ submitting ? "Signing in..." : "Sign in" }}
        </button>

        <p class="text-sm text-lg-text-sec text-center">
          No account?
          <RouterLink to="/register" class="text-lg-accent-bright hover:text-lg-accent-hover transition-colors">Create one</RouterLink>
        </p>
      </form>
    </main>
  </div>
</template>
