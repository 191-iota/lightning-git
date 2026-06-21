<script setup lang="ts">
import { useToastStore } from "@/stores/toast";

const store = useToastStore();

const kindStyles: Record<string, string> = {
  info: "border-lg-border bg-lg-bg text-lg-text",
  success: "border-emerald-300 bg-lg-bg text-lg-text",
  error: "border-rose-300 bg-lg-bg text-lg-text",
};
</script>

<template>
  <div
    class="fixed z-[1000] top-4 right-4 flex flex-col gap-2 max-w-sm w-[calc(100%-2rem)] sm:w-auto pointer-events-none"
  >
    <transition-group
      enter-active-class="transition-all duration-200 ease-out"
      enter-from-class="opacity-0 translate-x-4"
      enter-to-class="opacity-100 translate-x-0"
      leave-active-class="transition-all duration-150 ease-in"
      leave-from-class="opacity-100"
      leave-to-class="opacity-0 translate-x-4"
    >
      <div
        v-for="t in store.toasts"
        :key="t.id"
        class="rounded border flex items-start gap-3 py-2 px-3 text-sm pointer-events-auto bg-lg-bg shadow-sm"
        :class="kindStyles[t.kind]"
        role="status"
      >
        <p class="flex-1 leading-snug">{{ t.text }}</p>
        <button
          type="button"
          class="text-lg-text-muted hover:text-lg-text transition-colors p-1 -mr-1 leading-none"
          @click="store.dismiss(t.id)"
          aria-label="Dismiss"
        >
          <svg width="12" height="12" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" aria-hidden="true">
            <path d="M3 3 L11 11 M11 3 L3 11" />
          </svg>
        </button>
      </div>
    </transition-group>
  </div>
</template>
