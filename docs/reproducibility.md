# Reproducing experiments

Use a pinned source revision, both lockfiles, the documented toolchain, the same version-1 scenario JSON, seed, and the same commands at the same virtual times. The scheduler orders events by delivery time then insertion order. Mobility ticks are independent of caller step size. Native, actual WASM and production-worker tests exercise deterministic replay.

1. Import `examples/minimal.json` for static registration, `examples/research-mobility.json` for UMa movement, or `examples/authentication-failures.json` for isolated negative cases. The bundled Paris scenario includes OSM geometry and provenance; it avoids a live geography fetch changing the experiment.
2. Choose the radio model explicitly. Start or step the simulation. Speed controls change wall-clock pacing only; displayed protocol timestamps and echo latency are virtual milliseconds.
3. After registration, establish a session from the UE inspector and send an echo. Continue execution to deliver queued packets, then release the session or deregister. Record command timing if repeating this interactive experiment.
4. Save a named local copy or export scenario JSON. Export the bounded event log and JSON/CSV results separately. Results include units and model/scenario metadata. Compare exports externally; the UI provides a current-run dashboard, not a complete comparative-run database.

Scenario imports are validated in Rust before replacing the active engine. Unknown fields, including credentials, unsupported versions and excessive geometry are rejected. Optional new version-1 fields have explicit defaults, so old version-1 files still load. A future version-2 migration has not been implemented; future versions currently fail with a descriptive error. Named copies are bounded to 50; geometry cache entries to 100.

Scenario files store configuration, not secret contexts, active IP sessions or an executable journal of manual commands. Loading or worker recovery restarts the experiment at time zero. Event/results exports cannot currently restore an interactive session or replay all manual actions automatically. This is a remaining v1 release gate. Seeded automatic registration and movement are reproducible without those manual actions.

Logs are capped at 16,384 events and the UI shows the latest 500. The UI reports trace truncation. Simulation continues after the cap; export does not invent missing events. Split large experiments or reset before recording another run. No real subscriber keys belong in scenarios, logs or browser storage.

Build reproducibility check (same machine/toolchain):

```sh
npm run build
node scripts/verify-build.mjs
npm run build
node scripts/verify-build.mjs --compare
python3 scripts/generate-kdf-vectors.py > /tmp/ranterra-kdf.json
diff engine/tests/kdf-vectors.json /tmp/ranterra-kdf.json
```

This checks asset bytes and independently derived cryptographic fixtures. It is not a claim of cross-platform bit-identical native binaries or deterministic transcendental arithmetic on every CPU/runtime.
