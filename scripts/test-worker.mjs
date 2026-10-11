// Execute the actual Vite-produced browser worker in a Node worker with a minimal
// Web Worker host. WASM is loaded from disk; ALL external fetches are rejected.
import {
  Worker,
  parentPort,
  workerData,
  isMainThread,
} from "node:worker_threads";
import { readFile, readdir } from "node:fs/promises";
import { pathToFileURL, fileURLToPath } from "node:url";
import { resolve, basename } from "node:path";
import assert from "node:assert/strict";
if (!isMainThread) {
  globalThis.self = globalThis;
  globalThis.postMessage = (data) => parentPort.postMessage(data);
  globalThis.fetch = async (resource) => {
    const url = new URL(resource);
    if (!url.pathname.endsWith(".wasm"))
      throw Error(`External network forbidden in worker test: ${url}`);
    return new Response(
      await readFile(resolve(workerData.assets, basename(url.pathname))),
      { headers: { "Content-Type": "application/wasm" } },
    );
  };
  await import(pathToFileURL(workerData.entry).href);
  parentPort.on("message", (data) => globalThis.onmessage({ data }));
  parentPort.postMessage({ type: "host-ready" });
} else {
  const assets = resolve("dist/assets");
  const entry = (await readdir(assets)).find((n) =>
    /^engine\.worker-.*\.js$/.test(n),
  );
  assert(entry, "Run npm run build:web first");
  const worker = new Worker(fileURLToPath(import.meta.url), {
    workerData: { entry: resolve(assets, entry), assets },
  });
  const messages = [];
  const waiters = [];
  worker.on("message", (m) => {
    messages.push(m);
    for (const w of [...waiters])
      if (w.predicate(m)) {
        waiters.splice(waiters.indexOf(w), 1);
        clearTimeout(w.timeout);
        w.resolve(m);
      }
  });
  const wait = (predicate) =>
    new Promise((resolve, reject) => {
      const prior = messages.find(predicate);
      if (prior) {
        resolve(prior);
        return;
      }
      const item = {
        predicate,
        resolve,
        timeout: setTimeout(
          () => reject(Error("Worker response timed out")),
          10000,
        ),
      };
      waiters.push(item);
    });
  const post = (command) => worker.postMessage(command);
  try {
    await wait((m) => m.type === "host-ready");
    const scenario = JSON.parse(
      await readFile("examples/minimal.json", "utf8"),
    );
    post({ type: "load", scenario, revision: 1 });
    const loaded = await wait((m) => m.type === "snapshot" && m.revision === 1);
    assert.equal(loaded.snapshot.time_ms, 0);
    post({ type: "step", revision: 1 });
    const stepped = await wait(
      (m) => m.type === "snapshot" && m.snapshot.time_ms === 10,
    );
    assert.equal(stepped.running, false);
    assert(stepped.snapshot.events[0].hex.startsWith("7e 00 41"));
    post({ type: "play", revision: 1 });
    await wait((m) => m.running === true);
    post({ type: "pause", revision: 1 });
    await new Promise((r) => setTimeout(r, 180));
    const paused = messages.at(-1);
    assert.equal(paused.running, false);
    const time = paused.snapshot.time_ms;
    await new Promise((r) => setTimeout(r, 180));
    assert.equal(messages.at(-1).snapshot.time_ms, time);
    post({ type: "play", revision: 1 });
    const finished = await wait(
      (m) =>
        m.type === "snapshot" && m.snapshot.started && m.snapshot.pending === 0,
    );
    assert.equal(finished.snapshot.ues[0].state, "Registered");
    assert.equal(finished.running, false);
    const events = finished.snapshot.events;
    const id = scenario.ues[0].id;
    post({ type: "session", id, action: "establish", loss: 0, revision: 1 });
    await wait((m) => m.revision === 1 && m.snapshot?.sessions[0]?.active);
    post({ type: "session", id, action: "echo", loss: 0, revision: 1 });
    post({ type: "play", revision: 1 });
    await wait(
      (m) => m.revision === 1 && m.snapshot?.sessions[0]?.delivered === 1,
    );
    post({ type: "session", id, action: "release", loss: 0, revision: 1 });
    await wait(
      (m) =>
        m.revision === 1 &&
        m.snapshot?.sessions[0] &&
        !m.snapshot.sessions[0].active,
    );

    post({ type: "load", scenario, revision: 2 });
    await wait((m) => m.type === "snapshot" && m.revision === 2);
    post({ type: "play", revision: 1 }); // stale command must do nothing
    await new Promise((r) => setTimeout(r, 140));
    assert.equal(messages.at(-1).snapshot.time_ms, 0);
    post({ type: "play", revision: 2 });
    const replay = await wait(
      (m) =>
        m.revision === 2 && m.snapshot?.started && m.snapshot.pending === 0,
    );
    assert.deepEqual(events, replay.snapshot.events);
    post({ type: "load", scenario: { ...scenario, version: 99 }, revision: 3 });
    const invalid = await wait((m) => m.type === "error" && m.revision === 3);
    assert.match(invalid.message, /version/);
    post({ type: "recording", purpose: "export", revision: 2 });
    const exported = await wait(
      (m) => m.type === "recording" && m.revision === 2,
    );
    post({ type: "load", scenario: exported.recording, revision: 4 });
    const restored = await wait(
      (m) => m.type === "snapshot" && m.revision === 4,
    );
    assert.deepEqual(restored.snapshot, replay.snapshot);
    post({
      type: "load",
      scenario: { ...exported.recording, state_digest: "0".repeat(64) },
      revision: 5,
    });
    await wait((m) => m.type === "error" && m.revision === 5);
    post({ type: "recording", purpose: "save", revision: 4 });
    const unchanged = await wait(
      (m) => m.type === "recording" && m.revision === 4,
    );
    assert.deepEqual(unchanged.recording, exported.recording);
    if (process.argv.includes("--benchmark")) {
      const bounded = JSON.parse(await readFile("examples/paris.json", "utf8"));
      bounded.radio = { model: "UMa" };
      const g = bounded.gnodebs[0],
        u = bounded.ues[0];
      bounded.gnodebs = Array.from({ length: 16 }, (_, i) => ({
        ...g,
        id: i + 1,
        height_m: 25,
        lon: g.lon + (i % 4) * 0.001,
        lat: g.lat + Math.floor(i / 4) * 0.001,
      }));
      bounded.ues = Array.from({ length: 128 }, (_, i) => ({
        ...u,
        id: i + 101,
        lon: u.lon + (i % 16) * 0.0001,
        lat: u.lat + Math.floor(i / 16) * 0.0001,
      }));
      const elapsed = [];
      for (let i = 10; i < 20; i++) {
        const start = performance.now();
        post({ type: "load", scenario: bounded, revision: i });
        const reply = await wait(
          (m) => m.type === "snapshot" && m.revision === i,
        );
        assert.equal(reply.snapshot.ues.length, 128);
        elapsed.push(performance.now() - start);
        messages.length = 0;
      }
      elapsed.sort((a, b) => a - b);
      console.log(
        JSON.stringify({
          worker_load_roundtrip_median_ms: elapsed[5],
          worker_load_roundtrip_max_ms: elapsed[9],
          runs: 10,
          cells: 16,
          ues: 128,
          buildings: bounded.buildings.length,
          node_process_rss_bytes: process.memoryUsage().rss,
        }),
      );
    }
    console.log(
      "Production worker passed: offline WASM loading, step, pause/resume, reset, deterministic replay, stale-command isolation, transactional invalid-load rejection.",
    );
  } finally {
    await worker.terminate();
  }
}
