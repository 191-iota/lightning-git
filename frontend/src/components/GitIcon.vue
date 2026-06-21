<script setup lang="ts">
withDefaults(
  defineProps<{
    size?: number | string;
    strokeWidth?: number | string;
  }>(),
  {
    size: 18,
    strokeWidth: 1.75,
  },
);
</script>

<template>
  <svg
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    :stroke-width="strokeWidth"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <circle cx="6" cy="3" r="2" />
    <circle cx="6" cy="18" r="2" />
    <circle cx="18" cy="9" r="2" />
    <path d="M6 5v8a4 4 0 0 0 4 4h2a4 4 0 0 0 4-4v-2" />
  </svg>
</template>
