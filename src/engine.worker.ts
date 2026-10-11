/// <reference lib="webworker" />
import init, { BrowserEngine } from "./wasm/ranterra_engine";
import type { WorkerCommand, WorkerResponse } from "./types";
let engine: BrowserEngine | undefined;
let timer: ReturnType<typeof setInterval> | undefined;
let revision = 0;
let speed = 0.1;
const ready = init();
function pause() {
  clearInterval(timer);
  timer = undefined;
}
function emit(includeScenario = false) {
  if (engine)
    self.postMessage({
      type: "snapshot",
      ...(includeScenario
        ? { scenario: JSON.parse(engine.configuration()) }
        : {}),
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
let queued = 0;
self.onmessage = (event: MessageEvent<WorkerCommand>) => {
  const command = event.data;
  if (
    !command ||
    typeof command !== "object" ||
    !Number.isSafeInteger(command.revision)
  )
    return;
  if (command.revision < revision) return;
  if (queued >= 256) {
    fail("Worker command queue limit reached", command.revision);
    return;
  }
  queued++;
  chain = chain
    .then(async () => {
      await ready;
      if (command.type === "load") {
        if (
          !Number.isSafeInteger(command.revision) ||
          command.revision <= revision
        )
          return;
        pause();
        const next = new BrowserEngine(JSON.stringify(command.scenario));
        engine?.free();
        engine = next;
        revision = command.revision;
        emit(true);
        return;
      }
      if (command.revision !== revision || !engine) return;
      if (command.type === "recording") {
        pause();
        self.postMessage({
          type: "recording",
          recording: JSON.parse(engine.recording()),
          purpose: command.purpose,
          revision,
        } satisfies WorkerResponse);
        emit();
        return;
      }
      if (command.type === "compare") {
        const candidate = new BrowserEngine(JSON.stringify(command.recording));
        try {
          self.postMessage({
            type: "comparison",
            snapshot: JSON.parse(candidate.snapshot()),
            name: JSON.parse(candidate.configuration()).name,
            revision,
          } satisfies WorkerResponse);
        } finally {
          candidate.free();
        }
        return;
      }
      if (command.type === "speed") {
        if (![0.1, 1, 5, 10].includes(command.speed))
          throw Error("Unsupported playback speed");
        speed = command.speed;
        return;
      }
      if (command.type === "session") {
        if (
          !Number.isSafeInteger(command.id) ||
          command.id <= 0 ||
          command.id > 0xffffffff ||
          !Number.isInteger(command.loss) ||
          command.loss < 0 ||
          command.loss > 100
        )
          throw Error("Invalid session command");
        engine.session(command.id, command.action, command.loss);
        emit();
        return;
      }
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
            engine!.advance(Math.round(speed * 100));
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
    .catch((e) => fail(e, command.revision))
    .finally(() => {
      queued--;
    });
};
