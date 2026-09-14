/**
 * End-to-end proof that the foundation works:
 *
 *   POST /events -> Kafka -> worker (schema validate, identity resolve,
 *   merge policy, profile projection) -> GET /profiles/by-identity
 *
 * Run against a live stack: `docker compose up -d postgres redpanda`,
 * `cargo run -p api`, `cargo run -p worker`, then `npm run smoke-test`.
 */
import { randomUUID } from "node:crypto";
import { MessageBirdsClient } from "../src/client.js";
import type { EventEnvelope } from "../src/types.js";

const baseUrl = process.env.MESSAGEBIRDS_API_URL ?? "http://localhost:8080";
const client = new MessageBirdsClient({ baseUrl });

const tenantId = randomUUID();
const anonymousId = `anon-${randomUUID()}`;

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function main() {
  console.log(`[smoke-test] tenant=${tenantId} anonymous_id=${anonymousId}`);

  const envelope: EventEnvelope = {
    event_id: randomUUID(),
    event_type: "commerce.product_view",
    timestamp: new Date().toISOString(),
    source: { type: "web", name: "website" },
    identity: [
      {
        namespace: "anonymous_id",
        value: anonymousId,
        primary: true,
        source: "smoke-test",
      },
    ],
    schema: { name: "commerce.product_view", version: "1.0" },
    data: { product_id: "p1", category: "shoes", price: 79.99 },
    context: {
      profile_updates: {
        "core/person@1.0": { first_name: "Jane" },
        "core/contact@1.0": { email: "jane@example.com" },
        "core/device@1.0": { device_id: "device-1", platform: "web" },
      },
    },
    tenant_id: tenantId,
  };

  const { event_id } = await client.sendEvent(envelope);
  console.log(`[smoke-test] sent event ${event_id}, waiting for the worker to project it...`);

  const deadline = Date.now() + 15_000;
  let profile = null;
  while (Date.now() < deadline) {
    profile = await client.findProfileByIdentity(tenantId, "anonymous_id", anonymousId);
    if (profile) break;
    await sleep(500);
  }

  if (!profile) {
    throw new Error("timed out waiting for the profile to materialize — is the worker running?");
  }

  console.log("[smoke-test] profile:");
  console.log(JSON.stringify(profile, null, 2));

  const mixinKeys = Object.keys(profile.mixins);
  const expected = ["core/identity@1.0", "core/person@1.0", "core/contact@1.0", "core/device@1.0"];
  const missing = expected.filter((key) => !mixinKeys.includes(key));
  if (missing.length > 0) {
    throw new Error(`profile is missing expected mixins: ${missing.join(", ")}`);
  }
  if (profile.provenance.length === 0) {
    throw new Error("profile has no field provenance — lineage seed did not get written");
  }

  console.log(
    `[smoke-test] OK — profile ${profile.id} composed ${mixinKeys.length} mixins with ${profile.provenance.length} provenance records`,
  );
}

main().catch((err) => {
  console.error("[smoke-test] FAILED:", err);
  process.exit(1);
});
