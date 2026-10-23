export interface Env {
  PYCUSTOMERHUB_API_URL?: string;
  /** JSON object mapping write keys to tenant UUIDs. */
  PYCUSTOMERHUB_EDGE_WRITE_KEYS?: string;
  /** Deprecated pre-rebrand binding, still accepted during migration. */
  MESSAGEBIRDS_API_URL?: string;
}

const ALLOWED_METHODS = "POST, OPTIONS";
const ALLOWED_HEADERS = "content-type, x-write-key, authorization";

function corsHeaders(origin: string | null): HeadersInit {
  return {
    "Access-Control-Allow-Origin": origin ?? "*",
    "Access-Control-Allow-Methods": ALLOWED_METHODS,
    "Access-Control-Allow-Headers": ALLOWED_HEADERS,
    Vary: "Origin",
  };
}

/**
 * Cheap shape validation before forwarding to the origin — enough to
 * reject obviously-malformed bodies at the edge (400, no round trip to
 * `pch-api`) without re-implementing `pch-schema-registry`'s real
 * validation, which still runs in the worker pipeline as the source of
 * truth.
 */
export function isValidEnvelopeShape(body: unknown): body is Record<string, unknown> {
  if (typeof body !== "object" || body === null) return false;
  const b = body as Record<string, unknown>;
  return (
    typeof b.event_id === "string" &&
    typeof b.event_type === "string" &&
    typeof b.tenant_id === "string" &&
    typeof b.schema === "object" &&
    b.schema !== null &&
    Array.isArray(b.identity)
  );
}

/**
 * Edge ingestion gateway (Phase 6, Track D): terminates CORS for
 * browser-based SDKs (`sdk/web`), validates a write key against
 * `PYCUSTOMERHUB_EDGE_WRITE_KEYS`, rewrites `tenant_id` to the mapped
 * tenant, and rejects malformed bodies before they reach the origin.
 * The source fails closed when no mapping is configured; there is no
 * unauthenticated fallback. Rate limiting remains a separate concern.
 */
function isUuid(value: string): boolean {
  return /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(value);
}

export function parseWriteKeys(raw: string | undefined): Record<string, string> | null {
  if (!raw) return null;
  try {
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) return null;
    const keys: Record<string, string> = {};
    for (const [key, tenant] of Object.entries(parsed)) {
      const trimmedKey = key.trim();
      const trimmedTenant = typeof tenant === "string" ? tenant.trim() : "";
      if (!trimmedKey || !isUuid(trimmedTenant)) continue;
      keys[trimmedKey] = trimmedTenant;
    }
    return Object.keys(keys).length > 0 ? keys : null;
  } catch {
    return null;
  }
}

function writeKeyFromRequest(request: Request, body: Record<string, unknown>): string {
  const header = request.headers.get("x-write-key")?.trim();
  if (header) return header;
  const authorization = request.headers.get("Authorization");
  if (authorization?.toLowerCase().startsWith("bearer ")) {
    return authorization.slice(7).trim();
  }
  // The browser SDK historically treats its write key as tenant_id. Accept
  // that field as the edge credential and rewrite it after validation.
  return typeof body.tenant_id === "string" ? body.tenant_id.trim() : "";
}
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const origin = request.headers.get("Origin");

    if (request.method === "OPTIONS") {
      return new Response(null, { status: 204, headers: corsHeaders(origin) });
    }

    const url = new URL(request.url);
    if (url.pathname !== "/events" || request.method !== "POST") {
      return new Response(JSON.stringify({ error: "not found" }), {
        status: 404,
        headers: { "content-type": "application/json", ...corsHeaders(origin) },
      });
    }

    const apiUrl = env.PYCUSTOMERHUB_API_URL ?? env.MESSAGEBIRDS_API_URL;
    if (!apiUrl) {
      return new Response(JSON.stringify({ error: "upstream API URL is not configured" }), {
        status: 503,
        headers: { "content-type": "application/json", ...corsHeaders(origin) },
      });
    }

    const writeKeys = parseWriteKeys(env.PYCUSTOMERHUB_EDGE_WRITE_KEYS);
    if (!writeKeys) {
      return new Response(
        JSON.stringify({ error: "write-key auth is not configured" }),
        {
          status: 503,
          headers: { "content-type": "application/json", ...corsHeaders(origin) },
        },
      );
    }

    let body: unknown;
    try {
      body = await request.json();
    } catch {
      return new Response(JSON.stringify({ error: "invalid JSON body" }), {
        status: 400,
        headers: { "content-type": "application/json", ...corsHeaders(origin) },
      });
    }

    if (!isValidEnvelopeShape(body)) {
      return new Response(JSON.stringify({ error: "malformed event envelope" }), {
        status: 400,
        headers: { "content-type": "application/json", ...corsHeaders(origin) },
      });
    }

    const writeKey = writeKeyFromRequest(request, body);
    const tenantId = writeKey ? writeKeys[writeKey] : undefined;
    if (!tenantId) {
      return new Response(JSON.stringify({ error: "missing or invalid write key" }), {
        status: 401,
        headers: { "content-type": "application/json", ...corsHeaders(origin) },
      });
    }

    const forwarded = { ...body, tenant_id: tenantId };
    const upstream = await fetch(`${apiUrl}/events`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(forwarded),
    });

    const responseBody = await upstream.text();
    return new Response(responseBody, {
      status: upstream.status,
      headers: {
        "content-type": upstream.headers.get("content-type") ?? "application/json",
        ...corsHeaders(origin),
      },
    });
  },
};
