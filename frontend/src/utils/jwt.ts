// Supabase access tokens carry the user's metadata (including the display_name
// chosen at signup) in the JWT payload. Reading it client-side lets us show the
// current handle without a round-trip. We only read a non-sensitive claim; the
// token is still validated server-side on every request.

function decodePayload(token: string): Record<string, unknown> | null {
  const parts = token.split(".");
  if (parts.length !== 3) return null;
  try {
    const base64 = parts[1].replace(/-/g, "+").replace(/_/g, "/");
    const padded = base64.padEnd(base64.length + ((4 - (base64.length % 4)) % 4), "=");
    const json = decodeURIComponent(
      atob(padded)
        .split("")
        .map((c) => "%" + c.charCodeAt(0).toString(16).padStart(2, "0"))
        .join(""),
    );
    return JSON.parse(json) as Record<string, unknown>;
  } catch {
    return null;
  }
}

/** Best-effort extraction of the display_name from a Supabase access token. */
export function displayNameFromToken(token: string): string | undefined {
  const payload = decodePayload(token);
  const meta = payload?.["user_metadata"];
  if (meta && typeof meta === "object") {
    const name = (meta as Record<string, unknown>)["display_name"];
    if (typeof name === "string" && name.length > 0) return name;
  }
  return undefined;
}
