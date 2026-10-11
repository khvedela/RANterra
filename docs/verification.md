# Verification record — 2026-10-11

Evidence applies to the current **1.0.0-rc.1 research candidate**, not certification. [Release acceptance](V1_RELEASE_REPORT.md) remains blocked on authorized HTTPS hosting verification; the corrected Ant application/workflow passed both remote CI jobs at `5f13d43`. Counts and raw measurements are retained in [reliability-results.json](reliability-results.json) and [benchmark-results.json](benchmark-results.json).

## Ant workspace follow-up

The unchanged production baseline passed 24 browser checks before the visual migration. After the Ant Design 6.6.5 migration and reliability fixes, 30 production checks passed in Chromium/Firefox: the 12 existing workflows plus dialog focus trapping/Escape/return, responsive editing at 390×844 after page scrolling, and axe WCAG A/AA serious/critical scans per browser. This uses the actual WASM worker and static server CSP. `npm test` again passed all 66 Rust tests and actual WASM/worker/native replay; lint, types, input bounds and zero-vulnerability npm audit also pass. A full Rust/WASM production build matches all 13 hashes from the web build; removed font packages account for the old artifact-count difference. The deployment retains `ANT_DESIGN_NOTICES.txt` for 66 introduced runtime package paths. The license inventory now covers 492 packages, with named development-only MPL accessibility tools reviewed in [UI design](ui-design.md). No physical device, WebKit run or external accessibility certification is claimed. A retained remote Firefox trace identified a zoom-induced click landing on Save run while targeting Restore saved run. Scale animation was removed from production dialogs; exact experiment equality and the original timeouts remain intact. Ten repeated local restore/export checks pass (five per browser). Remote follow-up evidence is recorded in the [release report](V1_RELEASE_REPORT.md).

## Prior protocol/reliability follow-up checks

The original MVP baseline passed 14 Rust tests plus actual WASM/worker checks. Before this follow-up, `npm test` passed the existing suite. After the timer/session/reliability changes:

- `cargo test --workspace --locked`: **66 passed**, comprising 35 unit, 10 replay/policy, 2 RF, 4 reliability, 3 cryptographic, 7 signaling and 5 shared-SMF tests. The clean-copy reliability test binary completed in 35.68 seconds; elapsed time varies by build/debug profile and contention.
- `npm run test:protocols`: **13 uncached interactive Go/free5GC peers**, six AUTS negative subcases and **30 byte-exact codec frames** passed. This uses the application's registration and SMF state machines, not an alternate adapter allocator.
- `npm run test:wasm`, `npm run test:worker`, `npm run test:replay`: actual generated WASM, production worker and native↔WASM checkpoint continuation passed for normal and AUTS registration. The profile-2 checked-in recording remains compatible; new profile-3 tests reconstruct delayed operations and registration retry.
- `npm run test:browser`: **24 passed**, 12 each in Chromium/Firefox, against production static assets and real WASM. Includes normal/fault/AUTS registration, timed setup/release loss, failed N2, T3550 unconfirmed outcome, packets, inspector, saved scenarios/checkpoints, imports/comparison, offline computation, worker recovery, quota/denied-storage injection, malicious labels and static headers/routes.
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

Browser quota/denied-access tests inject failures; actual disk exhaustion and all browser storage implementations are not validated. No WebKit, physical mobile device or external penetration test ran. The [internal ASVS Level 1 assessment](asvs-assessment.json) contains 70 applicability/evidence decisions; four TLS/HSTS host requirements remain pending. It is not certification.

## Dependency and CI evidence

- npm audit: **zero known vulnerabilities**.
- Main and isolated-fuzz `cargo audit`: no vulnerability findings; visible unmaintained `paste` warning RUSTSEC-2024-0436 retained.
- Go test-graph audit: only documented isolated-validator GO-2022-1083 exception; additional reachable findings fail.
- License inventory: **426 locked packages**; nine standalone Go dependencies with retained notices. Fuzzer tooling is not shipped; libfuzzer-sys's bundled NCSA notice obligation is documented.
- The previously existing [remote CI run](https://github.com/khvedela/RANterra/actions/runs/38139175447) failed at npm installation because `@luma.gl/webgpu` and `@webgpu/types` were absent from the lockfile. Local legacy-peer configuration had concealed the problem. The lockfile now includes those peer packages and `.npmrc` forces normal peer resolution; clean local install passes.
- Revised GitHub Actions uses pinned action SHAs, Node 24.18.0, Rust 1.97.1, Go 1.27.2, both browser projects, audit/license/reproducibility checks and a separate 60-second sanitizer fuzz job. **Both jobs passed on candidate `57bb6aedaf3f8ea350af143b594db01651bbc4f0`**, [run 38148564181](https://github.com/khvedela/RANterra/actions/runs/38148564181). Candidate commits were pushed with user authorization; no deployment or release publication occurred.

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
# Linux CI uses a virtual display for real WebGL2:
# RANTERRA_HEADED=1 xvfb-run --auto-servernum npm run test:browser
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

## Remote candidate validation follow-up

Candidate `908533308ee46318d28bb448a3056bd17aef68ce` was committed and pushed with explicit user authorization. [Run 38147397454](https://github.com/khvedela/RANterra/actions/runs/38147397454) passed native tests, lint, production build, actual WASM/worker/replay, independent peers and Chromium (11 tests). The separate Linux sanitizer job passed 197,732 inputs in 61 seconds. The validation job failed because all 11 Firefox tests could not find the application controls at startup. Later audit/reproducibility steps were skipped; this run is not a full pass.

The follow-up adds a headless/headed Firefox WebGL2 probe, runs browser CI under Xvfb, retains failure screenshots/traces, and catches map-construction errors so unsupported graphics cannot unmount the entire application. Normal browser tests still require a real map canvas and no map warning; a separate negative test checks graceful behavior when WebGL is deliberately unavailable. Timeouts and protocol assertions were not relaxed. The corrective run now passes. Both headless and headed Firefox obtain WebGL2 under Xvfb (llvmpipe); the full suite is run headed. The initial runner lacked retained page traces, so the precise earlier graphics exception remains an inference, not a recovered diagnostic.

Hosting research also found conflicting global `no-cache` and immutable-asset rules under Cloudflare's comma-merge behavior. Cache rules now avoid that overlap, and the local server combines matching custom headers. Targeted Chromium/Firefox header tests pass. No deployment was performed.

Final corrective candidate **`57bb6aedaf3f8ea350af143b594db01651bbc4f0`**: [both CI jobs passed](https://github.com/khvedela/RANterra/actions/runs/38148564181). All 24 production browser tests passed in 2.5 minutes, along with the full native/protocol/runtime/audit/license/reproducibility/benchmark pipeline. The sanitizer job passed 216,847 inputs in 61 seconds. The remote 128-cycle soak used 2,097,152 bytes maximum linear memory with zero post-warm-up growth, taking 1,849.392581 ms. The [uploaded candidate](https://github.com/khvedela/RANterra/actions/runs/38148564181/artifacts/11706467440) was downloaded and all 38 manifest hashes and WASM magic checked. No external hosting was exercised. This evidence-only documentation update leaves the tested application and workflow unchanged.

## Final Ant candidate remote evidence

Source **`5f13d437f996163ba562b5f2b50685d6c3961d1b`** passed [run 38156308147](https://github.com/khvedela/RANterra/actions/runs/38156308147): 66 Rust tests, 13 independent peers, 30 codec fixtures, real WASM/worker/native replay, 30 production Chromium/Firefox checks in 3.5 minutes, input validation, lint/types, audits/licenses/KDF regeneration, reproducibility and benchmarks. Sanitizer fuzzing completed 195,928 executions in 61 seconds. The 128-cycle/2,560-echo soak took 1,834.420181 ms wall time with 2,097,152 bytes maximum linear memory and zero growth after 32 warm-up cycles; sampled Node RSS was 549,240,832 bytes, not browser peak memory.

The [uploaded artifact](https://github.com/khvedela/RANterra/actions/runs/38156308147/artifacts/11708811163) was downloaded independently: all 13 SHA-256 manifest entries match, WASM magic is correct, and Ant copyright/license notices are retained. GitHub reports archive digest `sha256:f53e1e0c93be91cb29cbfa71b748660afa3e48f1b22fd0b53b8a544aa7fd8319`. Local builds also reproduce their own 13 hashes; no cross-platform byte-equality claim is made.

The intervening [run 38155390325](https://github.com/khvedela/RANterra/actions/runs/38155390325) passed 29 browser tests and failed the Firefox checkpoint-equality assertion. The retained trace identified moving dialog targets, corrected in production UI rather than adding a delay or weakening replay equality. Five repeated restore/export checks per browser passed locally, followed by all 30 local checks in 2.3 minutes and the green remote suite. This evidence-only documentation update leaves the tested application/workflow and deployment output unchanged; CI is skipped only for that documentation commit. Hosted HTTPS/provider verification remains pending and no deployment/publication is performed.

## Cloudflare hosting preparation

The actual Wrangler 4.149.0 Pages dev runtime rejected the old broad rewrite and returned HTML/200 for nonexistent assets. This behavior was not reproduced by the older bespoke local server; its earlier green header checks did not prove the host gate. The candidate now includes a 404 page and supported explicit rewrites; the local server reads the actual rewrite file.

All 30 production browser tests passed against Wrangler Pages dev in 2.4 minutes, followed by both strengthened static-host tests (real worker JavaScript/WASM MIME and WASM magic, missing assets, unknown paths and deep refresh) in 9.6 seconds. The bespoke local server passed its two updated routing checks. Lint, TypeScript and a full Rust/WASM production build pass; all 14 artifact hashes reproduce between web/full builds. The engine and interface are unchanged. CI revalidation, account authentication and actual HTTPS/custom-domain verification remain pending. No hosted deployment or release publication has yet occurred.
