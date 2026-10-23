import type { EventEnvelope, IdentityRef, SchemaRef } from "@pycustomerhub/sdk";
import {
  EventQueue,
  LEGACY_QUEUE_STORAGE_KEY,
  QUEUE_STORAGE_KEY,
  type QueueStorage,
} from "./queue.js";

export interface PyCustomerHubWebAnalyticsOptions {
  /** The tenant to send events under. No real per-tenant write-key issuance
   * exists yet (see docs/ARCHITECTURE.md) — this is the tenant_id itself
   * until that's built. */
  writeKey: string;
  apiUrl: string;
  flushIntervalMs?: number;
  /** Defaults to `window.localStorage`. Inject an in-memory implementation
   * in non-browser environments (tests, SSR). */
  storage?: QueueStorage;
  fetchImpl?: typeof fetch;
}

const ANON_ID_KEY = "pycustomerhub:anonymous_id";
const LEGACY_ANON_ID_KEY = "messagebirds:anonymous_id";

export class PyCustomerHubWebAnalytics {
  private readonly tenantId: string;
  private readonly apiUrl: string;
  private readonly storage: QueueStorage;
  private readonly fetchImpl: typeof fetch;
  private readonly queue: EventQueue;
  private readonly anonymousId: string;
  private identity: IdentityRef[] = [];
  private flushing = false;
  private timer: ReturnType<typeof setInterval> | null = null;

  constructor(options: PyCustomerHubWebAnalyticsOptions) {
    this.tenantId = options.writeKey;
    this.apiUrl = options.apiUrl.replace(/\/$/, "");
    this.storage = options.storage ?? window.localStorage;
    this.fetchImpl = options.fetchImpl ?? fetch.bind(globalThis);
    this.anonymousId = this.loadOrCreateAnonymousId();
    this.queue = new EventQueue(this.storage, (envelope) => this.sendOnce(envelope));

    if (typeof document !== "undefined") {
      document.addEventListener("visibilitychange", () => {
        if (document.visibilityState === "hidden") void this.flush();
      });
    }
    if (typeof window !== "undefined") {
      window.addEventListener("pagehide", () => this.flushBeacon());
    }

    const interval = options.flushIntervalMs ?? 5000;
    if (interval > 0 && typeof setInterval !== "undefined") {
      this.timer = setInterval(() => void this.flush(), interval);
    }
  }

  /** Stops the flush timer and removes window/document listeners this instance added. */
  destroy(): void {
    if (this.timer) clearInterval(this.timer);
  }

  identify(namespace: string, value: string): void {
    this.identity = [{ namespace, value, primary: true, source: "web-sdk" }];
  }

  track(
    eventType: string,
    data: Record<string, unknown> = {},
    options?: { schema?: SchemaRef; context?: Record<string, unknown> },
  ): void {
    const identity: IdentityRef[] =
      this.identity.length > 0
        ? this.identity
        : [{ namespace: "anonymous_id", value: this.anonymousId, primary: true, source: "web-sdk" }];

    const envelope: EventEnvelope = {
      event_id: crypto.randomUUID(),
      event_type: eventType,
      timestamp: new Date().toISOString(),
      source: { type: "web", name: "web-sdk" },
      identity,
      data,
      schema: options?.schema ?? { name: eventType, version: "1.0" },
      context: options?.context,
      tenant_id: this.tenantId,
    };

    this.queue.enqueue(envelope);
    void this.flush();
  }

  async flush(): Promise<void> {
    if (this.flushing) return;
    this.flushing = true;
    try {
      await this.queue.flush((dropped) => {
        console.warn(`[pycustomerhub] dropped event ${dropped.event_id} after repeated failures`);
      });
    } finally {
      this.flushing = false;
    }
  }

  private async sendOnce(envelope: EventEnvelope): Promise<boolean> {
    try {
      const res = await this.fetchImpl(`${this.apiUrl}/events`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(envelope),
      });
      return res.ok;
    } catch {
      return false;
    }
  }

  /**
   * `navigator.sendBeacon` is fire-and-forget — its return value only
   * means the browser accepted the request for background delivery, not
   * that the server received it. That's the standard, accepted tradeoff
   * every beacon-based unload flush makes (there's no synchronous
   * alternative that reliably survives page teardown); events sent this
   * way are optimistically cleared from the persisted queue.
   */
  private flushBeacon(): void {
    if (typeof navigator === "undefined" || !navigator.sendBeacon) return;
    const raw =
      this.storage.getItem(QUEUE_STORAGE_KEY) ??
      this.storage.getItem(LEGACY_QUEUE_STORAGE_KEY);
    if (!raw) return;
    let items: { envelope: EventEnvelope }[];
    try {
      items = JSON.parse(raw);
    } catch {
      return;
    }
    for (const item of items) {
      navigator.sendBeacon(`${this.apiUrl}/events`, JSON.stringify(item.envelope));
    }
    this.storage.setItem(QUEUE_STORAGE_KEY, JSON.stringify([]));
    this.storage.setItem(LEGACY_QUEUE_STORAGE_KEY, JSON.stringify([]));
  }

  private loadOrCreateAnonymousId(): string {
    const existing = this.storage.getItem(ANON_ID_KEY);
    if (existing) return existing;

    const legacy = this.storage.getItem(LEGACY_ANON_ID_KEY);
    if (legacy) {
      this.storage.setItem(ANON_ID_KEY, legacy);
      return legacy;
    }

    const id = `anon-${crypto.randomUUID()}`;
    this.storage.setItem(ANON_ID_KEY, id);
    return id;
  }
}
