const API_URL = process.env.MESSAGEBIRDS_API_URL ?? "http://localhost:8080";

export class ApiError extends Error {
  constructor(
    public readonly status: number,
    message: string,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

/**
 * Server-only fetch against `mb-api`. Every caller of this file is a
 * Server Component or Server Action — nothing in `ui` ever calls the Rust
 * API from the browser, which is what lets `mb-api` skip CORS entirely.
 */
export async function apiFetch<T>(path: string, init?: RequestInit): Promise<T | null> {
  const res = await fetch(`${API_URL}${path}`, {
    ...init,
    cache: "no-store",
    headers: { "content-type": "application/json", ...(init?.headers ?? {}) },
  });
  if (res.status === 404) return null;
  if (!res.ok) {
    throw new ApiError(res.status, await res.text());
  }
  return (await res.json()) as T;
}
