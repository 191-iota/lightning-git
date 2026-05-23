<script setup lang="ts">
withDefaults(
  defineProps<{
    width?: string;
    height?: string;
    rounded?: "sm" | "md" | "lg" | "full";
  }>(),
  {
    width: "100%",
    height: "0.75rem",
    rounded: "md",
  },
);

const radiusClass = {
  sm: "rounded",
  md: "rounded-md",
  lg: "rounded-lg",
  full: "rounded-full",
};
</script>

<template>
  <div
    class="bg-lg-surface-2 animate-pulse"
    :class="radiusClass[rounded]"
    :style="{ width, height }"
  ></div>
</template>
