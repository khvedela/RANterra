# Verification record — 2026-10-11

This supersedes the MVP rejection-only verification record. It is evidence for a candidate, not standards certification. Stable v1.0 remains blocked; see [release report](V1_RELEASE_REPORT.md).

Baseline before edits: 14 native Rust tests plus actual WASM and production-worker tests passed.

Final local implementation checks:

- `cargo test --workspace`: 31 unit + 3 cryptographic integration tests passed. Coverage includes real registration, independent contexts, all fault profiles, NAS replay/count rollover, PDU lifecycle and late traffic after teardown, APER/malformed parsing, radio/geometry, deterministic scheduler/mobility and unknown-input rejection.
- `npm run build`: Rust WASM release, TypeScript and Vite production output succeeded. Separate emulation and MapLibre workers and actual .wasm asset emitted.
- `npm run test:wasm`: actual compiled WASM registration, session/echo/release/deregistration and replay checks passed.
- `npm run test:worker`: production Vite worker in a Node worker host with external fetch forbidden passed session traffic, pause/resume, revision ordering, replay and failed-load isolation.
- `npm run test:browser`: 8 production browser tests passed, 4 Chromium and 4 Firefox. Real WASM and production worker, no frontend protocol stubs. External geographic assets disabled. Map canvas, state changes, inspector, data-plane actions, editing, persistence, export/import rejection, offline computation, worker recovery, CSP/cache/MIME and route refresh exercised.
- ESLint, Rust formatting and Clippy (`--all-targets --locked -- -D warnings`) passed. Independent Python KDF fixture regeneration matched checked-in bytes.
- `node scripts/verify-build.mjs` followed by a second full build and `--compare`: all 38 production artifacts matched byte-for-byte on this machine/toolchain; WASM magic and worker presence checked. This does not claim identical output across different checkout paths/operating systems.
- Clean source-copy `npm ci`, build and native/WASM/worker checks passed without copied node_modules, generated WASM, dist or target. Repeated with Node 24.18.0 and rustup 1.97.1. The earlier initial copy also passed with Node 22.11 (engine warnings); use Node 24 as documented. Minor final storage/list and documentation edits were verified in the main checkout.
- Native replay example successfully loads both new research/mobility and authentication-fault scenarios. At 1,000 virtual ms the fault scenario isolates one registered UE and six rejected UEs; the timeout UE correctly remains pending until its 6,000 ms deadline (deadline covered in unit tests).
- Dependency checks: npm zero known vulnerabilities; cargo no known vulnerabilities with one unmaintained-package warning (`paste`); 417 locked package license expressions reviewed. No AGPL source introduced.

Production build warnings remain for the large geography chunk (~1.93 MB uncompressed), an unused loaders.gl Node child-process export, and wasm-pack's platform binary fallback (successfully builds the required binding tool). These did not prevent the real browser tests from passing. They are not silently represented as warning-free builds.

Browser screenshots are generated under ignored `test-results/production-workspace.png`. No fresh hosted deployment, remote GitHub Actions run, mobile/WebKit coverage, external-core interoperability session, independent RF-tool/drive-test comparison, sustained fuzzing or full ASVS assessment was performed.

## Performance evidence

See the release report for native/WASM timings, memory definitions and workload. Worker benchmark uses `node scripts/test-worker.mjs --benchmark`; it reloads the same 16-cell/128-UE/467-building scenario ten times, timing message submission through receipt of the serialized snapshot. It includes scenario validation, RF grid construction, serialization and structured-clone transfer, but not browser painting. Results from the final run are appended below.

Final worker-host result: **48.316 ms median**, **57.344 ms maximum**, ten runs. Sampled whole Node-process RSS **492,797,952 bytes** (includes worker host/JIT, not peak browser memory). No main-thread browser paint time is included.
