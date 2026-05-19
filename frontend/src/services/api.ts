import axios, { type AxiosRequestConfig, type InternalAxiosRequestConfig } from "axios";

const api = axios.create({
  baseURL: import.meta.env.VITE_API_BASE_URL || "http://localhost:8080",
  headers: {
    "Content-Type": "application/json",
  },
});

api.interceptors.request.use((config) => {
  const token = localStorage.getItem("token");
  if (token) {
    config.headers.Authorization = `Bearer ${token}`;
  }
  return config;
});

// the auth store registers a refresh callback that returns the new access
// token (or null). on 401 we try refresh once, then retry the original request.
// callback returns null -> we clear and bail.
type RefreshHandler = () => Promise<string | null>;
let refreshHandler: RefreshHandler | null = null;
export function onUnauthorized(handler: RefreshHandler) {
  refreshHandler = handler;
}

// flag added to retried requests so we dont infinite-loop on persistent 401
type RetriedConfig = InternalAxiosRequestConfig & { _retried?: boolean };

api.interceptors.response.use(
  (response) => response,
  async (error) => {
    const status = error.response?.status;
    const original = error.config as RetriedConfig | undefined;

    // dont try to refresh the refresh call itself
    const isRefreshCall = original?.url?.endsWith("/refresh");

    if (status === 401 && original && !original._retried && !isRefreshCall && refreshHandler) {
      original._retried = true;
      const fresh = await refreshHandler();
      if (fresh) {
        original.headers = original.headers || {};
        original.headers.Authorization = `Bearer ${fresh}`;
        return api.request(original as AxiosRequestConfig);
      }
      // refresh failed (token expired AND no usable refresh_token). auth was
      // cleared by the handler; bounce to login so the user isnt stuck on a
      // protected page they cant load.
      if (window.location.pathname !== "/login") {
        window.location.replace("/login");
      }
    }
    return Promise.reject(error);
  },
);

export default api;
