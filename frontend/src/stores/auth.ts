import { defineStore } from "pinia";
import { computed, ref } from "vue";
import api, { onUnauthorized } from "@/services/api";
import { displayNameFromToken } from "@/utils/jwt";
import type {
  LoginPayload,
  LoginRes,
  RefreshRes,
  RegisterPayload,
  UpdateUsernameRes,
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
  // handle to show in the nav: the chosen username, with the email as fallback
  // for sessions where the name could not be read from the token.
  const displayName = computed(() => user.value?.display_name || user.value?.email || "");

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

  // single-flight refresh , multiple concurrent 401s share one in-flight call
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
    setSession(data.access_token, data.refresh_token, {
      id: data.user_id,
      email: data.email,
      display_name: displayNameFromToken(data.access_token),
    });
  }

  async function register(payload: RegisterPayload) {
    await api.post("/register", payload);
  }

  // Persist a new handle on the backend, then mirror it locally. The held
  // access token still carries the old name until the next refresh, so we keep
  // our own copy current instead of re-reading it from the token.
  async function changeUsername(username: string) {
    const { data } = await api.patch<UpdateUsernameRes>("/api/user/me/username", { username });
    if (user.value) {
      user.value = { ...user.value, display_name: data.display_name };
      localStorage.setItem(USER_KEY, JSON.stringify(user.value));
    }
    return data.display_name;
  }

  function logout() {
    clearAuth();
  }

  return {
    user,
    token,
    refreshToken,
    isAuthenticated,
    displayName,
    login,
    register,
    changeUsername,
    logout,
    refresh,
  };
});
