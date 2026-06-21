import { beforeEach, describe, expect, it } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import type { RouteLocationNormalized } from "vue-router";
import router, { navigationGuard } from "./index";

// The guard's job is the access contract, independent of any specific view:
// protected routes require a session, guest-only routes bounce a logged-in
// user, and org-scoped routes require a selected org. The stores read their
// initial state from localStorage on creation, so each case seeds storage
// before the guard instantiates them.

function to(meta: Record<string, unknown>): RouteLocationNormalized {
  return { meta } as unknown as RouteLocationNormalized;
}

beforeEach(() => {
  localStorage.clear();
  setActivePinia(createPinia());
});

describe("navigationGuard", () => {
  it("sends an unauthenticated visitor on a protected route to login", () => {
    expect(navigationGuard(to({ requiresAuth: true }))).toEqual({ name: "login" });
  });

  it("bounces a logged-in user away from a guest-only route to the dashboard", () => {
    localStorage.setItem("token", "a");
    expect(navigationGuard(to({ requiresGuest: true }))).toEqual({ name: "dashboard" });
  });

  it("sends a logged-in user with no org selected to the org picker", () => {
    localStorage.setItem("token", "a");
    expect(navigationGuard(to({ requiresAuth: true, requiresOrg: true }))).toEqual({ name: "orgs" });
  });

  it("lets a logged-in user with an org through an org-scoped route", () => {
    localStorage.setItem("token", "a");
    localStorage.setItem("currentOrgId", "o1");
    expect(navigationGuard(to({ requiresAuth: true, requiresOrg: true }))).toBe(true);
  });

  it("lets a logged-out visitor reach a guest-only route", () => {
    expect(navigationGuard(to({ requiresGuest: true }))).toBe(true);
  });
});

describe("root route", () => {
  it("redirects / into the app dashboard rather than a public landing", () => {
    const root = router.getRoutes().find((r) => r.path === "/");
    expect(root?.redirect).toEqual({ name: "dashboard" });
  });
});
