<script setup lang="ts">
import { RouterLink, type RouteLocationRaw } from "vue-router";

export interface Tab {
  label: string;
  to: RouteLocationRaw;
  exact?: boolean;
}

defineProps<{ tabs: Tab[] }>();
</script>

<template>
  <div class="flex items-center gap-1 border-b border-lg-border -mx-1">
    <RouterLink
      v-for="t in tabs"
      :key="t.label"
      :to="t.to"
      :exact-active-class="'!border-lg-accent !text-lg-text'"
      :active-class="t.exact ? '' : '!border-lg-accent !text-lg-text'"
      class="px-4 py-2.5 text-sm font-medium border-b-2 border-transparent text-lg-text-sec hover:text-lg-text transition-colors -mb-px"
    >
      {{ t.label }}
    </RouterLink>
  </div>
</template>
