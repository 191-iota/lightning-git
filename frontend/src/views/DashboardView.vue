<script setup lang="ts">
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { useOrgStore } from '@/stores/org'

const authStore = useAuthStore()
const orgStore = useOrgStore()
const router = useRouter()

async function onLogout() {
  authStore.logout()
  orgStore.clear()
  await router.push({ name: 'login' })
}
</script>

<template>
  <div class="min-h-screen bg-zinc-950 text-zinc-100 px-6 py-8">
    <header class="flex items-center justify-between mb-8">
      <h1 class="text-xl font-semibold">Lightning Git</h1>
      <div class="flex items-center gap-4">
        <span class="text-sm text-zinc-400">{{ authStore.user?.email }}</span>
        <button
          class="text-sm text-zinc-300 hover:text-zinc-100"
          @click="onLogout"
        >
          Sign out
        </button>
      </div>
    </header>

    <main class="border border-zinc-800 rounded-lg p-6">
      <p class="text-zinc-400">Dashboard DAAASHBOAAAAéööööööööaaaard. Org XY project XY</p>
    </main>
  </div>
</template>
