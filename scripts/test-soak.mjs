// Bounded real-WASM lifecycle soak. Linear-memory high-water marks are not browser RSS.
import assert from "node:assert/strict";
import { readFile, mkdir, writeFile } from "node:fs/promises";
import init, { BrowserEngine } from "../src/wasm/ranterra_engine.js";
const wasm = await init({
  module_or_path: await readFile("src/wasm/ranterra_engine_bg.wasm"),
});
const input = await readFile("examples/minimal.json", "utf8");
const id = JSON.parse(input).ues[0].id;
const cycles = Number(process.env.RANTERRA_SOAK_CYCLES ?? 128);
assert(
  Number.isInteger(cycles) && cycles >= 128 && cycles <= 16384,
  "Soak cycles must be 128–16384",
);
const start = performance.now();
let warmMemory = 0;
let maxMemory = 0;
for (let cycle = 0; cycle < cycles; cycle++) {
  const run = new BrowserEngine(input);
  try {
    run.advance(1000);
    run.session(id, "establish", 0);
    run.advance(100);
    for (let packet = 0; packet < 20; packet++)
      run.session(id, "echo", packet % 2 ? 100 : 0);
    if (cycle % 2) run.advance(1000);
    run.session(id, "release", 0);
    run.advance(100);
    run.session(id, "deregister", 0);
    run.advance(1000);
    const snapshot = JSON.parse(run.snapshot());
    assert.equal(snapshot.pending, 0);
    assert.equal(snapshot.sessions[0].active, false);
    assert.equal(snapshot.sessions[0].sent, 20);
    assert.equal(
      snapshot.sessions[0].delivered + snapshot.sessions[0].dropped,
      20,
    );
    assert.equal(snapshot.ues[0].state, "Deregistered");
    const restored = new BrowserEngine(run.recording());
    try {
      assert.equal(restored.snapshot(), run.snapshot());
    } finally {
      restored.free();
    }
    // Rejected candidate loads must not damage the current instance or retain large allocations.
    assert.throws(() => new BrowserEngine('{"version":99}'));
    assert.equal(JSON.parse(run.snapshot()).ues[0].state, "Deregistered");
  } finally {
    run.free();
  }
  const memory = wasm.memory.buffer.byteLength;
  if (cycle === 31) warmMemory = memory;
  maxMemory = Math.max(maxMemory, memory);
}
assert(maxMemory < 64 * 1024 * 1024, "WASM linear memory exceeds 64 MiB bound");
assert(
  maxMemory - warmMemory <= 4 * 1024 * 1024,
  "Unexpected post-warmup linear-memory growth",
);
const result = {
  workload: `${cycles} create/register/session/20-echo/release/deregister/replay/free cycles`,
  cycles,
  packets: cycles * 20,
  warmup_cycles: 32,
  elapsed_wall_ms: performance.now() - start,
  wasm_warmup_linear_bytes: warmMemory,
  wasm_max_linear_bytes: maxMemory,
  post_warmup_growth_bytes: maxMemory - warmMemory,
  node_process_rss_bytes: process.memoryUsage().rss,
};
await mkdir("test-results", { recursive: true });
await writeFile(
  "test-results/wasm-soak.json",
  JSON.stringify(result, null, 2) + "\n",
);
console.log(JSON.stringify(result, null, 2));
