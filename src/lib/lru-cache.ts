/** Bound retained payloads by both entry count and estimated bytes. */
export class LruCache<K, V> {
  private entries = new Map<K, { value: V; bytes: number }>();
  private retainedBytes = 0;
  private readonly maxEntries: number;
  private readonly maxBytes: number;
  private readonly sizeOf: (value: V) => number;

  constructor(maxEntries: number, maxBytes: number, sizeOf: (value: V) => number) {
    this.maxEntries = maxEntries;
    this.maxBytes = maxBytes;
    this.sizeOf = sizeOf;
  }

  get(key: K): V | undefined {
    const entry = this.entries.get(key);
    if (!entry) return undefined;
    this.entries.delete(key);
    this.entries.set(key, entry);
    return entry.value;
  }

  set(key: K, value: V): void {
    const previous = this.entries.get(key);
    if (previous) {
      this.retainedBytes -= previous.bytes;
      this.entries.delete(key);
    }
    const bytes = this.sizeOf(value);
    // An oversized original can still be displayed, without retaining it in the cache.
    if (bytes > this.maxBytes || this.maxEntries <= 0) return;
    this.entries.set(key, { value, bytes });
    this.retainedBytes += bytes;
    while (this.entries.size > this.maxEntries || this.retainedBytes > this.maxBytes) {
      const oldest = this.entries.keys().next();
      if (oldest.done) break;
      this.retainedBytes -= this.entries.get(oldest.value)!.bytes;
      this.entries.delete(oldest.value);
    }
  }
}
