import assert from "node:assert/strict";
import { config, validateConfig, boundedText } from "../src/config.ts";
globalThis.location = new URL("https://research.example/");
const original = structuredClone(config);
for (const input of [
  null,
  [],
  "invalid",
  { extra: 1 },
  { search: 1 },
  { tiles: "javascript:alert(1)" },
  { tiles: "//evil.example/map" },
  { tiles: "/\\evil.example/map" },
  { tiles: "https://user:secret@example.org/" },
  { tiles: "", attribution: "<svg onload=alert(1)>" },
]) {
  assert.throws(() => validateConfig(input));
  assert.deepEqual(
    config,
    original,
    "validation partially mutated live config",
  );
}
assert.equal(
  validateConfig({
    tiles: "/maps/{z}/{x}/{y}.png",
    search: "https://search.example/",
  }).tiles,
  "/maps/{z}/{x}/{y}.png",
);
assert.equal(await boundedText(new Response("é"), 2), "é");
await assert.rejects(boundedText(new Response("é"), 1), /size limit/);
let cancelled = false;
const stream = new ReadableStream({
  start(c) {
    c.enqueue(new Uint8Array(17));
  },
  cancel() {
    cancelled = true;
  },
});
await assert.rejects(boundedText(new Response(stream), 16), /size limit/);
assert(cancelled, "oversized response stream not cancelled");
console.log(
  "Untrusted provider configuration is validated transactionally; byte-bounded streaming cancels oversized responses.",
);
