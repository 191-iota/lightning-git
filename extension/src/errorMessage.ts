import axios from "axios";

// Renders a human-readable message from an unknown thrown value. Prefers the
// backend's response body (string or { error }) for axios errors, falls back to
// status + message, then plain Error.message, then String(error).
export function getErrorMessage(error: unknown): string {
  if (axios.isAxiosError(error)) {
    const status = error.response?.status;
    const responseData = error.response?.data;
    if (typeof responseData === "string" && responseData.trim()) {
      return responseData.trim();
    }
    if (responseData && typeof responseData === "object") {
      if ("error" in responseData && responseData.error) {
        return String(responseData.error);
      }
      try {
        return JSON.stringify(responseData);
      } catch {
        return status ? `Request failed with status ${status}` : error.message;
      }
    }
    if (status) {
      return `Request failed with status ${status}: ${error.message}`;
    }
    return error.message;
  }
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}
