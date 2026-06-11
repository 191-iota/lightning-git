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

// ---- prompt dialog: one-field text input variant ----

export interface PromptOptions {
  title: string;
  message?: string;
  label?: string;
  defaultValue?: string;
  placeholder?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  minLength?: number;
  maxLength?: number;
  // optional charset rule (source string for a RegExp) plus the hint shown when
  // the input violates it.
  pattern?: string;
  patternHint?: string;
}

interface PromptState extends PromptOptions {
  open: boolean;
}

const promptState = ref<PromptState>({
  open: false,
  title: "",
});

let promptResolver: ((value: string | null) => void) | null = null;

export function promptDialog(options: PromptOptions): Promise<string | null> {
  if (promptResolver) {
    promptResolver(null);
    promptResolver = null;
  }
  promptState.value = { ...options, open: true };
  return new Promise<string | null>((resolve) => {
    promptResolver = resolve;
  });
}

export function resolvePrompt(value: string | null): void {
  promptState.value = { ...promptState.value, open: false };
  const r = promptResolver;
  promptResolver = null;
  r?.(value);
}

export function usePromptState() {
  return promptState;
}
