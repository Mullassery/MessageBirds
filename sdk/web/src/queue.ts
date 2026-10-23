import type { EventEnvelope } from "@pycustomerhub/sdk";

export interface QueuedEvent {
  envelope: EventEnvelope;
  attempts: number;
}

/** Matches the subset of `Storage` (window.localStorage) the queue needs. */
export interface QueueStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

export const QUEUE_STORAGE_KEY = "pycustomerhub:queue";
export const LEGACY_QUEUE_STORAGE_KEY = "messagebirds:queue";
const MAX_ATTEMPTS = 5;

/**
 * Backs the event queue against `localStorage` (or any `QueueStorage`) so
 * events survive a page reload between "queued" and "sent". `pch-api`'s
 * `POST /events` takes one envelope; this SDK does not use the
 * `/v1/batch` translation endpoint, so `flush` sends serially, one real
 * POST per event.
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
    const raw = this.storage.getItem(QUEUE_STORAGE_KEY);
    if (raw !== null) return parseQueue(raw);

    const legacy = this.storage.getItem(LEGACY_QUEUE_STORAGE_KEY);
    if (legacy === null) return [];

    const items = parseQueue(legacy);
    this.persist(items);
    this.storage.setItem(LEGACY_QUEUE_STORAGE_KEY, JSON.stringify([]));
    return items;
  }

  private persist(items: QueuedEvent[]): void {
    this.storage.setItem(QUEUE_STORAGE_KEY, JSON.stringify(items));
  }
}

function parseQueue(raw: string): QueuedEvent[] {
  try {
    return JSON.parse(raw) as QueuedEvent[];
  } catch {
    return [];
  }
}

function backoffMs(attempts: number): number {
  return Math.min(30_000, 1_000 * 2 ** (attempts - 1));
}
