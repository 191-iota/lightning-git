import { describe, expect, it } from "vitest";
import { AxiosError } from "axios";
import { getErrorMessage } from "./errorMessage";

function axiosErrorWith(data: unknown, status?: number, message = "Request failed"): AxiosError {
  const err = new AxiosError(message);
  err.response = {
    data,
    status: status ?? 500,
    statusText: "",
    headers: {},
    // axios requires a config object on the response; an empty one is fine here.
    config: {} as never,
  };
  return err;
}

describe("getErrorMessage", () => {
  it("returns a plain string response body, trimmed", () => {
    expect(getErrorMessage(axiosErrorWith("  boom  "))).toBe("boom");
  });

  it("prefers the body's error field for an object response", () => {
    expect(getErrorMessage(axiosErrorWith({ error: "nope" }))).toBe("nope");
  });

  it("stringifies an object response that has no error field", () => {
    expect(getErrorMessage(axiosErrorWith({ detail: "x" }))).toBe('{"detail":"x"}');
  });

  it("falls back to status + message when there is no usable response body", () => {
    const err = new AxiosError("network down");
    err.response = {
      data: undefined,
      status: 503,
      statusText: "",
      headers: {},
      config: {} as never,
    };
    expect(getErrorMessage(err)).toBe("Request failed with status 503: network down");
  });

  it("uses the axios message when there is no response at all", () => {
    expect(getErrorMessage(new AxiosError("timeout"))).toBe("timeout");
  });

  it("returns Error.message for a plain Error", () => {
    expect(getErrorMessage(new Error("kaboom"))).toBe("kaboom");
  });

  it("stringifies a non-Error value", () => {
    expect(getErrorMessage("just a string")).toBe("just a string");
    expect(getErrorMessage(123)).toBe("123");
    expect(getErrorMessage(null)).toBe("null");
  });
});
