import { readFile } from "node:fs/promises";
import assert from "node:assert/strict";
import init, { BrowserEngine } from "../src/wasm/ranterra_engine.js";
await init({
  module_or_path: await readFile(
    new URL("../src/wasm/ranterra_engine_bg.wasm", import.meta.url),
  ),
});
const scenario = {
  version: 1,
  name: "WASM verification",
  seed: 42,
  center: [2.36, 48.86],
  gnodebs: [
    {
      id: 1,
      name: "gNB",
      lon: 2.36,
      lat: 48.86,
      frequency_mhz: 3500,
      tx_power_dbm: 46,
      height_m: 25,
      gain_dbi: 16,
    },
  ],
  ues: [{ id: 101, name: "UE", lon: 2.361, lat: 48.86 }],
  buildings: [],
};
const a = new BrowserEngine(JSON.stringify(scenario)),
  b = new BrowserEngine(JSON.stringify(scenario));
a.advance(1000);
for (let i = 0; i < 100; i++) b.advance(10);
assert.deepEqual(JSON.parse(a.snapshot()), JSON.parse(b.snapshot()));
const result = JSON.parse(a.snapshot());
assert.equal(result.ues[0].state, "Registered");
assert(result.events.length >= 22);
assert(
  result.events.some(
    (e) => e.layer === "NGAP" && e.kind === "InitialUEMessage",
  ),
);
assert(
  result.events.some(
    (e) => e.kind === "Security Mode Complete" && e.hex.startsWith("7e 04"),
  ),
);
assert(
  result.events.some(
    (e) => e.kind === "Registration Complete" && e.hex.startsWith("7e 02"),
  ),
);
assert.equal(result.coverage.length, 1225);
assert.throws(() => new BrowserEngine('{"version":99}'));
a.session(101, "establish", 0);
a.session(101, "echo", 0);
a.advance(1000);
assert.equal(JSON.parse(a.snapshot()).sessions[0].delivered, 1);
a.session(101, "release", 0);
assert.equal(JSON.parse(a.snapshot()).sessions[0].active, false);
a.session(101, "deregister", 0);
assert.equal(JSON.parse(a.snapshot()).ues[0].state, "Deregistered");
a.free();
b.free();
console.log(
  "WASM runtime verified: NAS + NGAP binary round trip, authenticated registration, coverage, deterministic execution, invalid-input rejection.",
);
