/// <reference lib="webworker" />
import init, { BrowserEngine } from "./wasm/ranterra_engine";
import type { WorkerCommand, WorkerResponse } from "./types";
let engine: BrowserEngine | undefined;
let timer: ReturnType<typeof setInterval> | undefined;
let revision = 0;
const ready = init();
function pause() {
  clearInterval(timer);
  timer = undefined;
}
function emit() {
  if (engine)
    self.postMessage({
      type: "snapshot",
      snapshot: JSON.parse(engine.snapshot()),
      running: !!timer,
      revision,
    } satisfies WorkerResponse);
}
function fail(error: unknown, rev = revision) {
  pause();
  self.postMessage({
    type: "error",
    message: String(error),
    revision: rev,
  } satisfies WorkerResponse);
}
// Serialize commands even while the WASM module is loading.
let chain = Promise.resolve();
self.onmessage = (event: MessageEvent<WorkerCommand>) => {
  const command = event.data;
  chain = chain
    .then(async () => {
      await ready;
      if (command.type === "load") {
        pause();
        const next = new BrowserEngine(JSON.stringify(command.scenario));
        engine?.free();
        engine = next;
        revision = command.revision;
        emit();
        return;
      }
      if (command.revision !== revision || !engine) return;
      if (command.type === "pause") {
        pause();
        emit();
      }
      if (command.type === "step") {
        pause();
        engine.advance(10);
        emit();
      }
      if (command.type === "play" && !timer) {
        timer = setInterval(() => {
          try {
            engine!.advance(10);
            const s = JSON.parse(engine!.snapshot());
            if (s.pending === 0) pause();
            emit();
          } catch (e) {
            fail(e);
          }
        }, 100);
        emit();
      }
    })
    .catch((e) => fail(e, command.revision));
};
