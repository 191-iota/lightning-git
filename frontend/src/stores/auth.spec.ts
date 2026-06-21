import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";

// The auth store talks to the backend only through the shared axios instance.
// We replace that module so no real request is made and we can script the
// /login and /refresh responses. onUnauthorized is a no-op here; its wiring is
// covered by the api interceptor tests.
vi.mock("@/services/api", () => ({
  default: { post: vi.fn() },
  onUnauthorized: vi.fn(),
}));

import api from "@/services/api";
import { useAuthStore } from "./auth";

const post = api.post as unknown as ReturnType<typeof vi.fn>;

beforeEach(() => {
  localStorage.clear();
  post.mockReset();
  setActivePinia(createPinia());
});

describe("auth store refresh", () => {
  it("shares one in-flight request across concurrent callers (single-flight)", async () => {
    localStorage.setItem("refreshToken", "r0");
    const store = useAuthStore();

    let release!: (v: { data: { access_token: string; refresh_token: string } }) => void;
    post.mockReturnValue(new Promise((resolve) => { release = resolve; }));

    const first = store.refresh();
    const second = store.refresh();
    expect(post).toHaveBeenCalledTimes(1);

    release({ data: { access_token: "a1", refresh_token: "r1" } });
    const [a, b] = await Promise.all([first, second]);
    expect(a).toBe("a1");
    expect(b).toBe("a1");
  });

  it("returns null and clears the stale access token when there is no refresh token", async () => {
    localStorage.setItem("token", "stale");
    const store = useAuthStore();

    const result = await store.refresh();

    expect(result).toBeNull();
    expect(post).not.toHaveBeenCalled();
    expect(store.token).toBeNull();
    expect(localStorage.getItem("token")).toBeNull();
  });

  it("persists the rotated access and refresh tokens on success", async () => {
    localStorage.setItem("refreshToken", "r0");
    const store = useAuthStore();
    post.mockResolvedValue({ data: { access_token: "a1", refresh_token: "r1" } });

    const result = await store.refresh();

    expect(result).toBe("a1");
    expect(store.token).toBe("a1");
    expect(localStorage.getItem("token")).toBe("a1");
    expect(localStorage.getItem("refreshToken")).toBe("r1");
  });

  it("clears auth when the refresh request fails", async () => {
    localStorage.setItem("token", "a0");
    localStorage.setItem("refreshToken", "r0");
    const store = useAuthStore();
    post.mockRejectedValue(new Error("network"));

    const result = await store.refresh();

    expect(result).toBeNull();
    expect(store.token).toBeNull();
    expect(localStorage.getItem("refreshToken")).toBeNull();
  });

  it("makes a fresh request once the previous refresh has settled", async () => {
    localStorage.setItem("refreshToken", "r0");
    const store = useAuthStore();
    post.mockResolvedValue({ data: { access_token: "a1", refresh_token: "r1" } });

    await store.refresh();
    await store.refresh();

    expect(post).toHaveBeenCalledTimes(2);
  });
});

describe("auth store session lifecycle", () => {
  it("login stores the access token, refresh token and user", async () => {
    const store = useAuthStore();
    post.mockResolvedValue({
      data: { user_id: "u1", email: "dev@example.com", access_token: "a1", refresh_token: "r1" },
    });

    await store.login({ email: "dev@example.com", password: "pw" });

    expect(store.isAuthenticated).toBe(true);
    expect(store.token).toBe("a1");
    expect(store.user).toEqual({ id: "u1", email: "dev@example.com" });
    expect(localStorage.getItem("user")).toContain("u1");
  });

  it("logout removes the session from state and storage", () => {
    localStorage.setItem("token", "a");
    localStorage.setItem("refreshToken", "r");
    localStorage.setItem("user", JSON.stringify({ id: "u", email: "e@x.com" }));
    const store = useAuthStore();
    expect(store.isAuthenticated).toBe(true);

    store.logout();

    expect(store.isAuthenticated).toBe(false);
    expect(localStorage.getItem("token")).toBeNull();
    expect(localStorage.getItem("refreshToken")).toBeNull();
    expect(localStorage.getItem("user")).toBeNull();
  });
});
