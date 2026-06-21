<script setup lang="ts">
import { RouterLink } from "vue-router";
import GitIcon from "@/components/GitIcon.vue";

withDefaults(
  defineProps<{
    brandTo?: string;
    wide?: boolean;
  }>(),
  { brandTo: "/", wide: false },
);
</script>

<template>
  <nav class="lg-nav">
    <div
      class="flex items-center justify-between py-3.5"
      :class="wide ? 'lg-container-wide' : 'lg-container'"
    >
      <RouterLink
        :to="brandTo"
        class="flex items-center gap-2 font-medium text-[1rem] text-lg-text"
      >
        <GitIcon class="text-lg-text-sec" :size="18" />
        <span>lightning-git</span>
      </RouterLink>
      <div class="flex items-center gap-5 text-sm">
        <slot />
      </div>
    </div>
  </nav>
</template>
