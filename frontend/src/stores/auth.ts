import { defineStore } from "pinia";
import { computed, ref } from "vue";
import api, { onUnauthorized } from "@/services/api";
import type {
  LoginPayload,
  LoginRes,
  RefreshRes,
  RegisterPayload,
  User,
} from "@/types/api";

export const useAuthStore = defineStore("auth", () => {
  const TOKEN_KEY = "token";
  const REFRESH_KEY = "refreshToken";
  const USER_KEY = "user";

  const storedUser = localStorage.getItem(USER_KEY);
  const user = ref<User | null>(storedUser ? (JSON.parse(storedUser) as User) : null);
  const token = ref<string | null>(localStorage.getItem(TOKEN_KEY));
  const refreshToken = ref<string | null>(localStorage.getItem(REFRESH_KEY));

  const isAuthenticated = computed(() => !!token.value);

  function setSession(access: string, refresh: string, newUser: User) {
    token.value = access;
    refreshToken.value = refresh;
    user.value = newUser;
    localStorage.setItem(TOKEN_KEY, access);
    localStorage.setItem(REFRESH_KEY, refresh);
    localStorage.setItem(USER_KEY, JSON.stringify(newUser));
  }

  function setTokens(access: string, refresh: string) {
    token.value = access;
    refreshToken.value = refresh;
    localStorage.setItem(TOKEN_KEY, access);
    localStorage.setItem(REFRESH_KEY, refresh);
  }

  function clearAuth() {
    user.value = null;
    token.value = null;
    refreshToken.value = null;
    localStorage.removeItem(TOKEN_KEY);
    localStorage.removeItem(REFRESH_KEY);
    localStorage.removeItem(USER_KEY);
  }

  // single-flight refresh — multiple concurrent 401s share one in-flight call
  let inFlight: Promise<string | null> | null = null;
  async function refresh(): Promise<string | null> {
    if (inFlight) return inFlight;
    // no refresh_token means we cant recover. clear the stale access token
    // so the router guard sees us as logged out and lets /login render.
    if (!refreshToken.value) {
      clearAuth();
      return null;
    }
    inFlight = (async () => {
      try {
        const { data } = await api.post<RefreshRes>("/refresh", {
          refresh_token: refreshToken.value,
        });
        setTokens(data.access_token, data.refresh_token);
        return data.access_token;
      } catch {
        clearAuth();
        return null;
      } finally {
        inFlight = null;
      }
    })();
    return inFlight;
  }

  onUnauthorized(refresh);

  async function login(credentials: LoginPayload) {
    const { data } = await api.post<LoginRes>("/login", credentials);
    setSession(data.access_token, data.refresh_token, { id: data.user_id, email: data.email });
  }

  async function register(payload: RegisterPayload) {
    await api.post("/register", payload);
  }

  function logout() {
    clearAuth();
  }

  return {
    user,
    token,
    refreshToken,
    isAuthenticated,
    login,
    register,
    logout,
    refresh,
  };
});
