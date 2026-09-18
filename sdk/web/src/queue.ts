import type { EventEnvelope } from "@messagebirds/sdk";

export interface QueuedEvent {
  envelope: EventEnvelope;
  attempts: number;
}

/** Matches the subset of `Storage` (window.localStorage) the queue needs. */
export interface QueueStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

const STORAGE_KEY = "messagebirds:queue";
const MAX_ATTEMPTS = 5;

/**
 * Backs the event queue against `localStorage` (or any `QueueStorage`) so
 * events survive a page reload between "queued" and "sent". `mb-api`'s
 * `POST /events` takes one envelope, not a batch — there is no batch
 * endpoint to send this queue to in one call, so `flush` sends serially,
 * one real POST per event.
 */
export class EventQueue {
  private readonly storage: QueueStorage;
  private readonly send: (envelope: EventEnvelope) => Promise<boolean>;
  private nextRetryAt = 0;

  constructor(storage: QueueStorage, send: (envelope: EventEnvelope) => Promise<boolean>) {
    this.storage = storage;
    this.send = send;
  }

  enqueue(envelope: EventEnvelope): void {
    const items = this.load();
    items.push({ envelope, attempts: 0 });
    this.persist(items);
  }

  /**
   * Sends queued events in order, oldest first. Stops at the first
   * failure (rather than reordering or hammering a possibly-down
   * endpoint) and schedules a backoff before the next flush is allowed to
   * attempt anything. An event that fails `MAX_ATTEMPTS` times is dropped
   * — logged via `onDropped`, not silently discarded.
   */
  async flush(onDropped?: (envelope: EventEnvelope) => void): Promise<void> {
    if (Date.now() < this.nextRetryAt) return;

    const items = this.load();
    let index = 0;
    while (index < items.length) {
      const item = items[index];
      const ok = await this.send(item.envelope);
      if (ok) {
        index += 1;
        continue;
      }

      item.attempts += 1;
      if (item.attempts >= MAX_ATTEMPTS) {
        onDropped?.(item.envelope);
        index += 1;
        continue;
      }

      this.nextRetryAt = Date.now() + backoffMs(item.attempts);
      this.persist(items.slice(index));
      return;
    }

    this.persist([]);
  }

  private load(): QueuedEvent[] {
    const raw = this.storage.getItem(STORAGE_KEY);
    if (!raw) return [];
    try {
      return JSON.parse(raw) as QueuedEvent[];
    } catch {
      return [];
    }
  }

  private persist(items: QueuedEvent[]): void {
    this.storage.setItem(STORAGE_KEY, JSON.stringify(items));
  }
}

function backoffMs(attempts: number): number {
  return Math.min(30_000, 1_000 * 2 ** (attempts - 1));
}
