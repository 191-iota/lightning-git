<script setup lang="ts">
import { RouterView } from 'vue-router'
import ToastHost from '@/components/ToastHost.vue'
</script>

<template>
  <!-- key on fullPath forces a fresh component instance on every route change,
       so query-only navigation (?branch=, ?file=) and browser back/forward
       re-run onMounted and re-bind the per-file WS instead of reusing stale
       refs from the previous route. -->
  <RouterView :key="$route.fullPath" />
  <ToastHost />
</template>
