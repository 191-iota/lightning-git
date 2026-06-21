<script setup lang="ts">
import { ref } from "vue";

export interface TreeNode {
  name: string;
  fullPath: string;
  isFile: boolean;
  children: TreeNode[];
}

const props = defineProps<{
  node: TreeNode;
  selected: string | null;
  editedFiles: Set<string>;
  depth?: number;
}>();

const emit = defineEmits<{
  (e: "open", path: string): void;
}>();

const expanded = ref(true);
const depth = props.depth ?? 0;
</script>

<template>
  <li v-if="node.isFile">
    <button
      class="w-full text-left px-2 py-1 rounded cursor-pointer hover:bg-lg-surface truncate flex items-center gap-2"
      :class="{ 'bg-lg-surface text-lg-ink': node.fullPath === selected }"
      :style="{ paddingLeft: `${depth * 0.75 + 0.5}rem` }"
      @click="emit('open', node.fullPath)"
    >
      <span
        class="inline-block w-2 h-2 rounded-full flex-shrink-0"
        :class="editedFiles.has(node.fullPath) ? 'bg-lg-ink animate-pulse' : 'bg-lg-border-strong'"
      ></span>
      <span class="truncate text-sm" :class="{ 'text-lg-ink font-medium': editedFiles.has(node.fullPath) }">
        {{ node.name }}
      </span>
      <span
        v-if="editedFiles.has(node.fullPath)"
        class="ml-auto text-[0.65rem] uppercase tracking-wider text-lg-text-sec flex-shrink-0 font-mono"
      >live</span>
    </button>
  </li>
  <li v-else>
    <button
      class="w-full text-left px-2 py-1 rounded cursor-pointer hover:bg-lg-surface flex items-center gap-2"
      :style="{ paddingLeft: `${depth * 0.75 + 0.5}rem` }"
      @click="expanded = !expanded"
    >
      <span class="text-xs text-lg-text-muted w-3">{{ expanded ? "▾" : "▸" }}</span>
      <span class="truncate text-sm text-lg-text-sec">{{ node.name }}</span>
    </button>
    <ul v-if="expanded">
      <FileTreeNode
        v-for="child in node.children"
        :key="child.name + child.fullPath"
        :node="child"
        :selected="selected"
        :edited-files="editedFiles"
        :depth="depth + 1"
        @open="(p) => emit('open', p)"
      />
    </ul>
  </li>
</template>
