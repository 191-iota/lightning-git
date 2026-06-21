import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AxiosResponse, InternalAxiosRequestConfig } from "axios";
import api, { onUnauthorized } from "./api";

// We exercise the real axios instance and its real interceptors by swapping in
// a custom adapter (axios's public transport hook) instead of the network. The
// adapter records every outgoing config so we can assert what the interceptors
// produced, and it can be scripted to return a 401 to drive the refresh-retry
// path. The request URLs/params are not asserted - only the contractual
// behaviour: the bearer header, refresh-once, retry-once, and never refreshing
// the refresh call itself.

interface Recorded {
  url?: string;
  authorization?: unknown;
}

let calls: Recorded[];
let scripted: (config: InternalAxiosRequestConfig, callIndex: number) => AxiosResponse;
const realLocation = window.location;

function okResponse(config: InternalAxiosRequestConfig): AxiosResponse {
  return { data: {}, status: 200, statusText: "OK", headers: {}, config };
}

function throw401(config: InternalAxiosRequestConfig): never {
  const error = new Error("Request failed with status code 401") as Error & {
    config: InternalAxiosRequestConfig;
    response: { status: number; data: unknown; headers: object; config: InternalAxiosRequestConfig };
  };
  error.config = config;
  error.response = { status: 401, data: {}, headers: {}, config };
  throw error;
}

beforeEach(() => {
  localStorage.clear();
  calls = [];
  api.defaults.adapter = async (config) => {
    const index = calls.length;
    calls.push({ url: config.url, authorization: config.headers?.Authorization });
    return scripted(config, index);
  };
  // default handler is a passthrough; individual tests override as needed
  onUnauthorized(async () => null);
});

afterEach(() => {
  vi.restoreAllMocks();
  Object.defineProperty(window, "location", { configurable: true, value: realLocation });
});

describe("request interceptor", () => {
  it("attaches the bearer token from storage to every request", async () => {
    localStorage.setItem("token", "abc");
    scripted = (config) => okResponse(config);

    await api.get("/api/things");

    expect(calls).toHaveLength(1);
    expect(calls[0].authorization).toBe("Bearer abc");
  });

  it("sends no Authorization header when no token is stored", async () => {
    scripted = (config) => okResponse(config);

    await api.get("/api/public");

    expect(calls[0].authorization).toBeUndefined();
  });
});

describe("401 response interceptor", () => {
  it("refreshes once and retries the original request with the new token", async () => {
    localStorage.setItem("token", "stale");
    onUnauthorized(async () => {
      localStorage.setItem("token", "fresh");
      return "fresh";
    });
    scripted = (config, index) => (index === 0 ? throw401(config) : okResponse(config));

    const res = await api.get("/api/secure");

    expect(res.status).toBe(200);
    expect(calls).toHaveLength(2);
    expect(calls[1].authorization).toBe("Bearer fresh");
  });

  it("never tries to refresh the refresh call itself", async () => {
    const handler = vi.fn(async () => "fresh");
    onUnauthorized(handler);
    scripted = (config) => throw401(config);

    await expect(api.post("/refresh", {})).rejects.toBeTruthy();

    expect(handler).not.toHaveBeenCalled();
    expect(calls).toHaveLength(1);
  });

  it("retries at most once, even if the retried request also 401s", async () => {
    localStorage.setItem("token", "stale");
    onUnauthorized(async () => "fresh");
    scripted = (config) => throw401(config);

    await expect(api.get("/api/secure")).rejects.toBeTruthy();

    // original + exactly one retry; the _retried flag stops a second refresh
    expect(calls).toHaveLength(2);
  });

  it("redirects to login and does not retry when refresh yields no token", async () => {
    localStorage.setItem("token", "stale");
    const replace = vi.fn();
    Object.defineProperty(window, "location", {
      configurable: true,
      value: { pathname: "/dashboard", replace },
    });
    onUnauthorized(async () => null);
    scripted = (config) => throw401(config);

    await expect(api.get("/api/secure")).rejects.toBeTruthy();

    expect(calls).toHaveLength(1);
    expect(replace).toHaveBeenCalledWith("/login");
  });
});
