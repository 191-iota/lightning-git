<script setup lang="ts">
import { ref } from "vue";
import { RouterLink } from "vue-router";
import { useAuthStore } from "@/stores/auth";
import { useToastStore } from "@/stores/toast";
import NavBar from "@/components/NavBar.vue";

const email = ref("");
const username = ref("");
const password = ref("");
const submitting = ref(false);
const registered = ref(false);
const registeredEmail = ref("");

const authStore = useAuthStore();
const toast = useToastStore();

async function onSubmit() {
  submitting.value = true;
  try {
    await authStore.register({
      email: email.value,
      username: username.value,
      password: password.value,
    });
    registeredEmail.value = email.value;
    registered.value = true;
  } catch {
    toast.error("Registration failed. The email or username may already be taken.");
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <div class="min-h-screen flex flex-col bg-lg-bg text-lg-text">
    <NavBar brand-to="/pricing">
      <RouterLink to="/pricing" class="lg-link">Pricing</RouterLink>
      <RouterLink to="/login" class="lg-link">Log in</RouterLink>
    </NavBar>

    <main class="flex-1 flex items-center justify-center px-4 py-16">
      <div
        v-if="registered"
        class="w-full max-w-md lg-card p-8 space-y-5 text-center"
      >
        <h1 class="text-2xl font-semibold">Confirm your email</h1>
        <p class="text-sm text-lg-text-sec leading-relaxed">
          We sent a confirmation link to
          <span class="text-lg-accent-bright font-mono break-all">{{ registeredEmail }}</span>.
          Click the link in that email, then come back and sign in.
        </p>
        <p class="text-xs text-lg-text-muted">
          Did not receive it? Check your spam folder or try again in a minute.
        </p>
        <RouterLink to="/login" class="lg-btn-primary w-full inline-flex">
          Go to sign in
        </RouterLink>
      </div>

      <form
        v-else
        class="w-full max-w-md lg-card p-8 space-y-5"
        @submit.prevent="onSubmit"
      >
        <h1 class="text-2xl font-semibold">Create account</h1>

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
          <span class="text-xs uppercase tracking-wider text-lg-text-sec font-medium">Username</span>
          <input
            v-model="username"
            type="text"
            required
            minlength="3"
            maxlength="32"
            autocomplete="username"
            class="lg-input mt-1.5"
            placeholder="octocat"
          />
        </label>

        <label class="block">
          <span class="text-xs uppercase tracking-wider text-lg-text-sec font-medium">Password</span>
          <input
            v-model="password"
            type="password"
            required
            minlength="8"
            autocomplete="new-password"
            class="lg-input mt-1.5"
            placeholder="At least 8 characters"
          />
        </label>

        <button
          type="submit"
          :disabled="submitting"
          class="lg-btn-primary w-full disabled:opacity-50"
        >
          {{ submitting ? "Creating..." : "Create account" }}
        </button>

        <p class="text-xs text-lg-text-muted text-center leading-relaxed">
          You will need to confirm your email before signing in.
        </p>

        <p class="text-sm text-lg-text-sec text-center">
          Already have an account?
          <RouterLink to="/login" class="text-lg-accent-bright hover:text-lg-accent-hover transition-colors">Sign in</RouterLink>
        </p>
      </form>
    </main>
  </div>
</template>
