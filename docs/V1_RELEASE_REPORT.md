# RANterra v1 release report

**2026-10-11 — candidate 1.0.0-rc.1, target 1.0.0. Release acceptance: BLOCKED on hosted verification and revalidation of the Cloudflare routing correction.** The Ant Design candidate `5f13d43` passed both remote CI jobs and the local checks described below. Cloudflare hosting is now authorized; account authentication and hosted verification are pending. Release publication remains on hold.

## Cloudflare hosting follow-up

User authorization now covers a free Cloudflare Pages deployment, with `ranterra.davidkh.com` proposed as the custom domain. Both the dashboard session and the existing CLI authorization were unavailable; a phone-compatible OAuth device flow was offered. No deployment or DNS changes have yet been performed.

Wrangler 4.149.0 independently exposed two previously unverified production-host defects: the broad rewrite is rejected as a loop, and default SPA fallback returns HTML/200 for missing assets. Explicit `/workspace` and `/experiment/*` rewrites to `/`, plus a top-level `404.html`, fix both behaviors without a server/backend. The local server now reads the actual rewrite file; browser tests additionally check missing WASM, unknown routes and workspace refresh. Tests can target the real hosted origin while still blocking external geographic traffic. The earlier 13-artifact CI result below remains historical evidence for the Ant candidate; the routing correction adds a 14th artifact. All 30 production browser checks passed against Wrangler Pages dev in 2.4 minutes; strengthened real worker/WASM MIME and magic assertions also passed in Chromium and Firefox. The local verifier passed its routing checks, lint/types passed, and web/full Rust/WASM builds reproduce all 14 hashes. Remote CI revalidation and actual HTTPS/custom-domain verification remain pending.

## Ant Design workspace upgrade

The interface now uses Ant Design 6.6.5 following the requested design guidance: token-based neutral surfaces, system typography, accessible darker blue primary actions, consistent forms/buttons, cards, navigation, state badges, metric statistics, a per-cell results table, focus-safe dialogs and responsive setup/inspector drawers. The real MapLibre/deck.gl map, protocol inspector, scenario workflow and Rust/WASM engine remain connected. The [updated preview](preview.jpg) shows four authenticated UEs and a real established session/echo; geographic requests were disabled and loaded geometry was used.

Validation found and fixed passive map annotations intercepting UE clicks, RF overlays tinting controls/markers, checkpoint restore racing an export before its asynchronous IndexedDB read, and mobile drawers opening above the viewport after page scrolling. Dialogs now complete the Ant close lifecycle to restore focus, with an explicit native Tab boundary guard. Small/short layouts retain the timeline and editing controls rather than clipping them. Dialog command hit targets stay stationary by omitting scale animation. Automated accessibility checks have no serious/critical findings for the tested WCAG A/AA rules; this is not a full accessibility certification.

Evidence for this UI change: the **24-test production browser baseline passed**, then the expanded **30 checks passed in 2.3 minutes** (15 Chromium, 15 Firefox, including 390×844 responsive editing after page scrolling, focus/Escape/return and axe workspace/results scans). `npm test` passed **66 Rust tests**, actual WASM, production worker and native↔WASM replay; provider-input tests, ESLint, TypeScript, full Rust/WASM production build and npm audit passed. All **13 production artifact hashes** matched between the web build and the full Rust/WASM build. Platform fonts remove the 26 former font artifacts. **492 dependency licenses** are reviewed; unmodified MPL-2.0 axe packages are permitted only as development validators, with obligations documented in [UI design](ui-design.md). Runtime Ant dependencies remain within the permissive license policy; the deployed `ANT_DESIGN_NOTICES.txt` retains copyright and license text for 66 introduced runtime package paths.

A corrective remote run at `ad24862` ([38155390325](https://github.com/khvedela/RANterra/actions/runs/38155390325)) passed 29/30 browser checks but failed exact checkpoint equality in Firefox. Its retained trace showed the opening zoom moving a restore click onto the neighboring save action; this overwrote the checkpoint with the initial state. The UI omits dialog scale animation, retains the exact replay assertion, and passes five repeated restore/export checks in each browser (**10/10 in 1.3 minutes**). The failed run is not counted as a full pass.

The corrected application and workflow are verified at source commit **`5f13d437f996163ba562b5f2b50685d6c3961d1b`**. The evidence-only documentation follow-up leaves that tested source and its 13 deployment assets unchanged. No deployment or publication is performed.

## Remote CI result and hosting decision

[GitHub Actions run 38156308147](https://github.com/khvedela/RANterra/actions/runs/38156308147) completed successfully on source **`5f13d437f996163ba562b5f2b50685d6c3961d1b`**: **66 Rust tests, 13 independent peers, 30 codec fixtures, 30 Chromium/Firefox checks** (3.5 minutes), actual WASM/worker/replay, lint, audits/licenses, KDF fixtures, benchmarks and **13 matching artifact hashes**. The sanitizer job completed **195,928 executions in 61 seconds**. The [candidate build artifact](https://github.com/khvedela/RANterra/actions/runs/38156308147/artifacts/11708811163) was downloaded and all 13 file hashes independently matched its manifest; WASM magic and retained Ant license notices were also checked. Its uncompressed deployment assets total **7,132,224 bytes**. GitHub reports the archive digest `sha256:f53e1e0c93be91cb29cbfa71b748660afa3e48f1b22fd0b53b8a544aa7fd8319`.

The initial Ant migration also passed [run 38153545490](https://github.com/khvedela/RANterra/actions/runs/38153545490) at `2674ef4` before the mobile and stationary-dialog corrections. The earlier pre-redesign [run 38148564181](https://github.com/khvedela/RANterra/actions/runs/38148564181) passed with 24 browser tests and 38 font-bearing artifacts; those older counts are historical. The failed corrective run described above remains part of the evidence.

The first candidate run (`9085333`, [failed run](https://github.com/khvedela/RANterra/actions/runs/38147397454)) passed Chromium but failed all Firefox startup assertions. CI now supplies an Xvfb display and records a graphics preflight: both headless and headed Firefox obtained WebGL2 with llvmpipe under that display. The full suite runs headed under Xvfb and passes. The original run did not retain page traces, so the precise original graphics failure is inferred from the environment change rather than recovered from a trace. Failure screenshots/traces are now retained. Map initialization exceptions now produce a visible warning while preserving simulation/export controls, with an explicit no-WebGL regression test. Timeouts and the normal real-map/protocol assertions were not weakened.

Host research also exposed overlapping cache headers: Cloudflare merges values, so a global `no-cache` rule conflicted with immutable assets. Rules now avoid that overlap and the local server/tests enforce the merge behavior. **Cloudflare Pages Free is recommended**, using the provided domain and prebuilt CI artifact; no rented server is necessary. This is not a deployment decision or action. See [free-host comparison and current primary sources](hosting-options.md). Actual HTTPS, headers, routing, provider configuration and hosted browser checks remain open.

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

| Check                                                   | Observed result                                                                                                                                      |
| ------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| Native Rust                                             | **66 passed**: 35 unit, 10 experiment, 2 RF, 4 reliability, 3 crypto, 7 signaling, 5 SMF                                                             |
| Independent Go/free5GC peer                             | **13 passed**, plus six AUTS negative subcases; both registration roles, shared SMF, timers and failed N2                                            |
| Independent codec round trips                           | **30 byte-exact NAS/NGAP fixtures**                                                                                                                  |
| Production Chromium/Firefox                             | **30 passed locally and remotely**, including responsive editing, focus and axe scans                                                                |
| Actual WASM, production worker, native↔WASM replay      | Passed, including AUTS, session traffic/cleanup, ordering, offline computation, checkpoint continuation and stale-command isolation                  |
| Native deterministic mutations                          | 12,000 inputs / **144,000 parser/authentication calls**; 254 allocation/release cycles; queue saturation and malformed transport                     |
| Coverage-guided native fuzzing                          | **837,065 executions / 301 seconds**, no crash or sanitizer finding; seed 501, nightly-2026-07-20, 38,113 coverage edges (not a coverage percentage) |
| Real-WASM extended soak                                 | **16,384 cycles / 327,680 echo requests**, 103,438.37 ms wall time; 2,097,152-byte maximum linear memory; **zero growth after 32 warm-up cycles**    |
| Clean source-copy install/build/test                    | Passed without copied node_modules, dist, target or generated WASM; Node 24.18.0 / Rust 1.97.1                                                       |
| TypeScript, Vite, ESLint, rustfmt, Clippy `-D warnings` | Passed                                                                                                                                               |
| Independent Python KDF fixtures                         | Regenerated and byte-compared successfully                                                                                                           |
| Build reproducibility/artifact verification             | Current builds match **13 artifact hashes**; WASM magic and forbidden source artifacts checked; previous font-bearing build had 38                   |
| npm audit                                               | **0 known vulnerabilities**                                                                                                                          |
| Rust main/fuzz audits                                   | No vulnerability findings; visible unmaintained `paste` warning RUSTSEC-2024-0436                                                                    |
| Go validation audit                                     | Explicit isolated-tool exception GO-2022-1083; no additional reachable finding                                                                       |
| Licenses                                                | **492 locked packages**, plus nine standalone Go dependencies with retained notices; no AGPL integrated                                              |
| Internal ASVS Level 1 review                            | 70 decisions/evidence references; **4 pending host requirements**; not certification                                                                 |
| Remote candidate CI                                     | **Both jobs passed** on `5f13d43`, [run 38156308147](https://github.com/khvedela/RANterra/actions/runs/38156308147)                                  |
| Hosted deployment                                       | **Not authorized or verified**                                                                                                                       |

Initial browser runs exposed real import/checkbox timing problems and outdated expectations, which were fixed. A run during heavy compilation also hit three Chromium boot timeouts; the subsequent full run passed without increasing the timeout. Successful final runs do not erase those earlier failures. Storage errors are injected, not a claim of exhausting real disk quota. The five-minute fuzzer and 103-second extended soak are finite campaigns, not day-long reliability certification. Fuzz tooling is isolated from browser assets; six permissively licensed tooling dependencies and their NCSA obligation were reviewed.

See [verification commands and evidence](verification.md), [machine-readable results](reliability-results.json), [peer boundary](interop-validation.md), and [ASVS decisions](asvs-assessment.json).

## Protocol references and supported scope

Release 18 subset: **TS 24.501 18.7.0**, **TS 33.501 18.5.0**, **TS 23.502 18.5.0**, **TS 38.413 18.4.0**. AUTS additionally follows **TS 33.102 18.0.0**; MILENAGE values use **TS 35.208 18.0.0**; the NEA2 fixture uses **TS 33.401 18.0.0**. NIA2 has an independent free5GC vector; KDF expectations use independently computed Python HMAC fixtures.

Timer scope: normal non-emergency WB-N1 initial registration; T3560/T3550=6 s, T3510=15 s, T3511=10 s, default T3502=720 s; T3592=16 s, four retries/fifth abort. N2 setup timeout (10 s), missing N2 release cleanup and echo timeout (2 s) are explicitly laboratory behavior. UE retry releases collocated virtual N1 state, not a full RRC exchange. Deregistration is atomic after completed session release.

Independent peers use free5GC libraries and independently derived keys against the real shared Rust state machines. They are authored test peers, **not a running unmodified free5GC/Open5GS/UERANSIM core**. No complete SMF, SBI or PFCP interoperability claim is made. See [procedure matrix and primary references](protocol-support.md).

## RF validation and measured performance

TR 38.901 **18.0.0**, Table 7.4.1-1 UMi/UMa subset. At 100 m horizontal distance/3.5 GHz, LOS fixtures are UMi **85.314188 dB** and UMa **83.1381567 dB**. Sixteen independent published ns-3.47 numerical references match within 0.0001 dB. Tests cover domains/breakpoints, NLOS ordering, antenna effects, seeded shadowing, interference, movement and reassociation. This is equation validation, not field calibration or a live ns-3 comparison. Missing building data assumes LOS; heights/materials/terrain/correlated fading are not modeled. See [RF validation](radio-validation.md).

Apple M3 Pro, macOS arm64; 10 runs, **16 gNodeBs / 128 UEs / 467 buildings / 1,225 grid samples**, UMa. Timings are **wall-clock**, not simulated latency:

| Metric                                       | Native release, Rust 1.97.1 | WASM, Node 24.18.0 |
| -------------------------------------------- | --------------------------: | -----------------: |
| RF/engine initialization median              |                    22.45 ms |           46.03 ms |
| RF/engine initialization maximum             |                    25.14 ms |           77.29 ms |
| 1,000 virtual-ms registration advance median |                    13.31 ms |           19.32 ms |
| Native peak RSS / sampled Node RSS           |            10,174,464 bytes |  510,263,296 bytes |
| WASM linear memory                           |                           — |    6,488,064 bytes |
| Serialized snapshot                          |             1,038,551 bytes |    1,038,013 bytes |

Worker reload (validation + RF + serialization + transfer) measured **49.07 ms median / 53.90 ms maximum**; sampled whole-process RSS **569,049,088 bytes**. Native RSS measures the already-built executable, excluding compilation. Node RSS includes host/JIT allocations and is not browser peak memory. The extended soak separately sampled RSS at 49,872,896 bytes; cross-process RSS differences are not a leak trend. Browser painting latency and peak tab memory remain unmeasured.

Commands: `npm run benchmark`, `npm run benchmark:wasm`, `node scripts/test-worker.mjs --benchmark`. Existing bounded guards (<64 MiB WASM linear memory; <5 seconds initialization) pass. Motion updates UE links incrementally; editing reconstructs the RF grid. No fidelity is reduced automatically. See [exact numbers and history](benchmark-results.json).

The Linux CI runner separately measured native initialization median/max **47.16/47.82 ms**, WASM **76.83/187.69 ms**, and registration median **18.15 ms native / 34.63 ms WASM**. WASM linear memory was **6,488,064 bytes**; sampled Node RSS **481,226,752 bytes**. CI's 128-cycle/2,560-echo soak took **1,834.42 ms** with zero post-warm-up growth. These shared-runner results are not substituted for local-machine figures or a rendering SLA.

## Release gates and remaining limitations

| Gate                                                                                                        | Status                                                                               |
| ----------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| Clean local static boot, real browser WASM, concurrent authenticated registration and enforced NAS security | PASS for documented laboratory subset                                                |
| PDU session establishment, encoded N2, virtual packets, failure/teardown and protocol inspection            | PASS for one-session IPv4 subset                                                     |
| Movement affects radio/state; equations validated and limitations documented                                | PASS for specified models and reassociation                                          |
| Save/restore/import/export/replay and transactional invalid-input handling                                  | PASS for bounded supported actions/profiles                                          |
| Local automated validation and applicable local security checks                                             | PASS within the explicit test/assessment scope                                       |
| Green remote CI for the candidate application/workflow                                                      | **PENDING** for the routing correction; previous Ant candidate passed at `5f13d43`   |
| Authorized production HTTPS deployment/provider verification                                                | OPEN — deployment authorized; account authentication and hosted verification pending |
| Stable v1.0 release eligibility                                                                             | **FAIL / BLOCKED until routing CI and hosted verification pass**                     |

Deliberate limitations remain visible: no production subscriber credentials; no RRC/AS security/PHY/NG setup/SCTP/internet forwarding/SBI/PFCP; one IPv4 session per UE, fixed QFI/5QI and no throughput enforcement; limited timer/failure variants; no protocol handover or automatic mobility reconnection; no RSRQ, terrain or height/material-aware RF; no WebKit/mobile certification, field calibration or measured browser-paint/peak-memory budget; bounded histories/journals and pool exhaustion after 254 allocations until reset; synchronous bounded replay/grid computation in a worker; advisory extension traits and no-op controller rather than a trained policy. Large geography bundle warnings remain.

These limits are not renamed successes. Expansion beyond the documented research subset requires further implementation and validation. The candidate stays **1.0.0-rc.1**. Candidate and corrective commits are pushed. Cloudflare deployment is authorized but awaits account access and hosted verification. Release publication is still withheld.
