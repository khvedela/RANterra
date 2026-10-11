import { mkdtemp, readFile, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { execFileSync } from "node:child_process";
import assert from "node:assert/strict";
import init, { BrowserEngine } from "../src/wasm/ranterra_engine.js";
await init({
  module_or_path: await readFile("src/wasm/ranterra_engine_bg.wasm"),
});
const dir = await mkdtemp(join(tmpdir(), "ranterra-replay-"));
let wasm, restored;
try {
  const scenario = JSON.parse(await readFile("examples/minimal.json", "utf8"));
  const service = {
    dnn: "research.example",
    snssai: { sst: 2, sd: [1, 2, 3] },
  };
  scenario.session_policies = [{ request: service, max_sessions: 128 }];
  scenario.ues[0].session = service;
  wasm = new BrowserEngine(JSON.stringify(scenario));
  wasm.advance(1000);
  wasm.session(101, "establish", 0);
  wasm.session(101, "echo", 0);
  const input = join(dir, "input.json"),
    output = join(dir, "output.json");
  await writeFile(input, wasm.recording());
  // Native reconstruction must verify the exact WASM protocol/state digest and continue.
  const native = JSON.parse(
    execFileSync(
      "cargo",
      [
        "run",
        "--quiet",
        "--locked",
        "-p",
        "ranterra-engine",
        "--example",
        "experiment",
        "--",
        input,
        "100",
        output,
      ],
      { encoding: "utf8", maxBuffer: 10_000_000 },
    ),
  );
  wasm.advance(100);
  assert.deepEqual(native.events, JSON.parse(wasm.snapshot()).events);
  restored = new BrowserEngine(await readFile(output, "utf8"));
  assert.deepEqual(
    JSON.parse(restored.recording()),
    JSON.parse(wasm.recording()),
  );
  for (const run of [wasm, restored]) {
    run.session(101, "echo", 100);
    run.session(101, "release", 0);
    run.session(101, "deregister", 0);
  }
  assert.equal(restored.recording(), wasm.recording());
  console.log(
    "Native ↔ WASM command replay verified: custom policy, active session, in-flight packet, loss, continuation, release and deregistration.",
  );
} finally {
  wasm?.free();
  restored?.free();
  await rm(dir, { recursive: true, force: true });
}
