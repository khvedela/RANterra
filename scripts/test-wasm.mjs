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
a.advance(500);
for (let i = 0; i < 50; i++) b.advance(10);
assert.deepEqual(JSON.parse(a.snapshot()), JSON.parse(b.snapshot()));
const result = JSON.parse(a.snapshot());
assert.equal(result.ues[0].state, "RegistrationRejected");
assert.equal(result.events.length, 8);
assert(
  result.events.some(
    (e) => e.layer === "NGAP" && e.kind === "InitialUEMessage",
  ),
);
assert(result.events.some((e) => e.hex === "7e 00 44 07"));
assert.equal(result.coverage.length, 1225);
assert.throws(() => new BrowserEngine('{"version":99}'));
a.free();
b.free();
console.log(
  "WASM runtime verified: NAS + NGAP binary round trip, rejection, coverage, deterministic execution, invalid-input rejection.",
);
