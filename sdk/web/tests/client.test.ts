import { describe, expect, it } from "vitest";
import { PyCustomerHubWebAnalytics } from "../src/client.js";
import { InMemoryStorage } from "../src/memory-storage.js";

describe("PyCustomerHubWebAnalytics", () => {
  it("migrates the anonymous id from the pre-rebrand storage key", () => {
    const storage = new InMemoryStorage();
    storage.setItem("messagebirds:anonymous_id", "anon-legacy");

    const analytics = new PyCustomerHubWebAnalytics({
      writeKey: "tenant-1",
      apiUrl: "http://localhost:8080",
      storage,
      flushIntervalMs: 0,
      fetchImpl: async () => new Response(null, { status: 200 }),
    });
    analytics.destroy();

    expect(storage.getItem("pycustomerhub:anonymous_id")).toBe("anon-legacy");
  });
});
