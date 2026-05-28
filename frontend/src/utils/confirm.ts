import { ref } from "vue";

export interface ConfirmOptions {
  title: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  danger?: boolean;
}

interface ConfirmState extends ConfirmOptions {
  open: boolean;
}

const state = ref<ConfirmState>({
  open: false,
  title: "",
  message: "",
});

let resolver: ((value: boolean) => void) | null = null;

export function confirmDialog(options: ConfirmOptions): Promise<boolean> {
  if (resolver) {
    resolver(false);
    resolver = null;
  }
  state.value = { ...options, open: true };
  return new Promise<boolean>((resolve) => {
    resolver = resolve;
  });
}

export function resolveConfirm(value: boolean): void {
  state.value = { ...state.value, open: false };
  const r = resolver;
  resolver = null;
  r?.(value);
}

export function useConfirmState() {
  return state;
}
