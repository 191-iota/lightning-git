import { createRouter, createWebHistory } from "vue-router";
import { useAuthStore } from "@/stores/auth";
import { useOrgStore } from "@/stores/org";

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/",
      redirect: "/dashboard",
    },
    {
      path: "/login",
      name: "login",
      component: () => import("@/views/LoginView.vue"),
      meta: { requiresGuest: true },
    },
    {
      path: "/register",
      name: "register",
      component: () => import("@/views/RegisterView.vue"),
      meta: { requiresGuest: true },
    },
    {
      path: "/orgs",
      name: "orgs",
      component: () => import("@/views/OrgListView.vue"),
      meta: { requiresAuth: true },
    },
    {
      path: "/orgs/new",
      name: "orgs-new",
      component: () => import("@/views/OrgCreateView.vue"),
      meta: { requiresAuth: true },
    },
    {
      path: "/dashboard",
      name: "dashboard",
      component: () => import("@/views/DashboardView.vue"),
      meta: { requiresAuth: true, requiresOrg: true },
    },
    {
      path: "/projects/new",
      name: "projects-new",
      component: () => import("@/views/ProjectCreateView.vue"),
      meta: { requiresAuth: true, requiresOrg: true },
    },
    {
      path: "/projects/:id",
      name: "project",
      component: () => import("@/views/ProjectView.vue"),
      meta: { requiresAuth: true, requiresOrg: true },
    },
    {
      path: "/projects/:id/overlay",
      name: "overlay",
      component: () => import("@/views/OverlayView.vue"),
      meta: { requiresAuth: true, requiresOrg: true },
    },
  ],
});

router.beforeEach((to) => {
  const authStore = useAuthStore();
  const orgStore = useOrgStore();
  if (to.meta.requiresAuth && !authStore.isAuthenticated) return { name: "login" };
  if (to.meta.requiresGuest && authStore.isAuthenticated) return { name: "dashboard" };
  // dashboard etc. need an org picked first
  if (to.meta.requiresOrg && !orgStore.currentOrgId) return { name: "orgs" };
  return true;
});

export default router;
