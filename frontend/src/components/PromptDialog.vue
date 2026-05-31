<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { resolvePrompt, usePromptState } from "@/utils/confirm";

const state = usePromptState();
const input = ref<HTMLInputElement | null>(null);
const draft = ref("");

watch(
  () => state.value.open,
  async (open) => {
    if (open) {
      draft.value = state.value.defaultValue ?? "";
      await nextTick();
      input.value?.focus();
      input.value?.select();
    }
  },
);

const trimmed = computed(() => draft.value.trim());
const tooShort = computed(() =>
  state.value.minLength !== undefined && trimmed.value.length < state.value.minLength,
);
const tooLong = computed(() =>
  state.value.maxLength !== undefined && trimmed.value.length > state.value.maxLength,
);
const invalid = computed(() => trimmed.value.length === 0 || tooShort.value || tooLong.value);

const validationHint = computed(() => {
  if (trimmed.value.length === 0) return "Required.";
  if (tooShort.value) return `At least ${state.value.minLength} characters.`;
  if (tooLong.value) return `At most ${state.value.maxLength} characters.`;
  return "";
});

function submit() {
  if (invalid.value) return;
  resolvePrompt(trimmed.value);
}

function cancel() {
  resolvePrompt(null);
}

function onKeydown(e: KeyboardEvent) {
  if (!state.value.open) return;
  if (e.key === "Escape") {
    e.stopPropagation();
    cancel();
  } else if (e.key === "Enter") {
    e.stopPropagation();
    submit();
  }
}
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
        aria-labelledby="prompt-title"
        @keydown="onKeydown"
      >
        <div
          class="absolute inset-0 bg-black/30 backdrop-blur-sm"
          @click="cancel"
        ></div>
        <div
          class="relative w-full max-w-md rounded-lg border border-lg-border bg-lg-bg shadow-[0_20px_60px_rgba(0,0,0,0.18),0_1px_3px_rgba(0,0,0,0.08)]"
        >
          <div class="p-6">
            <h2
              id="prompt-title"
              class="font-mono text-base font-bold text-lg-ink leading-tight tracking-[-0.01em]"
            >
              {{ state.title }}
            </h2>
            <p
              v-if="state.message"
              class="mt-3 text-sm text-lg-text-sec leading-relaxed whitespace-pre-line"
            >
              {{ state.message }}
            </p>
            <label class="block mt-4">
              <span
                v-if="state.label"
                class="block text-xs font-medium text-lg-text-sec mb-1.5"
              >{{ state.label }}</span>
              <input
                ref="input"
                v-model="draft"
                type="text"
                class="lg-input"
                :placeholder="state.placeholder"
                :maxlength="state.maxLength"
              />
            </label>
            <p
              v-if="validationHint && draft.length > 0"
              class="mt-1.5 text-[0.7rem] text-lg-rose"
            >
              {{ validationHint }}
            </p>
          </div>
          <div
            class="px-6 py-4 flex items-center justify-end gap-2 border-t border-lg-border bg-lg-bg-deep rounded-b-lg"
          >
            <button
              type="button"
              class="lg-btn-secondary text-xs px-4 py-2"
              @click="cancel"
            >
              {{ state.cancelLabel || "Cancel" }}
            </button>
            <button
              type="button"
              class="lg-btn-primary text-xs px-4 py-2 disabled:opacity-50 disabled:cursor-not-allowed"
              :disabled="invalid"
              @click="submit"
            >
              {{ state.confirmLabel || "Save" }}
            </button>
          </div>
        </div>
      </div>
    </transition>
  </teleport>
</template>
