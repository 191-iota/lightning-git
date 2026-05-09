const JWT_LIKE = /\beyJ[a-zA-Z0-9_-]{10,}\.[a-zA-Z0-9._-]{10,}\.[a-zA-Z0-9._-]{10,}\b/g;

export function redact(input: string): string {
  return input.replace(JWT_LIKE, "***redacted-jwt***");
}