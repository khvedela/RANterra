# RANterra v1 release report

Date: 2026-10-11. Candidate: **1.0.0-rc.1**, intended stable version **1.0.0**. **Release acceptance: FAIL / BLOCKED. Do not publish this candidate as production-ready v1.0.** No release was published and no external infrastructure was deployed.

The existing prototype has been extended into a functioning browser-only authenticated registration and virtual data-plane application. This is a substantial implementation, with genuine encoded protocols and cryptography, but several requested release gates remain incomplete. The status below distinguishes implemented narrow support from missing requirements.


## Follow-up implementation: policies, replay and independent validation

The configurable-policy and portable-replay blockers are now implemented for the supported single-session IPv4 model:

- Up to eight validated DNN/S-NSSAI pairs (SST and optional SD), per-UE requested service, per-policy active-session capacity and browser forms. NAS Registration Accept, N1 selection, SM accept and N2 setup carry the configured slice/service. AMF rejection uses protected unforwarded N1 / 5GMM cause 91; resource rejection uses genuine 5GSM cause 26 with a 10-second virtual T3396 backoff. Rejects allocate no IP/TEID; backoff and successful retry are tested.
- Rust-owned ordered command journal, format/profile validation, SHA-256 exact-state compatibility check, candidate replay before commit, and failed-command rollback. Export/import and explicit IndexedDB run checkpoints restore registration/security progression, active sessions, counters, allocator state, pending packets, timers and deterministic continuation without serialized keys. Scenario v1→v2 and IndexedDB v1→v2 migrations preserve existing data. Portable comparison replays another run without replacing the active run.
- Native↔WASM replay cross-check passes with custom policy, in-flight packet, loss, continued traffic, release and deregistration. The timeout event is now scheduled at 6,000 ms even when mobility crosses it in a large step; unchanged velocity input no longer resets execution and playback speed survives scenario replacement.
- Independent free5GC codecs decode/re-encode 30 NAS/NGAP frames exactly and verify the inner N2 setup transfer. This caught OxiRush 0.4.2's erroneous RRC establishment-cause IE ID 237; an explicit adapter now emits standard ID 90. Non-GBR N2 setup now includes session-AMBR matching the NAS profile.
- Sixteen published ns-3.47 numerical path-loss reference points match within 0.0001 dB, plus breakpoint/frequency-edge checks. These are external numerical reference checks, not a drive-test calibration or live ns-3 run.

See [replay workflow](reproducibility.md), [independent validator](../validation/free5gc/README.md) and [RF references](radio-validation.md). New standalone validator dependencies are isolated from browser assets and have retained Apache-2.0/MIT/BSD-3-Clause notices. No new runtime dependencies or external deployment were introduced.

## Implemented features

- Synthetic subscriber provisioning, null-scheme SUCI identity validation, MILENAGE AUTN/SQN verification, RES*/XRES* and HRES*/HXRES* checks, 5G key derivation, independent UE/AMF NIA2/NEA2 security contexts and UL/DL counters.
- Encoded registration request, authentication, Security Mode, Registration Accept/Complete and protected deregistration. Registration is not displayed as successful until the AMF verifies the final protected message. Negative credentials/AUTN/response/MAC/count/unknown-subscriber/timeout/coverage cases cannot register and are isolated between UEs.
- Protected 5GSM N1 session establishment/release, NGAP N2 setup/release resources, local SMF/UPF state, virtual IPv4 allocation, directional TEIDs, checked GTP-U and IPv4/ICMP echo packets, queued delivery, virtual RTT, loss and counters. No frontend protocol mocks.
- Selectable bounded UMi/UMa path loss, LOS/NLOS footprint obstruction, seeded optional shadowing, sector orientation, configurable resource allocation/numerology, thermal noise and co-channel SINR. Waypoint motion updates measurements, coverage and A3-style reassociation.
- Existing geographic workspace preserved, with registered/failed/disconnected states, session and traffic inspector actions, trajectory drawing, fault selection, protocol/UE filtering, binary inspection, duration/speed/seed controls, results dashboard, CSV/JSON export and named local copies.
- Bounded/validated inputs and geometry, transactional engine replacement, stale-command rejection, worker recovery, bounded traces/queues/cache, configurable external providers, CSP/static hosting rules, Chromium/Firefox tests, CI, audits and license inventory.

## Architectural changes

The React/MapLibre/deck.gl → Worker → WASM boundary remains intact. The shared native Rust engine now separates `auth`, `registration`, `session`, `radio`, `transport`, and deterministic scheduling. USIM/network authentication and separate NAS contexts replace the rejection-only happy path. SMF/UPF control is collocated and explicitly does not claim PFCP. Radio calculations remain in the worker and use an indexed footprint subset. Research traits expose radio, mobility, transport, controller, metrics and executor boundaries; the included controller baseline is a documented no-op, not AI.

Registration and echo traverse the scheduler. Session control and deregistration currently execute atomically at the current virtual time; their protocol frames are real but their control-plane latency is not a modeled result. Reassociation invalidates registration/session state and is not labeled an NGAP handover. See [architecture](architecture.md) and [protocol matrix](protocol-support.md).

## Running and building

Prerequisites: Node.js 24, npm, rustup/Rust 1.97.1, wasm32-unknown-unknown, wasm-pack 0.15.0. Preserve both lockfiles.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
rustup toolchain install 1.97.1 --component rustfmt --component clippy --target wasm32-unknown-unknown
cargo install wasm-pack --version 0.15.0 --locked
npm ci
npm run build:wasm
npm run dev
```

Development: http://127.0.0.1:5173. Production static build:

```sh
npm run build
node scripts/verify-build.mjs
node scripts/serve-dist.mjs
```

Production verification: http://127.0.0.1:4173. Deploy only the `dist/` artifact after authorization, applying `_headers`/`_redirects`, correct WASM/worker MIME types and provider policy/attribution configuration. See [deployment](deployment.md). No backend is needed for emulation.

## Verification evidence

Before edits, the existing 14 Rust tests, actual WASM runtime and production-worker suite passed. After implementation:

| Check actually run locally | Result |
|---|---|
| Native Rust unit tests | 32 passed |
| Cryptographic integration tests | 3 passed: MILENAGE also covered in unit suite; NEA2, NIA2, independent KDF fixtures here |
| Replay/policy integration tests | 9 passed, including checked-in portable run |
| RF reference integration tests | 2 passed, covering 16 external points and boundaries |
| Independent codec check | 30 NAS/NGAP frames byte-exact through free5GC |
| Native↔WASM replay | Custom policy and stateful continuation verified |
| WASM release compilation and actual runtime | Passed; registration/session/echo/release/deregistration and deterministic replay |
| Production-worker execution | Passed; offline WASM, ordering, pause/resume/reset, stale revisions, invalid-load rollback, session/traffic |
| Chromium + Firefox production-browser suite | 14 passed (7 per browser), including run persistence/migration/quota recovery |
| TypeScript / Vite production build | Passed |
| ESLint / rustfmt / Clippy with warnings denied | Passed |
| Python independent KDF fixture regeneration | Passed byte comparison |
| Standalone Go validator audit | One documented isolated-tool exception: GO-2022-1083; any new reachable finding fails the added audit |
| npm vulnerability audit | 0 known vulnerabilities |
| cargo vulnerability audit | No known vulnerabilities; unmaintained `paste` warning RUSTSEC-2024-0436 remains |
| Locked dependency/license inventory | 417 packages reviewed by allowlisted expressions; no AGPL integrated |
| GitHub Actions pipeline | Added, not executed remotely in this work |

Browser tests cover boot and WebGL map canvas, entity editing, successful/failed registration, session/echo/release, decoded protocol inspection, persistence, scenario export and invalid import, offline already-loaded computation, worker crash/recovery, static cache/security headers and route refresh. External geographic requests are disabled in automation to avoid burdening public services; live provider availability and every hosting platform are not certified. No WebKit/mobile-browser suite, prolonged soak/fuzz campaign or exhausted-quota test has run.

A clean copied source tree excluding node_modules/dist/target/generated WASM successfully installed dependencies, built and passed native/WASM/worker tests. The initial run also happened to pass under Node 22.11 despite engine warnings; Node 24 remains the supported documented/CI version. The supported Node 24.18.0 / rustup 1.97.1 rerun also passed. Two consecutive final production builds matched all 38 artifact hashes. Details are recorded in [verification](verification.md).

## Protocol references and fidelity

The supported subset targets Release 18: TS 24.501 **18.7.0**, TS 33.501 **18.5.0**, TS 23.502 **18.5.0**, TS 38.413 **18.4.0**. Published MILENAGE values use TS 35.208 **18.0.0**; the NEA2 fixture uses TS 33.401 **18.0.0**. NIA2 has an independent free5GC vector; 5G KDF expectations were computed independently using Python HMAC. See [exact primary references and omissions](protocol-support.md).

No claim of complete 3GPP compliance, live UERANSIM/Open5GS interoperability, carrier-grade security or standardized 6G is made. Supported registration/session frames are encoded and decoded; omitted RRC/AS/NG setup/SBI/PFCP procedures are explicitly outside the current virtual profile.

## RF validation and benchmarks

TR 38.901 **18.0.0**, Table 7.4.1-1 subset. At 100 m horizontal separation and 3.5 GHz, LOS reference fixtures are UMi **85.314188 dB** and UMa **83.1381567 dB**. Tests check domain rejection, NLOS ordering, antenna direction, deterministic shadowing, interference, motion and reassociation. Missing footprint data assumes LOS; no terrain/material/height model or correlated fading is claimed. See [RF assumptions](radio-validation.md). Independent numerical comparison with published ns-3.47 reference points now passes; measured calibration/live RF-tool execution is still absent.

Earlier candidate benchmark, retained for comparison. Measured locally on Apple M3 Pro, arm64 macOS (native benchmark: installed Rust 1.96.1; WASM: pinned Rust 1.97.1 / Node 24.18.0), 10 runs, **16 gNodeBs / 128 UEs / 467 buildings / 1,225 grid samples**, UMa:

| Metric | Native release | Actual WASM in Node 24 |
|---|---:|---:|
| RF/engine initialization median | 23.23 ms | 47.48 ms |
| RF/engine initialization maximum | 34.34 ms | 199.00 ms |
| Registration advance median (1,000 virtual ms) | 12.51 ms | 18.02 ms |
| Peak native RSS / sampled Node process RSS | 9.88 MB | 461.57 MB |
| WASM linear memory | n/a | 6.49 MB |
| Serialized snapshot | 1.04 MB | 1.04 MB |

Native RSS came from `/usr/bin/time -l`; Node RSS includes the host, JIT, modules and runtime and is not WASM linear memory or peak browser-tab memory. RF numbers measure initialization including scenario validation and grid computation. They are wall-clock timings, not virtual network latency. Concurrent system activity affected the WASM maximum. Browser rendering latency and full browser-tab peak memory are not established by this benchmark. Worker reload round trips (validation + RF + serialization + transfer, ten runs) measured **48.32 ms median / 57.34 ms maximum** in the Node worker host; sampled whole-process RSS was 492.80 MB. This does not measure browser painting.

Follow-up measured results on the same bounded workload: WASM RF median **54.20 ms**, max **90.94 ms**, registration median **20.25 ms**, linear memory **6.36 MB** (sampled Node RSS 519.13 MB). Worker reload median **58.97 ms**, max **70.68 ms** (sampled process RSS 289.14 MB). No silent fidelity changes; the bounded regression budgets pass. Different whole-process RSS samples are not a reliable memory-leak trend.

Commands: `npm run benchmark`, `npm run benchmark:wasm`, `node scripts/test-worker.mjs --benchmark`. The WASM regression guard checks <64 MiB linear memory and <5 s initialization for this bounded workload; this is a safety budget, not a real-time SLA. Editing rebuilds the grid; motion updates only UE links. No fidelity is silently reduced under load.

## Acceptance gates and remaining work

| Required gate | Status / evidence or blocker |
|---|---|
| Clean production static boot and browser WASM | PASS locally, production Chromium/Firefox |
| Multiple independent authenticated registrations | PASS for documented laboratory subset |
| Genuine NAS integrity/ciphering | PASS for NIA2/NEA2; known vectors, replay/tamper/count checks |
| Establish/use/release a PDU session | PASS for configurable DNN/slice/capacity with one IPv4 session per UE |
| Inspect actual protocol bytes | PASS within trace cap; no synthetic success substitution |
| Movement affects RF and network state | PASS for waypoints/coverage loss/reassociation; protocol handover unsupported |
| Scientific model validation/documentation | PASS for the bounded equations with independent published ns-3 numerical references; field calibration remains outside validated scope |
| Save/restore/export/import/replay experiments | PASS for supported bounded engine actions: portable journal replay, checkpoints, migrations and active-state continuation |
| Invalid inputs cannot corrupt active state | Covered by bounded schema, packet and transactional load tests; extensive fuzz/storage-quota proof absent |
| Required automated coverage | PARTIAL: local suites pass; independent codec validation added; full external state-machine interoperability and extended reliability cases remain |
| Successful production deployment | NOT VERIFIED: local static deployment passes; no external hosting authorized/executed |
| Visible limitations and honest scope | PASS: README, UI labels/help, protocol/RF/security documents |

Remaining engineering priorities before claiming the requested v1.0:

1. Independent full registration/session state-machine interoperability and broader failure/retransmission/AUTS coverage remain. Configurable DNN/S-NSSAI and capacity reject/backoff are now implemented; the registration timeout is still a laboratory deadline.
2. Portable command replay, state reconstruction, comparison and version-1 migration now pass for supported actions. Remaining extensions include future engine-profile migrations, longer/cancelable replay jobs, multi-session/QoS expansion and richer comparative statistics. Current recording limits are explicit (4 MB, 4,096 commands, one hour).
3. Broader RF reference checks/calibration, geometry-height support where data exists, measured browser rendering/memory budgets and improved incremental grid recomputation. Do not call current reassociation protocol handover or promise automatic reconnection.
4. Complete applicable ASVS verification, extended malformed-packet fuzzing/soak/storage-failure coverage, track the unmaintained dependency, observe remote CI and verify a configured hosted deployment only after authorization.
5. Richer measured controller objectives/observations and useful baseline policy integration remain future research work. Current traits/no-op baseline are foundations, not a trained controller or completed 6G platform.

Other deliberate limits: no real subscriber secrets, one session per UE, allocation pool stops after 254 sessions until reset, no internet sockets, no PFCP claim, no RSRQ, no SSB/PHY scheduler, no multipolygon holes/terrain, bounded history, public geography requires provider configuration for scale. Large geography bundle warnings remain; the UI architecture was preserved rather than rewritten.
