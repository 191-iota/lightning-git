<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { resolveConfirm, useConfirmState } from "@/utils/confirm";

const state = useConfirmState();
const confirmBtn = ref<HTMLButtonElement | null>(null);

// autofocus the primary action whenever the dialog opens, so Enter resolves
// it immediately without a tab. Escape unwinds via window keydown below.
watch(
  () => state.value.open,
  async (open) => {
    if (open) {
      await nextTick();
      confirmBtn.value?.focus();
    }
  },
);

function onKeydown(e: KeyboardEvent) {
  if (!state.value.open) return;
  if (e.key === "Escape") {
    e.stopPropagation();
    resolveConfirm(false);
  }
}

const confirmClass = computed(() =>
  state.value.danger
    ? "lg-btn-secondary border-lg-rose/60 text-lg-rose hover:bg-lg-rose/10 hover:border-lg-rose"
    : "lg-btn-primary",
);
</script>

<template>
  <teleport to="body">
    <transition
      enter-active-class="transition-opacity duration-150"
      enter-from-class="opacity-0"
      enter-to-class="opacity-100"
      leave-active-class="transition-opacity duration-100"
      leave-from-class="opacity-100"
      leave-to-class="opacity-0"
    >
      <div
        v-if="state.open"
        class="fixed inset-0 z-[1100] flex items-center justify-center p-4"
        role="dialog"
        aria-modal="true"
        :aria-labelledby="'confirm-title'"
        @keydown="onKeydown"
      >
        <div
          class="absolute inset-0 bg-black/30 backdrop-blur-sm"
          @click="resolveConfirm(false)"
        ></div>
        <div
          class="relative w-full max-w-md rounded-lg border border-lg-border bg-lg-bg shadow-[0_20px_60px_rgba(0,0,0,0.18),0_1px_3px_rgba(0,0,0,0.08)]"
        >
          <div class="p-6">
            <h2
              id="confirm-title"
              class="font-mono text-base font-bold text-lg-ink leading-tight tracking-[-0.01em]"
            >
              {{ state.title }}
            </h2>
            <p class="mt-3 text-sm text-lg-text-sec leading-relaxed whitespace-pre-line">
              {{ state.message }}
            </p>
          </div>
          <div
            class="px-6 py-4 flex items-center justify-end gap-2 border-t border-lg-border bg-lg-bg-deep rounded-b-lg"
          >
            <button
              type="button"
              class="lg-btn-secondary text-xs px-4 py-2"
              @click="resolveConfirm(false)"
            >
              {{ state.cancelLabel || "Cancel" }}
            </button>
            <button
              ref="confirmBtn"
              type="button"
              :class="confirmClass"
              class="text-xs px-4 py-2"
              @click="resolveConfirm(true)"
            >
              {{ state.confirmLabel || "Confirm" }}
            </button>
          </div>
        </div>
      </div>
    </transition>
  </teleport>
</template>
