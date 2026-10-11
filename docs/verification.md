# Verification record — 2026-10-11

This supersedes the MVP rejection-only verification record. It is evidence for a candidate, not standards certification. Stable v1.0 remains blocked; see [release report](V1_RELEASE_REPORT.md).

## Follow-up validation

The policy/replay follow-up suite comprises 32 unit tests + 9 replay/policy integration tests + 2 RF reference tests + 3 crypto integration tests (**46 Rust tests**). Actual WASM/production worker and native↔WASM portable replay pass. The external free5GC check decodes/re-encodes **30 NAS/NGAP frames** exactly, including the N2 request transfer and required AMBR. Sixteen external ns-3.47 numerical RF points pass at tolerance 0.0001 dB; ns-3 itself was not executed.

Browser automation now includes 14 tests (7 per Chromium/Firefox): the earlier 8 plus custom DNN/SST/SD, explicit run persistence across page reload, exported replay/comparison, corrupted-recording isolation, policy resource rejection, IndexedDB-v1 migration and injected quota failure with usable export. All 14 passed in the final full browser run (52.9 seconds).

The replay suite covers registration-stage checkpoints and deterministic continuation with active sessions and in-flight packets, same-time commands, rejected policy/backoff/retry, explicit loss, teardown, bounds, malformed ordering/digest/profile, command rollback and timeout/mobility step invariance. `examples/session-policy-experiment.json` is checked as an executable fixture.

New checks: `npm run test:replay`, `npm run test:protocols`, `python3 scripts/check-validation-licenses.py`. Go 1.27.2 is pinned for the standalone independent validator only. Rust/npm runtime dependencies are unchanged. The six linked validator dependencies have explicit reviewed notices; the APER advisory is tracked in the validator documentation rather than concealed.

Latest follow-up bounded workload (Apple M3 Pro, Node 24, 16 cells/128 UEs/467 buildings/1,225 samples, 10 runs): WASM RF initialization median **54.199 ms**, maximum **90.941 ms**, registration advance median **20.245 ms**, linear memory **6,356,992 bytes**, sampled Node RSS **519,127,040 bytes**. Worker reload round-trip median **58.970 ms**, max **70.683 ms**; sampled whole-process RSS **289,144,832 bytes**. These are wall-clock worker-host results, not virtual latency or browser peak memory. Regression budgets passed.

Follow-up build checks: full Rust/WASM/TypeScript/Vite build, lint, rustfmt and Clippy with warnings denied passed; two complete production builds matched all 38 artifact hashes. The final policy-dialog presentation adjustment was checked with two byte-identical web builds and the relevant Chromium/Firefox workflow rerun. npm audit found zero known vulnerabilities; cargo audit retains the unmaintained paste warning. The pinned Go 1.27.2 / govulncheck 1.9.0 audit has only the explicit isolated APER advisory exception; the exception guard passed and retains raw JSON under test-results. No hosted deployment or publication was attempted.

## Earlier candidate evidence (retained historical baseline)

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
