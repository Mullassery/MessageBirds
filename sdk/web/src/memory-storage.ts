import type { QueueStorage } from "./queue.js";

/** Non-persistent `QueueStorage` for tests and non-browser environments. */
export class InMemoryStorage implements QueueStorage {
  private data = new Map<string, string>();

  getItem(key: string): string | null {
    return this.data.get(key) ?? null;
  }

  setItem(key: string, value: string): void {
    this.data.set(key, value);
  }
}
