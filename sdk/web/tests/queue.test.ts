import { describe, expect, it, vi } from "vitest";
import type { EventEnvelope } from "@messagebirds/sdk";
import { EventQueue } from "../src/queue.js";
import { InMemoryStorage } from "../src/memory-storage.js";

function envelope(id: string): EventEnvelope {
  return {
    event_id: id,
    event_type: "test.event",
    timestamp: new Date().toISOString(),
    source: { type: "web", name: "test" },
    identity: [],
    schema: { name: "test.event", version: "1.0" },
    tenant_id: "tenant-1",
  };
}

describe("EventQueue", () => {
  it("sends a single queued event and clears it on success", async () => {
    const storage = new InMemoryStorage();
    const send = vi.fn().mockResolvedValue(true);
    const queue = new EventQueue(storage, send);

    queue.enqueue(envelope("a"));
    await queue.flush();

    expect(send).toHaveBeenCalledTimes(1);
    expect(JSON.parse(storage.getItem("messagebirds:queue")!)).toEqual([]);
  });

  it("sends events in order and stops at every currently-persisted item once each succeeds", async () => {
    const storage = new InMemoryStorage();
    const sentIds: string[] = [];
    const send = vi.fn(async (e: EventEnvelope) => {
      sentIds.push(e.event_id);
      return true;
    });
    const queue = new EventQueue(storage, send);

    queue.enqueue(envelope("a"));
    queue.enqueue(envelope("b"));
    queue.enqueue(envelope("c"));
    await queue.flush();

    expect(sentIds).toEqual(["a", "b", "c"]);
  });

  it("stops at the first failure and keeps failed-and-later events queued", async () => {
    const storage = new InMemoryStorage();
    const send = vi
      .fn()
      .mockResolvedValueOnce(true) // a succeeds
      .mockResolvedValueOnce(false); // b fails
    const queue = new EventQueue(storage, send);

    queue.enqueue(envelope("a"));
    queue.enqueue(envelope("b"));
    queue.enqueue(envelope("c"));
    await queue.flush();

    expect(send).toHaveBeenCalledTimes(2); // never attempted c
    const remaining = JSON.parse(storage.getItem("messagebirds:queue")!);
    expect(remaining.map((i: { envelope: EventEnvelope }) => i.envelope.event_id)).toEqual(["b", "c"]);
    expect(remaining[0].attempts).toBe(1);
  });

  it("does not retry again before the backoff window elapses", async () => {
    const storage = new InMemoryStorage();
    const send = vi.fn().mockResolvedValue(false);
    const queue = new EventQueue(storage, send);

    queue.enqueue(envelope("a"));
    await queue.flush(); // attempt 1, fails, schedules backoff
    await queue.flush(); // called immediately after — should be a no-op

    expect(send).toHaveBeenCalledTimes(1);
  });

  it("drops an event after MAX_ATTEMPTS failures and reports it via onDropped", async () => {
    const storage = new InMemoryStorage();
    const send = vi.fn().mockResolvedValue(false);
    const queue = new EventQueue(storage, send);
    const dropped: EventEnvelope[] = [];

    queue.enqueue(envelope("a"));

    for (let i = 0; i < 5; i++) {
      // bypass the real backoff delay between attempts for the test
      (queue as unknown as { nextRetryAt: number }).nextRetryAt = 0;
      await queue.flush((e) => dropped.push(e));
    }

    expect(dropped).toHaveLength(1);
    expect(dropped[0].event_id).toBe("a");
    expect(JSON.parse(storage.getItem("messagebirds:queue")!)).toEqual([]);
  });
});
