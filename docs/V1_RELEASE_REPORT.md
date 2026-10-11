# RANterra v1 release report

**2026-10-11 — candidate 1.0.0-rc.1, target 1.0.0. Release acceptance: BLOCKED.** The four latest engineering work areas are implemented and locally validated for the supported research profile. A green remote CI run for this candidate and an authorized HTTPS deployment verification are still required. The user has now authorized committing and pushing this candidate to obtain remote CI evidence. CI is pending; deployment and publication remain explicitly unauthorized.

This is a functioning browser-only research application with actual cryptography, encoded protocols and a virtual data plane. It is not a complete carrier core, an interoperability certification or standardized 6G implementation. Local validation is not substituted for hosting evidence.

## Completed in this follow-up

1. **Registration retransmission and recovery.** Normal WB-N1 initial registration implements T3560/T3550 at 6 seconds with four retries and fifth-expiry outcomes. Security Mode and Registration Accept retries use fresh protected NAS COUNTs; an outstanding repeated authentication challenge reuses verified RES* without resetting SQN. T3510 (15 seconds), T3511 (10 seconds) and default T3502 (720 seconds after the fifth failed attempt) drive deterministic retry while preserving USIM freshness. AMF T3550 exhaustion enters registered as prescribed, but the application shows `RegistrationUnconfirmed` and blocks sessions because Registration Complete was not verified. No false REGISTERED success is shown.
2. **Failed N2 resource handling.** A real encoded NGAP failed-session list/unsuccessful transfer with radio-network cause 22 (`radio_resources_not_available`) causes protected network-initiated session release with PTI 0. Failed setup never activates forwarding. Reservations, stale responses and release cleanup are tested against an independent peer. Other N2 failure causes are not claimed.
3. **Timed browser session signaling.** Profile-3 session commands enqueue actual protected N1 and N2 messages with configurable 1–1000 ms virtual delivery stages. Bounded message loss, N2 rejection, T3592 retries and missing-acknowledgement cleanup are exposed in the UE inspector. Pending operations disable invalid actions; explicit release must complete before deregistration. Registration retains seeded 10–30 ms per-hop transport. Session delay is a laboratory model, not physical radio latency.
4. **Reliability and security validation.** An AddressSanitizer/libFuzzer campaign completed 837,065 inputs; an extended real-WASM soak completed 16,384 lifecycle/replay/free cycles and 327,680 original echo requests. Browser tests cover storage denial/quota failure and malicious text labels. The internal ASVS 5.0.0 Level 1 assessment covers all 70 requirements, with four host requirements explicitly pending. Provider configuration is transactional and rejects unsafe URL forms; cache keys include provider identity; IndexedDB capacity updates use one transaction. CI actions are pinned and parser fuzzing is included.

The new replay profile hashes the scenario and ordered command journal as well as exact non-RF state, detecting edits that affect future queued work. Checkpoints reconstruct timers, pending session messages, counters and deterministic continuation without serializing keys. Existing profile-2 recordings retain their original atomic session driver and digest; new signaling fields cannot be silently applied to them.

Validation found and fixed import/worker acknowledgement races, a controlled signaling-checkbox race, a sample generator that assumed instant session setup, and an incomplete npm peer-dependency lockfile. The previous remote CI run failed at `npm ci`; the repaired lockfile adds only its missing WebGPU peer packages, and repository `.npmrc` prevents local legacy-peer settings from masking the defect. Clean local install/build/test now succeeds.

## Implemented application and architecture

- Synthetic null-scheme SUCI/subscriber provisioning, MILENAGE AUTN/SQN and AUTS, RES*/XRES*/HRES*, required 5G key derivation, independent UE/AMF NIA2/NEA2 contexts and directional COUNTs, encoded registration and protected deregistration.
- Shared `smf::Smf` for both production and independent validation: configurable DNN/S-NSSAI/capacity, protected service/resource rejects, IPv4 allocation, N2-gated activation, TEIDs, checked GTP-U/IPv4/ICMP echo, loss/latency/counters and release.
- Bounded UMi/UMa LOS/NLOS path loss, footprint obstruction, seeded shadowing, sector configuration, thermal noise/SINR, deterministic waypoints and explicitly labeled A3-style reassociation.
- Existing map workspace, entity/protocol inspectors, playback controls, scenario policies, results/CSV/JSON, named saves, checkpoint/replay/comparison and version migrations.
- Bounded schemas/geometry/queues/traces, transactional loads, stale-worker-command protection, worker recovery, configurable geographic providers, CSP/security headers and static-host configuration.

React/MapLibre/deck.gl → Worker → WASM remains intact. Shared Rust remains browser-independent. `registration`, `auth`, `session`, `smf`, `radio`, `transport`, `experiment` and `simulation/control` separate procedure, crypto, codec, policy and scheduling responsibilities. Transport exposes its next delivery time; due deliveries precede timer expiry at equal timestamps. Controller/radio/mobility/transport/metrics/executor traits remain available; the supplied no-op controller is explicitly a baseline, not AI. See [architecture](architecture.md).

## Exact run and build commands

Supported verification environment: Node 24.18.0, npm, Rust 1.97.1, wasm-pack 0.15.0. Go 1.27.2 is needed only for the independent validator; Python 3 is used for audit/vector checks.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
rustup toolchain install 1.97.1 --component rustfmt --component clippy --target wasm32-unknown-unknown
cargo install wasm-pack --version 0.15.0 --locked
npm ci
npm run build:wasm
npm run dev
```

Open http://127.0.0.1:5173. Preserve lockfiles and normal peer resolution. Static production verification:

```sh
npm run build
node scripts/verify-build.mjs
node scripts/serve-dist.mjs
```

Open http://127.0.0.1:4173. Only `dist/` is deployable. After authorization, configure provider URLs/attribution, HTTPS, `_headers`/`_redirects`, JavaScript and WASM MIME types, uncached HTML/config and immutable hashed assets. Confirm actual host responses and run application smoke tests on that host. See [deployment guide](deployment.md). No backend, native sockets or cloud account is needed for emulation.

Examples: [minimal](../examples/minimal.json), [AUTS recovery](../examples/auts-resynchronization.json), [timed signaling recovery/N2 failure](../examples/signaling-recovery.json), and [legacy profile-2 checkpoint](../examples/session-policy-experiment.json). The current `reference_experiment` example generates and replays profile 3.

## Verification evidence

The original MVP baseline passed 14 native tests and actual WASM/worker checks before the initial upgrade. The prior suite was run again before this follow-up. These are the latest local results, not inferred CI results:

| Check | Observed result |
|---|---|
| Native Rust | **66 passed**: 35 unit, 10 experiment, 2 RF, 4 reliability, 3 crypto, 7 signaling, 5 SMF |
| Independent Go/free5GC peer | **13 passed**, plus six AUTS negative subcases; both registration roles, shared SMF, timers and failed N2 |
| Independent codec round trips | **30 byte-exact NAS/NGAP fixtures** |
| Production Chromium/Firefox | **22 passed**, 11 per browser |
| Actual WASM, production worker, native↔WASM replay | Passed, including AUTS, session traffic/cleanup, ordering, offline computation, checkpoint continuation and stale-command isolation |
| Native deterministic mutations | 12,000 inputs / **144,000 parser/authentication calls**; 254 allocation/release cycles; queue saturation and malformed transport |
| Coverage-guided native fuzzing | **837,065 executions / 301 seconds**, no crash or sanitizer finding; seed 501, nightly-2026-07-20, 38,113 coverage edges (not a coverage percentage) |
| Real-WASM extended soak | **16,384 cycles / 327,680 echo requests**, 103,438.37 ms wall time; 2,097,152-byte maximum linear memory; **zero growth after 32 warm-up cycles** |
| Clean source-copy install/build/test | Passed without copied node_modules, dist, target or generated WASM; Node 24.18.0 / Rust 1.97.1 |
| TypeScript, Vite, ESLint, rustfmt, Clippy `-D warnings` | Passed |
| Independent Python KDF fixtures | Regenerated and byte-compared successfully |
| Build reproducibility/artifact verification | Two consecutive builds match **38 artifact hashes**; WASM magic and forbidden source artifacts checked |
| npm audit | **0 known vulnerabilities** |
| Rust main/fuzz audits | No vulnerability findings; visible unmaintained `paste` warning RUSTSEC-2024-0436 |
| Go validation audit | Explicit isolated-tool exception GO-2022-1083; no additional reachable finding |
| Licenses | **426 locked packages**, plus nine standalone Go dependencies with retained notices; no AGPL integrated |
| Internal ASVS Level 1 review | 70 decisions/evidence references; **4 pending host requirements**; not certification |
| Remote candidate CI / hosted deployment | **Not verified for this tree** |

Initial browser runs exposed real import/checkbox timing problems and outdated expectations, which were fixed. A run during heavy compilation also hit three Chromium boot timeouts; the subsequent full run passed without increasing the timeout. Successful final runs do not erase those earlier failures. Storage errors are injected, not a claim of exhausting real disk quota. The five-minute fuzzer and 103-second extended soak are finite campaigns, not day-long reliability certification. Fuzz tooling is isolated from browser assets; six permissively licensed tooling dependencies and their NCSA obligation were reviewed.

See [verification commands and evidence](verification.md), [machine-readable results](reliability-results.json), [peer boundary](interop-validation.md), and [ASVS decisions](asvs-assessment.json).

## Protocol references and supported scope

Release 18 subset: **TS 24.501 18.7.0**, **TS 33.501 18.5.0**, **TS 23.502 18.5.0**, **TS 38.413 18.4.0**. AUTS additionally follows **TS 33.102 18.0.0**; MILENAGE values use **TS 35.208 18.0.0**; the NEA2 fixture uses **TS 33.401 18.0.0**. NIA2 has an independent free5GC vector; KDF expectations use independently computed Python HMAC fixtures.

Timer scope: normal non-emergency WB-N1 initial registration; T3560/T3550=6 s, T3510=15 s, T3511=10 s, default T3502=720 s; T3592=16 s, four retries/fifth abort. N2 setup timeout (10 s), missing N2 release cleanup and echo timeout (2 s) are explicitly laboratory behavior. UE retry releases collocated virtual N1 state, not a full RRC exchange. Deregistration is atomic after completed session release.

Independent peers use free5GC libraries and independently derived keys against the real shared Rust state machines. They are authored test peers, **not a running unmodified free5GC/Open5GS/UERANSIM core**. No complete SMF, SBI or PFCP interoperability claim is made. See [procedure matrix and primary references](protocol-support.md).

## RF validation and measured performance

TR 38.901 **18.0.0**, Table 7.4.1-1 UMi/UMa subset. At 100 m horizontal distance/3.5 GHz, LOS fixtures are UMi **85.314188 dB** and UMa **83.1381567 dB**. Sixteen independent published ns-3.47 numerical references match within 0.0001 dB. Tests cover domains/breakpoints, NLOS ordering, antenna effects, seeded shadowing, interference, movement and reassociation. This is equation validation, not field calibration or a live ns-3 comparison. Missing building data assumes LOS; heights/materials/terrain/correlated fading are not modeled. See [RF validation](radio-validation.md).

Apple M3 Pro, macOS arm64; 10 runs, **16 gNodeBs / 128 UEs / 467 buildings / 1,225 grid samples**, UMa. Timings are **wall-clock**, not simulated latency:

| Metric | Native release, Rust 1.97.1 | WASM, Node 24.18.0 |
|---|---:|---:|
| RF/engine initialization median | 22.45 ms | 46.03 ms |
| RF/engine initialization maximum | 25.14 ms | 77.29 ms |
| 1,000 virtual-ms registration advance median | 13.31 ms | 19.32 ms |
| Native peak RSS / sampled Node RSS | 10,174,464 bytes | 510,263,296 bytes |
| WASM linear memory | — | 6,488,064 bytes |
| Serialized snapshot | 1,038,551 bytes | 1,038,013 bytes |

Worker reload (validation + RF + serialization + transfer) measured **49.07 ms median / 53.90 ms maximum**; sampled whole-process RSS **569,049,088 bytes**. Native RSS measures the already-built executable, excluding compilation. Node RSS includes host/JIT allocations and is not browser peak memory. The extended soak separately sampled RSS at 49,872,896 bytes; cross-process RSS differences are not a leak trend. Browser painting latency and peak tab memory remain unmeasured.

Commands: `npm run benchmark`, `npm run benchmark:wasm`, `node scripts/test-worker.mjs --benchmark`. Existing bounded guards (<64 MiB WASM linear memory; <5 seconds initialization) pass. Motion updates UE links incrementally; editing reconstructs the RF grid. No fidelity is reduced automatically. See [exact numbers and history](benchmark-results.json).

## Release gates and remaining limitations

| Gate | Status |
|---|---|
| Clean local static boot, real browser WASM, concurrent authenticated registration and enforced NAS security | PASS for documented laboratory subset |
| PDU session establishment, encoded N2, virtual packets, failure/teardown and protocol inspection | PASS for one-session IPv4 subset |
| Movement affects radio/state; equations validated and limitations documented | PASS for specified models and reassociation |
| Save/restore/import/export/replay and transactional invalid-input handling | PASS for bounded supported actions/profiles |
| Local automated validation and applicable local security checks | PASS within the explicit test/assessment scope |
| Green remote CI for the exact candidate commit | OPEN — revised workflow and lockfile fix not run remotely |
| Authorized production HTTPS deployment/provider verification | OPEN — no deployment authorized/performed |
| Stable v1.0 release eligibility | **FAIL / BLOCKED until external gates pass** |

Deliberate limitations remain visible: no production subscriber credentials; no RRC/AS security/PHY/NG setup/SCTP/internet forwarding/SBI/PFCP; one IPv4 session per UE, fixed QFI/5QI and no throughput enforcement; limited timer/failure variants; no protocol handover or automatic mobility reconnection; no RSRQ, terrain or height/material-aware RF; no WebKit/mobile certification, field calibration or measured browser-paint/peak-memory budget; bounded histories/journals and pool exhaustion after 254 allocations until reset; synchronous bounded replay/grid computation in a worker; advisory extension traits and no-op controller rather than a trained policy. Large geography bundle warnings remain.

These limits are not renamed successes. Expansion beyond the documented research subset requires further implementation and validation. The candidate stays **1.0.0-rc.1**. Commit/push is now authorized for candidate validation. Deployment/publication remains withheld pending host selection and separate authorization.
