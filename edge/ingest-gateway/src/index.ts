export interface Env {
  MESSAGEBIRDS_API_URL: string;
}

const ALLOWED_METHODS = "POST, OPTIONS";
const ALLOWED_HEADERS = "content-type";

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
 * `mb-api`) without re-implementing `mb-schema-registry`'s real
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
 * browser-based SDKs (`sdk/web`) hitting `mb-api` directly, which has no
 * CORS layer by design (every prior caller ran server-side — see
 * docs/ARCHITECTURE.md), and rejects malformed bodies before they reach
 * the origin. Deliberately thin: no API-key/write-key auth and no rate
 * limiting — real per-tenant write-key issuance doesn't exist anywhere in
 * this codebase yet, so there is nothing real to check here (a fake check
 * would be worse than the disclosed gap).
 */
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

    const upstream = await fetch(`${env.MESSAGEBIRDS_API_URL}/events`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(body),
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
