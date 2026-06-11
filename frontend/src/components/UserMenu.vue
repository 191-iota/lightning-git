<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import { useAuthStore } from "@/stores/auth";
import { useToastStore } from "@/stores/toast";
import { promptDialog } from "@/utils/confirm";

// Sign-out cleanup (clearing the org/project stores, navigating to /login)
// differs per host view, so the parent keeps owning it and we emit upward.
const emit = defineEmits<{ logout: [] }>();

const auth = useAuthStore();
const toast = useToastStore();

const open = ref(false);
const root = ref<HTMLElement | null>(null);

function toggle() {
  open.value = !open.value;
}

function close() {
  open.value = false;
}

function onDocumentClick(e: MouseEvent) {
  if (root.value && !root.value.contains(e.target as Node)) close();
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") close();
}

onMounted(() => {
  document.addEventListener("mousedown", onDocumentClick);
  document.addEventListener("keydown", onKeydown);
});
onBeforeUnmount(() => {
  document.removeEventListener("mousedown", onDocumentClick);
  document.removeEventListener("keydown", onKeydown);
});

async function onChangeUsername() {
  close();
  const next = await promptDialog({
    title: "Change username",
    message: "This is the handle other members see and use to invite you.",
    label: "New username",
    defaultValue: auth.user?.display_name ?? "",
    placeholder: "your-handle",
    confirmLabel: "Save",
    minLength: 3,
    maxLength: 32,
    pattern: "^[a-zA-Z0-9_-]+$",
    patternHint: "Use letters, numbers, hyphens or underscores.",
  });
  if (next === null) return;
  if (next === auth.user?.display_name) return;

  try {
    const saved = await auth.changeUsername(next);
    toast.success(`Username changed to ${saved}.`);
  } catch (err: unknown) {
    const status =
      typeof err === "object" && err !== null
        ? ((err as { response?: { status?: number } }).response?.status ?? 0)
        : 0;
    if (status === 409) {
      toast.error("That username is already taken.");
    } else {
      toast.error("Could not change username.");
    }
  }
}

function onLogout() {
  close();
  emit("logout");
}
</script>

<template>
  <div ref="root" class="relative">
    <button
      type="button"
      class="lg-link inline-flex items-center gap-1.5"
      :aria-expanded="open"
      aria-haspopup="menu"
      @click="toggle"
    >
      <span class="max-w-[12rem] truncate">{{ auth.displayName }}</span>
      <svg
        viewBox="0 0 12 12"
        :class="['w-3 h-3 transition-transform', open ? 'rotate-180' : '']"
        fill="none"
        aria-hidden="true"
      >
        <path d="M3 4.5 6 7.5 9 4.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    </button>

    <transition
      enter-active-class="transition duration-100 ease-out"
      enter-from-class="opacity-0 -translate-y-1"
      enter-to-class="opacity-100 translate-y-0"
      leave-active-class="transition duration-75 ease-in"
      leave-from-class="opacity-100 translate-y-0"
      leave-to-class="opacity-0 -translate-y-1"
    >
      <div
        v-if="open"
        role="menu"
        class="lg-card absolute right-0 mt-2 w-48 py-1 shadow-[0_12px_32px_rgba(0,0,0,0.18)] z-50"
      >
        <div
          v-if="auth.user?.email"
          class="px-3 py-2 text-xs text-lg-text-muted truncate border-b border-lg-border"
        >
          {{ auth.user.email }}
        </div>
        <button
          type="button"
          role="menuitem"
          class="block w-full text-left px-3 py-2 text-sm text-lg-text hover:bg-lg-surface transition-colors"
          @click="onChangeUsername"
        >
          Change username
        </button>
        <button
          type="button"
          role="menuitem"
          class="block w-full text-left px-3 py-2 text-sm text-lg-text hover:bg-lg-surface transition-colors"
          @click="onLogout"
        >
          Sign out
        </button>
      </div>
    </transition>
  </div>
</template>
