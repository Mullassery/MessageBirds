import { describe, expect, it } from "vitest";
import { isValidEnvelopeShape } from "../src/index.js";

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
