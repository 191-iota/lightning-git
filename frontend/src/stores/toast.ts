import { defineStore } from "pinia";
import { ref } from "vue";

export type ToastKind = "info" | "success" | "error";

export interface Toast {
  id: number;
  kind: ToastKind;
  text: string;
}

const DEFAULT_TTL_MS = 4500;

export const useToastStore = defineStore("toast", () => {
  const toasts = ref<Toast[]>([]);
  let nextId = 1;

  function push(kind: ToastKind, text: string, ttl = DEFAULT_TTL_MS) {
    const id = nextId++;
    toasts.value.push({ id, kind, text });
    if (ttl > 0) {
      setTimeout(() => dismiss(id), ttl);
    }
    return id;
  }

  function dismiss(id: number) {
    toasts.value = toasts.value.filter((t) => t.id !== id);
  }

  function info(text: string) {
    return push("info", text);
  }
  function success(text: string) {
    return push("success", text);
  }
  function error(text: string) {
    return push("error", text);
  }

  return { toasts, push, dismiss, info, success, error };
});
