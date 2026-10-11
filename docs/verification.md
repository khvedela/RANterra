# Verification record — 2026-10-11

Evidence applies to the current **1.0.0-rc.1 research candidate**, not certification. [Release acceptance](V1_RELEASE_REPORT.md) remains blocked on remote CI for this tree and authorized HTTPS hosting verification. Counts and raw measurements are retained in [reliability-results.json](reliability-results.json) and [benchmark-results.json](benchmark-results.json).

## Baseline and final local checks

The original MVP baseline passed 14 Rust tests plus actual WASM/worker checks. Before this follow-up, `npm test` passed the existing suite. After the timer/session/reliability changes:

- `cargo test --workspace --locked`: **66 passed**, comprising 35 unit, 10 replay/policy, 2 RF, 4 reliability, 3 cryptographic, 7 signaling and 5 shared-SMF tests. The clean-copy reliability test binary completed in 35.68 seconds; elapsed time varies by build/debug profile and contention.
- `npm run test:protocols`: **13 uncached interactive Go/free5GC peers**, six AUTS negative subcases and **30 byte-exact codec frames** passed. This uses the application's registration and SMF state machines, not an alternate adapter allocator.
- `npm run test:wasm`, `npm run test:worker`, `npm run test:replay`: actual generated WASM, production worker and native↔WASM checkpoint continuation passed for normal and AUTS registration. The profile-2 checked-in recording remains compatible; new profile-3 tests reconstruct delayed operations and registration retry.
- `npm run test:browser`: **22 passed**, 11 each in Chromium/Firefox, against production static assets and real WASM. Includes normal/fault/AUTS registration, timed setup/release loss, failed N2, T3550 unconfirmed outcome, packets, inspector, saved scenarios/checkpoints, imports/comparison, offline computation, worker recovery, quota/denied-storage injection, malicious labels and static headers/routes.
- `npm run test:inputs`: provider configuration is validated before application; unsafe URL forms/unknown fields and byte-limit overflow fail; oversized response readers are cancelled.
- `npm run build`, TypeScript, ESLint, rustfmt and Clippy with denied warnings pass. Independent Python KDF expectations regenerate byte-identically.
- `npm ci && npm run build && npm test && npm run test:inputs` passed in a copied source tree with no node_modules/dist/target/generated WASM. Node 24.18.0, Rust 1.97.1. Final example/help-text corrections were checked separately in the main tree; final production browser/reproducibility tests use those corrections.
- Two consecutive production builds match **38 artifact hashes**. Artifact verification checks WASM magic and rejects accidental source/VCS/dependency-tree files.
- The current-profile `reference_experiment` generator produces a recording that the native `experiment` example successfully replays; the new signaling example parses/runs natively.

Some earlier runs failed: import/control-state races were fixed, stale tests were updated for timed delivery, and three Chromium boot assertions timed out during simultaneous heavy compilation. A subsequent full browser run passed without relaxing the timeout. A local disk-capacity incident interrupted two checks; generated Rust artifacts were cleaned and affected checks reran successfully. Failures are not counted as passing evidence.

## Reliability evidence

`engine/tests/reliability.rs` exercises 12,000 deterministic inputs / 144,000 parser/authentication calls, 254 allocation/release cycles through pool exhaustion, queue overflow rollback and malformed transport isolation.

The isolated `validation/fuzz` harness, cargo-fuzz 0.13.2 and nightly-2026-07-20, ran AddressSanitizer/libFuzzer for **301 seconds / 837,065 executions**, seed 501. No crash or sanitizer finding. Final counters: **38,113 coverage edges, 67,726 features, 7,878 corpus entries**, observed RSS 573 MiB. These are instrumentation counters, not a percentage of source covered. Maximum input 65,536 bytes, per-input timeout 5 seconds, RSS limit 2,048 MiB. Fuzzer counts depend on platform/time despite seeded generation.

`RANTERRA_SOAK_CYCLES=16384 npm run test:soak` completed **16,384** create/register/session/20-echo/release/deregister/replay/free cycles with **327,680** original echo requests in **103,438.373709 ms wall time**. Linear memory was **2,097,152 bytes** at warm-up and maximum, with **zero growth after 32 cycles**. Sampled Node RSS was 49,872,896 bytes. This extended workload is still approximately 103 seconds, not an hours-long soak or browser-memory proof. The CI default is 128 cycles.

Browser quota/denied-access tests inject failures; actual disk exhaustion and all browser storage implementations are not validated. No WebKit/mobile or external penetration test ran. The [internal ASVS Level 1 assessment](asvs-assessment.json) contains 70 applicability/evidence decisions; four TLS/HSTS host requirements remain pending. It is not certification.

## Dependency and CI evidence

- npm audit: **zero known vulnerabilities**.
- Main and isolated-fuzz `cargo audit`: no vulnerability findings; visible unmaintained `paste` warning RUSTSEC-2024-0436 retained.
- Go test-graph audit: only documented isolated-validator GO-2022-1083 exception; additional reachable findings fail.
- License inventory: **426 locked packages**; nine standalone Go dependencies with retained notices. Fuzzer tooling is not shipped; libfuzzer-sys's bundled NCSA notice obligation is documented.
- The previously existing [remote CI run](https://github.com/khvedela/RANterra/actions/runs/38139175447) failed at npm installation because `@luma.gl/webgpu` and `@webgpu/types` were absent from the lockfile. Local legacy-peer configuration had concealed the problem. The lockfile now includes those peer packages and `.npmrc` forces normal peer resolution; clean local install passes.
- Revised GitHub Actions uses pinned action SHAs, Node 24.18.0, Rust 1.97.1, Go 1.27.2, both browser projects, audit/license/reproducibility checks and a separate 60-second sanitizer fuzz job. **It has not run remotely for this uncommitted tree.** No push, deployment or publication occurred.

## Reproduce

```sh
export PATH="$HOME/.cargo/bin:$PATH"
npm ci
npm run build
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
npm run lint
npm run typecheck
npm test
npm run test:inputs
npm run test:protocols
npx playwright install chromium firefox
npm run test:browser
RANTERRA_SOAK_CYCLES=16384 npm run test:soak
# Install the isolated toolchain per validation/fuzz/README.md first:
sh scripts/fuzz-protocols.sh 300
npm audit --audit-level=moderate
cargo audit
cargo audit --file validation/fuzz/Cargo.lock
node scripts/check-licenses.mjs
python3 scripts/check-validation-licenses.py
python3 scripts/audit-validation.py
python3 scripts/generate-kdf-vectors.py > /tmp/kdf.json
diff engine/tests/kdf-vectors.json /tmp/kdf.json
node scripts/verify-build.mjs
npm run build
node scripts/verify-build.mjs --compare
npm run benchmark
npm run benchmark:wasm
node scripts/test-worker.mjs --benchmark
```

Install cargo-audit 0.22.2 as described in CI. Go's validation toolchain is pinned independently. Run the soak after browser tests, since Playwright clears `test-results`. The standalone test peer is not an unmodified external mobile core; see [exact boundary](interop-validation.md).

Benchmarks cover the existing 16-cell/128-UE/467-building/1,225-grid-sample UMa scenario. Native initialization median/max **22.45/25.14 ms**; WASM **46.03/77.29 ms**; worker reload **49.07/53.90 ms**. Native peak RSS **10,174,464 bytes** excludes compilation; WASM linear memory **6,488,064 bytes**. Full data and memory definitions are in the release report. No RF fidelity was reduced. Geography chunk/loaders.gl/wasm-bindgen fallback build warnings remain visible; successful builds are not described as warning-free.
