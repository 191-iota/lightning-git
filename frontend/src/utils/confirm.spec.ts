import { afterEach, describe, expect, it } from "vitest";
import {
  confirmDialog,
  promptDialog,
  resolveConfirm,
  resolvePrompt,
  useConfirmState,
  usePromptState,
} from "./confirm";

// confirm.ts is a small promise bridge between imperative call sites and the
// single shared dialog component. The stable contract is: calling opens the
// dialog, resolving settles the returned promise and closes the dialog, and
// opening a second dialog while one is still pending cancels the first so a
// caller can never be left awaiting a promise that never settles. The exact
// state shape beyond `open` is an implementation detail and is not asserted.

afterEach(() => {
  // settle anything a test left pending so it cannot leak into the next one
  resolveConfirm(false);
  resolvePrompt(null);
});

describe("confirmDialog", () => {
  it("opens the dialog and resolves true when confirmed", async () => {
    const state = useConfirmState();
    const answer = confirmDialog({ title: "Delete project", message: "Are you sure?" });
    expect(state.value.open).toBe(true);
    expect(state.value.title).toBe("Delete project");

    resolveConfirm(true);
    await expect(answer).resolves.toBe(true);
    expect(state.value.open).toBe(false);
  });

  it("resolves false when cancelled", async () => {
    const answer = confirmDialog({ title: "x", message: "y" });
    resolveConfirm(false);
    await expect(answer).resolves.toBe(false);
  });

  it("cancels a still-pending dialog when a new one opens", async () => {
    const first = confirmDialog({ title: "first", message: "" });
    const second = confirmDialog({ title: "second", message: "" });
    // the superseded promise must settle (to false) instead of hanging forever
    await expect(first).resolves.toBe(false);

    resolveConfirm(true);
    await expect(second).resolves.toBe(true);
  });
});

describe("promptDialog", () => {
  it("opens the prompt and resolves with the entered string", async () => {
    const state = usePromptState();
    const answer = promptDialog({ title: "Rename" });
    expect(state.value.open).toBe(true);

    resolvePrompt("new-name");
    await expect(answer).resolves.toBe("new-name");
    expect(state.value.open).toBe(false);
  });

  it("resolves null when dismissed", async () => {
    const answer = promptDialog({ title: "x" });
    resolvePrompt(null);
    await expect(answer).resolves.toBeNull();
  });

  it("cancels a still-pending prompt when a new one opens", async () => {
    const first = promptDialog({ title: "first" });
    const second = promptDialog({ title: "second" });
    await expect(first).resolves.toBeNull();

    resolvePrompt("ok");
    await expect(second).resolves.toBe("ok");
  });
});
