import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import init, { BrowserEngine } from "../src/wasm/ranterra_engine.js";
const wasm = await init({
  module_or_path: await readFile("src/wasm/ranterra_engine_bg.wasm"),
});
const s = JSON.parse(await readFile("examples/paris.json", "utf8"));
s.radio = { model: "UMa" };
const g = s.gnodebs[0],
  u = s.ues[0];
s.gnodebs = Array.from({ length: 16 }, (_, i) => ({
  ...g,
  id: i + 1,
  height_m: 25,
  lon: g.lon + (i % 4) * 0.001,
  lat: g.lat + Math.floor(i / 4) * 0.001,
}));
s.ues = Array.from({ length: 128 }, (_, i) => ({
  ...u,
  id: i + 101,
  lon: u.lon + (i % 16) * 0.0001,
  lat: u.lat + Math.floor(i / 16) * 0.0001,
}));
const times = [],
  registration = [];
let snapshotBytes = 0;
for (let i = 0; i < 10; i++) {
  let t = performance.now();
  const engine = new BrowserEngine(JSON.stringify(s));
  times.push(performance.now() - t);
  t = performance.now();
  engine.advance(1000);
  registration.push(performance.now() - t);
  snapshotBytes = engine.snapshot().length;
  engine.free();
}
times.sort((a, b) => a - b);
registration.sort((a, b) => a - b);
console.log(
  JSON.stringify(
    {
      cells: 16,
      ues: 128,
      buildings: s.buildings.length,
      samples: 1225,
      runs: 10,
      rf_median_ms: times[5],
      rf_max_ms: times[9],
      registration_median_ms: registration[5],
      wasm_linear_memory_bytes: wasm.memory.buffer.byteLength,
      node_rss_bytes: process.memoryUsage().rss,
      snapshot_json_bytes: snapshotBytes,
    },
    null,
    2,
  ),
);

assert(
  wasm.memory.buffer.byteLength < 64 * 1024 * 1024,
  "WASM memory regression above 64 MiB for bounded workload",
);
assert(times.at(-1) < 5000, "RF initialization exceeds 5 second safety budget");
