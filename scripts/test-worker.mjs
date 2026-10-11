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
    assert.equal(finished.snapshot.ues[0].state, "RegistrationRejected");
    assert.equal(finished.running, false);
    const events = finished.snapshot.events;
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
    console.log(
      "Production worker passed: offline WASM loading, step, pause/resume, reset, deterministic replay, stale-command isolation, transactional invalid-load rejection.",
    );
  } finally {
    await worker.terminate();
  }
}
