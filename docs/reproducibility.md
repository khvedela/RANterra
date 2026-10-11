# Deterministic experiments and portable replay

The application now uses a Rust-owned `Experiment` boundary. It journals every successful state-changing session command at the exact engine virtual time: establishment (including a protocol rejection), echo with loss percentage, release and deregistration. Commands sharing a timestamp keep insertion order. Failed local commands are rolled back and excluded. Scenario edits/reset begin a new run; each recording contains that run's complete initial configuration.

## Workflow

1. Import a scenario, configure network session policies and each UE's DNN/S-NSSAI, then run registration. Pause/step/speed affect wall-clock pacing only.
2. Establish sessions and send packets. Echo loss is a deterministic explicit percentage, independent of the RF approximation.
3. In Results, **Export experiment** downloads a portable checkpoint; **Save run** stores one local checkpoint. Both pause playback at the checkpoint boundary. **Restore saved run** reconstructs that checkpoint, including active sessions, counters, timers and in-flight packets. Ordinary Save copy/Export still operate on initial scenario configuration.
4. Use Import for a scenario or an exported experiment. Experiment import replays and checks its digest before replacing the active engine. A malformed, incompatible or altered recording cannot replace it. Resume from the reconstructed state normally.
5. **Compare experiment** independently replays an imported checkpoint and compares virtual time, registered UEs, delivered and dropped echo packets with the current run. It does not replace the current engine.

Try `examples/session-policy-experiment.json`: two UEs request research.example / SST 2 / SD 010203, capacity is one; one rejection/backoff and later successful retry, release, delivered traffic and injected loss are already recorded. The restored second UE has an active session. `cargo run --quiet --locked -p ranterra-engine --example reference_experiment` regenerates the file.

## Format and migration

Scenario schema **2** adds network service policies and per-UE requests. Version-1 input migrates deterministically using explicit defaults: lab / SST 1 / no SD / 128 active sessions; absent RF configuration keeps the legacy model. Both versions are validated; unknown fields/versions fail. Canonical exports use version 2. IndexedDB upgrades from schema 1 to 2 by preserving existing scenario/cache stores and adding the run store.

Experiment format **1**, `format: ranterra-experiment`, carries `engine_profile: ranterra-lab/2`, canonical scenario, ordered timestamped typed commands, checkpoint time, started flag and a SHA-256 state digest. It contains no exported security contexts or keys. Replay derives the same synthetic lab credentials and reconstructs both endpoints, COUNTs, SQNs, allocator and transport queue. This is deterministic reconstruction, not secret-memory serialization.

The digest covers exact protocol/event history, state transitions, cell association, session/counter/failure state, pending work and virtual time. Floating-point radio observations are excluded because final bits may vary by runtime. The digest detects incompatibility/corruption; it is not a signature or authenticity guarantee. The profile must match exactly. New protocol behavior requires a profile revision or an explicit future replay migration, never silent reinterpretation.

Pause/resume/playback speed are not journal entries because they do not alter virtual state. Reset and configuration edits define a new experiment. The checkpoint does not restore UI selection, open panels or real-time pacing. For bounded core operations it restores the supported engine state and permits deterministic continuation, including packets that had not arrived at export time.

Limits: 2 MB scenario JSON, 4 MB recording, 4,096 commands, one hour virtual time; 16 cells/128 UEs and the existing geometry limits apply. The journal refuses new actions when full rather than truncating them. Traces still cap at 16,384 events with an explicit UI warning; replay reconstructs state even when the visible event history is capped. Browser history and local storage are not backups; export checkpoints for portability. Real subscriber credentials are prohibited.

## Native replay and checks

```sh
cargo run --locked -p ranterra-engine --example experiment -- examples/session-policy-experiment.json
# Restore, advance 100 virtual ms, export a new checkpoint:
cargo run --locked -p ranterra-engine --example experiment -- examples/session-policy-experiment.json 100 /tmp/continued-experiment.json
npm test
npm run test:replay
npm run test:protocols
```

Tests cover checkpoints across registration stages, concurrent service selection/backoff, same-time commands, active sessions, in-flight traffic, explicit loss, release/deregistration, malformed ordering/limits/digest, failed-command rollback, mobility/timeout step invariance, and native→WASM→native continuation. Browser tests cover actual worker persistence, import, comparison and corrupted-run isolation.

Build-byte reproducibility remains a separate check:

```sh
npm run build
node scripts/verify-build.mjs
npm run build
node scripts/verify-build.mjs --compare
```
