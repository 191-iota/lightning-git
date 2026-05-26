<script setup lang="ts">
withDefaults(
  defineProps<{
    size?: number;
  }>(),
  { size: 14 },
);
</script>

<template>
  <svg
    :width="size"
    :height="size"
    viewBox="0 0 14 14"
    fill="none"
    stroke="currentColor"
    stroke-width="1.5"
    stroke-linecap="round"
    aria-hidden="true"
  >
    <path d="M3 3 L11 11 M11 3 L3 11" />
  </svg>
</template>
