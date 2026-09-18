/**
 * End-to-end proof that the Web SDK's queue/track/flush path works against
 * a real running stack:
 *
 *   track() -> localStorage-backed queue -> flush() -> POST /events
 *   -> Kafka -> worker -> GET /profiles/by-identity
 *
 * Runs in Node (no browser), so storage/fetch are injected explicitly —
 * exactly the seam `MessageBirdsWebAnalyticsOptions.storage`/`fetchImpl`
 * exist for. Point `MESSAGEBIRDS_API_URL` at the edge gateway
 * (`edge/ingest-gateway`, via `wrangler dev`) to also prove CORS/validation
 * forwarding works, or straight at `mb-api` to test the SDK alone.
 *
 * Run against a live stack: `docker compose up -d postgres redpanda`,
 * `cargo run -p api`, `cargo run -p worker`, then `npm run smoke-test`.
 */
import { randomUUID } from "node:crypto";
import type { ProfileView } from "@messagebirds/sdk";
import { InMemoryStorage, MessageBirdsWebAnalytics } from "../src/index.js";

const apiUrl = process.env.MESSAGEBIRDS_API_URL ?? "http://localhost:8080";
// The SDK only ever POSTs events through `apiUrl` (which may be the edge
// ingestion gateway — it proxies POST /events only). Reading the profile
// back to verify is a server-side/ops concern, same as `ui/`'s direct
// calls to `mb-api` — it always goes straight to the origin, not through
// the ingestion-only gateway.
const verifyApiUrl = process.env.MESSAGEBIRDS_VERIFY_API_URL ?? "http://localhost:8080";
const tenantId = randomUUID();

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function findProfileByIdentity(namespace: string, value: string): Promise<ProfileView | null> {
  const params = new URLSearchParams({ tenant_id: tenantId, namespace, value });
  const res = await fetch(`${verifyApiUrl}/profiles/by-identity?${params}`);
  if (res.status === 404) return null;
  if (!res.ok) throw new Error(`GET /profiles/by-identity failed: ${await res.text()}`);
  return (await res.json()) as ProfileView;
}

async function main() {
  const analytics = new MessageBirdsWebAnalytics({
    writeKey: tenantId,
    apiUrl,
    storage: new InMemoryStorage(),
    flushIntervalMs: 0, // this script flushes explicitly
  });

  const anonymousId = `anon-${randomUUID()}`;
  analytics.identify("anonymous_id", anonymousId);

  console.log(`[web-smoke-test] tenant=${tenantId} anonymous_id=${anonymousId} apiUrl=${apiUrl}`);

  analytics.track(
    "commerce.product_view",
    { product_id: "p1", category: "shoes", price: 79.99 },
    {
      schema: { name: "commerce.product_view", version: "1.0" },
      context: {
        profile_updates: {
          "core/person@1.0": { first_name: "Jane" },
          "core/contact@1.0": { email: "jane@example.com" },
        },
      },
    },
  );

  await analytics.flush();
  console.log("[web-smoke-test] flushed queue, waiting for the worker to project it...");

  const deadline = Date.now() + 15_000;
  let profile: ProfileView | null = null;
  while (Date.now() < deadline) {
    profile = await findProfileByIdentity("anonymous_id", anonymousId);
    if (profile) break;
    await sleep(500);
  }

  if (!profile) {
    throw new Error("timed out waiting for the profile to materialize — is the worker running?");
  }

  const mixinKeys = Object.keys(profile.mixins);
  const expected = ["core/identity@1.0", "core/person@1.0", "core/contact@1.0"];
  const missing = expected.filter((key) => !mixinKeys.includes(key));
  if (missing.length > 0) {
    throw new Error(`profile is missing expected mixins: ${missing.join(", ")}`);
  }

  console.log(`[web-smoke-test] OK — profile ${profile.id} composed ${mixinKeys.length} mixins`);
  analytics.destroy();
}

main().catch((err) => {
  console.error("[web-smoke-test] FAILED:", err);
  process.exit(1);
});
