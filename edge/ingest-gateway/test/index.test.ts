import { afterEach, describe, expect, it, vi } from "vitest";
import handler, { isValidEnvelopeShape, parseWriteKeys } from "../src/index.js";

const TENANT = "11111111-1111-4111-8111-111111111111";

function validEnvelope() {
  return {
    event_id: "evt-1",
    event_type: "commerce.product_view",
    timestamp: "2026-01-01T00:00:00Z",
    source: { type: "web", name: "website" },
    identity: [{ namespace: "anonymous_id", value: "anon-1", source: "web-sdk" }],
    schema: { name: "commerce.product_view", version: "1.0" },
    tenant_id: "tenant-1",
  };
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("isValidEnvelopeShape", () => {
  it("accepts a well-formed envelope", () => {
    expect(isValidEnvelopeShape(validEnvelope())).toBe(true);
  });

  it.each(["event_id", "event_type", "tenant_id", "schema", "identity"])(
    "rejects a body missing %s",
    (field) => {
      const body = validEnvelope() as Record<string, unknown>;
      delete body[field];
      expect(isValidEnvelopeShape(body)).toBe(false);
    },
  );

  it("rejects non-object bodies", () => {
    expect(isValidEnvelopeShape(null)).toBe(false);
    expect(isValidEnvelopeShape("a string")).toBe(false);
    expect(isValidEnvelopeShape(42)).toBe(false);
    expect(isValidEnvelopeShape([])).toBe(false);
  });

  it("rejects identity that isn't an array", () => {
    const body = { ...validEnvelope(), identity: { not: "an array" } };
    expect(isValidEnvelopeShape(body)).toBe(false);
  });
});

describe("parseWriteKeys", () => {
  it("parses a key -> tenant object", () => {
    expect(parseWriteKeys(`{"wk_live":"${TENANT}"}`)).toEqual({ wk_live: TENANT });
  });

  it("fails closed for missing, malformed, or empty mappings", () => {
    expect(parseWriteKeys(undefined)).toBeNull();
    expect(parseWriteKeys("not-json")).toBeNull();
    expect(parseWriteKeys("[]")).toBeNull();
    expect(parseWriteKeys("{}")).toBeNull();
    expect(parseWriteKeys('{"wk_live":"not-a-uuid"}')).toBeNull();
  });
});

describe("write-key auth", () => {
  it("fails closed when no write keys are configured", async () => {
    const request = new Request("https://gateway.example/events", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(validEnvelope()),
    });
    const response = await handler.fetch(request, { PYCUSTOMERHUB_API_URL: "http://api:8080" });
    expect(response.status).toBe(503);
  });

  it("rejects an unknown write key", async () => {
    const request = new Request("https://gateway.example/events", {
      method: "POST",
      headers: { "content-type": "application/json", "x-write-key": "nope" },
      body: JSON.stringify(validEnvelope()),
    });
    const response = await handler.fetch(request, {
      PYCUSTOMERHUB_API_URL: "http://api:8080",
      PYCUSTOMERHUB_EDGE_WRITE_KEYS: `{"wk_live":"${TENANT}"}`,
    });
    expect(response.status).toBe(401);
  });

  it("rewrites tenant_id from the validated header", async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      new Response("{}", { status: 200, headers: { "content-type": "application/json" } }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const body = { ...validEnvelope(), tenant_id: "untrusted" };
    const request = new Request("https://gateway.example/events", {
      method: "POST",
      headers: { "content-type": "application/json", "x-write-key": "wk_live" },
      body: JSON.stringify(body),
    });
    const response = await handler.fetch(request, {
      PYCUSTOMERHUB_API_URL: "http://api:8080",
      PYCUSTOMERHUB_EDGE_WRITE_KEYS: `{"wk_live":"${TENANT}"}`,
    });

    expect(response.status).toBe(200);
    const forwarded = JSON.parse(fetchMock.mock.calls[0][1].body);
    expect(forwarded.tenant_id).toBe(TENANT);
  });

  it("accepts the legacy body field as the edge credential", async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      new Response("{}", { status: 200, headers: { "content-type": "application/json" } }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const request = new Request("https://gateway.example/events", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ ...validEnvelope(), tenant_id: "wk_live" }),
    });
    const response = await handler.fetch(request, {
      PYCUSTOMERHUB_API_URL: "http://api:8080",
      PYCUSTOMERHUB_EDGE_WRITE_KEYS: `{"wk_live":"${TENANT}"}`,
    });

    expect(response.status).toBe(200);
    const forwarded = JSON.parse(fetchMock.mock.calls[0][1].body);
    expect(forwarded.tenant_id).toBe(TENANT);
  });
});

describe("env fallback", () => {
  it("forwards through the legacy upstream binding when the new one is unset", async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      new Response("{}", { status: 200, headers: { "content-type": "application/json" } }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const request = new Request("https://gateway.example/events", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(validEnvelope()),
    });
    const response = await handler.fetch(request, {
      MESSAGEBIRDS_API_URL: "http://legacy:8080",
      PYCUSTOMERHUB_EDGE_WRITE_KEYS: `{"tenant-1":"${TENANT}"}`,
    });

    expect(response.status).toBe(200);
    expect(fetchMock).toHaveBeenCalledWith(
      "http://legacy:8080/events",
      expect.objectContaining({ method: "POST" }),
    );
  });
});
