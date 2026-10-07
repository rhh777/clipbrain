import assert from "node:assert/strict";
import test from "node:test";
import { LruCache } from "../src/lib/lru-cache.ts";

test("recently used entries survive count-based eviction", () => {
  const cache = new LruCache<string, string>(2, 100, (value) => value.length);
  cache.set("a", "first");
  cache.set("b", "second");
  assert.equal(cache.get("a"), "first");
  cache.set("c", "third");
  assert.equal(cache.get("b"), undefined);
  assert.equal(cache.get("a"), "first");
  assert.equal(cache.get("c"), "third");
});

test("byte budget evicts entries even below the count limit", () => {
  const cache = new LruCache<string, string>(100, 8, (value) => value.length);
  cache.set("a", "12345");
  cache.set("b", "1234");
  assert.equal(cache.get("a"), undefined);
  assert.equal(cache.get("b"), "1234");
});

test("replacement releases its previous budget and oversized payloads are not retained", () => {
  const cache = new LruCache<string, string>(10, 8, (value) => value.length);
  cache.set("a", "123456");
  cache.set("a", "1");
  cache.set("b", "1234567");
  assert.equal(cache.get("a"), "1");
  assert.equal(cache.get("b"), "1234567");
  cache.set("a", "123456789");
  assert.equal(cache.get("a"), undefined);
  assert.equal(cache.get("b"), "1234567");
});
