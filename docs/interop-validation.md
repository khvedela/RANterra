# Independent endpoint and reliability validation

The 2026-10-11 follow-up adds an independently authored Go test peer to the existing codec round trips. It runs as a local subprocess over bounded stdin/stdout; it does not launch a 5G core, open network sockets, deploy, or use production subscriber credentials.

## Independent peer boundary

`engine/examples/interop_endpoint.rs` delegates to the application's actual Rust registration endpoints, NAS security contexts, NGAP envelopes, shared SMF lifecycle and session/packet codecs. It contains no authentication shortcut or alternate cryptography. The peer in `validation/free5gc/peer_test.go` receives wire PDUs and uses public synthetic credentials. It independently verifies AUTN, derives RES* and the 5G key hierarchy, verifies/decrypts NAS, and generates protected responses. The adapter never exports derived keys or decrypted replies during registration.

Pinned independent components: free5GC NAS 1.1.5, NGAP 1.0.10, util/MILENAGE 1.0.7, and Go 1.27.2. MILENAGE and NAS NIA2/NEA2 come from these libraries; the KDF construction uses Go's standard HMAC-SHA-256. No custom AES, MAC or hash primitive was introduced. Rust continues to use OxiRush/MILENAGE, not the Go implementations.

| Test | Evidence and scope |
|---|---|
| `TestIndependentUEAgainstRustAMF` | Three subscriber procedures interleaved by stage; independent SUCI/NAS/NGAP, AUTN/RES*, protected Security Mode Complete and Registration Complete. Each subscriber then exchanges 260 protected UL/DL messages across COUNT rollover. Corrupt MACs, replay and cross-subscriber packets are rejected before the valid packet is accepted. |
| `TestRustUERejectsPlaintextSecurityMode` | Independent network-generated authentication vector; Rust UE RES* checked in Go; plaintext and wrong-direction Security Mode Commands rejected, then valid protected command/complete and accept/complete succeed. |
| `TestIndependentSessionAndUserPlane` | Protected UL service request, Rust N1/N2 setup, independent decoded IPv4/slice/TEID and required-transfer checks, Go-generated N2 response, and independently constructed GTP-U/IPv4/ICMP request/reply with checksums. Wrong TEID and wrong N2 IE order are rejected. |
| `TestIndependentWrongAuthenticationResponse` | Incorrect RES* produces Authentication Reject and leaves no usable AMF security context. |
| `TestRustUERejectsIndependentInvalidAUTN` | Corrupted independent AUTN produces Authentication Failure cause 20 and leaves no usable UE security context. |

The peer now drives the same `smf::Smf` used by `simulation::Engine`; the adapter contains no allocator, policy engine or fabricated success path. It provisions policies before registration, then submits authenticated N1 requests and independently encoded N2 acknowledgements. Addresses and uplink TEIDs are read from received protocol bytes, and the peer chooses its own downlink TEID. Packet forwarding is unavailable before N2 acknowledgement and after release begins.

Additional peer tests:

- `TestIndependentSMFLifecyclePoliciesAndCleanup`: custom DNN/SST/SD, unsupported-service cause 91 without allocation, pending-capacity cause 26/T3396, duplicate pending request without reallocation, gNodeB-selected TEIDs, wrong contexts, traffic, release in both completion orders, backoff, retry, stale-tunnel isolation and allocation continuity.
- `TestIndependentSMFT3592AndSetupTimeout`: missing N2 setup response times out, late response cannot activate a tunnel, release-request retry does not extend T3592, four protected retransmissions use fresh NAS counts, and fifth expiry aborts. Completion after abort is rejected; resources can be allocated again.
- `TestIndependentAUTSResynchronizesRustAMF`: independently generated AUTS recovers SQN 100, obtains a fresh RAND/SQN 101 and completes registration with independently derived keys.
- `TestIndependentAUTSNegativeCases`: bad MAC-S, wrong RAND, cross-subscriber AUTS, old challenge replay, 48-bit exhaustion and truncation cannot authenticate.
- `TestIndependentNetworkVerifiesRustAUTSAndAuthRetransmission`: independent network verifies the Rust UE's AUTS, completes resynchronization and registration, checks cached RES* for the still-outstanding repeated challenge, and rejects challenge replay after Security Mode.
- `TestIndependentAUTSBoundedRepeatedResynchronization`: two validated resynchronizations are allowed; a third is rejected to bound the laboratory procedure.
- `TestIndependentRegistrationRetransmissionTimers`: independent decoding/response to AMF retransmissions, protected COUNT progression, T3560 exhaustion and T3550 unconfirmed state.
- `TestIndependentFailedN2ResourceCleanup`: an independently encoded failed resource list/cause, PTI-0 protected release and resource cleanup without activating traffic.

The Rust-UE peer also receives fresh protected repeated Security Mode and Registration Accept messages and verifies its retransmitted completions.

The Go peer is an authored test implementation using free5GC libraries, **not a running free5GC AMF/SMF or an unmodified UERANSIM/Open5GS peer**. This closes independent allocation/policy/release validation for the documented one-session profile. It does not establish interoperability of a complete commercial SMF. Radio, RRC, AS security, NG setup, SCTP, SBI and N4 are absent. The supported N2 failed-resource cause and normal initial-registration timers are now validated; multiple sessions and full conditional registration machinery remain outside scope.

## Timer and resynchronization scope

T3592 follows TS 24.501 §6.3.3.5 and Table 10.3.2: ordinary terrestrial value **16,000 virtual ms**, four retransmissions, abort on the fifth expiry. `Smf::advance` processes deadlines in `(time, UE ID)` order and returns clear N1 PDUs for the caller to protect; the independent adapter verifies fresh NAS counts. Callers must advance the SMF clock before delivering messages at a new timestamp. N2 setup uses a **10,000 ms laboratory deadline**. After NAS Release Complete, T3592 no longer retransmits; an outstanding N2 acknowledgement is cleaned up at the existing release deadline as an explicitly local timeout. Both release acknowledgements are required before normal reservation cleanup.

Profile-3 browser session control now uses configurable virtual delivery stages and bounded message-loss injection. T3560 and T3550 use 6-second intervals, four retransmissions and a fifth-expiry outcome; protected retries advance NAS COUNT. T3550 fifth expiry marks the AMF registered but the UI explicitly unconfirmed. T3510=15 seconds, T3511=10 seconds and default T3502=720 seconds model normal WB-N1 initial registration; the fifth failed attempt enters T3502. The UE releases its local virtual N1 context before retry, preserves SQN and obtains a new challenge. There is no full RRC connection-release exchange. Legacy profile-2 recordings retain their old behavior. Registration retains seeded 10–30 ms per-hop transport; configurable session delay aggregates each N1/N2 delivery stage and does not claim physical radio latency.

AUTS uses vetted MILENAGE f1*/f5*, AMF=0000 and constant-time MAC-S verification against the outstanding RAND. The network consumes that challenge, advances the recovered SQN without 48-bit wrap and issues a new vector/RAND. The simplified USIM freshness policy is strictly monotonic, without an IND array or operator-specific acceptance window. `SqnOutOfSync` provisions synthetic USIM SQN 100; import `examples/auts-resynchronization.json` or select the laboratory setting. Actual WASM and browser tests exercise the recovery, not a UI mock. Published TS 35.208 f1*/f5* values and an independent free5GC AUTS fixture supplement the interactive checks.

## Issues fixed

- The UE previously accepted the exact expected plaintext Security Mode Command after AKA because the receive path only invoked integrity verification when the header already claimed protection. That stage now requires security header type 3, with the supported plaintext Authentication Reject exception. Both independent and native tests prove a rejected forged command does not consume the valid security context.
- Session security operations now check the sending/receiving endpoint's own registration state, permitting an independent peer without accessing the other local endpoint's state. AMF registration success still requires a verified Registration Complete.
- N1 dispatch now checks payload-container type before interpreting the contents as 5GSM. Session/NAS parsing has explicit size bounds.
- Echo validation checks IPv4 and ICMP checksums, routing, fragmentation flags and TTL in both directions. Unsupported fragments and malformed replies cannot count as delivered traffic.
- Unknown transport UE/cell/route inputs are rejected before indexing state. Malformed frames are traced and dropped without stopping unrelated UEs. Unsolicited registration PDUs cannot tear down an already registered endpoint. A lost/corrupt echo expires after **2,000 virtual ms**, releases its in-flight slot and increments dropped traffic. This is a laboratory delivery deadline, not a standardized 5G timer. Valid normal-transport replay remains compatible with engine profile `ranterra-lab/2`.
- The worker rejects UE IDs above `u32::MAX` before WASM conversion and ignores already-stale revisions before queue accounting. This prevents a large JavaScript integer from wrapping into another UE's ID.

One peer-development failure was an incorrectly ordered N2 response. Inspection of TS 38.413 §9.4.1 confirmed that senders must use the object-set order. The peer was corrected; the engine retains order and multiplicity rejection. Decoder acceptance alone is not evidence of semantic validity.

## Bounded reliability campaign

`engine/tests/reliability.rs` covers 12,000 deterministic truncation/bit-mutation/extension/random inputs over six seed packet types, checked against twelve parser/authentication paths (144,000 calls), including wire service selection, downlink TEID/release validation and AUTS; 254 session allocation/release cycles through address-pool exhaustion; a full 128-packet in-flight queue and transactional overflow; and injected malformed transport frames alongside valid independent UEs. Counters, cleanup, deadlines and replay are asserted. Mutation tests require no parser panic; they do not assume every mutated packet must be invalid.

`npm run test:soak` runs real WASM through 128 create/register/session/20-packet/release/deregister/replay/free cycles (2,560 original echo requests). Each checkpoint is replayed into a second instance, each instance is freed, and rejected loads preserve the live engine. It records elapsed wall time, WASM linear-memory high-water marks and sampled whole-process RSS in `test-results/wasm-soak.json`. Limits are <64 MiB linear memory and ≤4 MiB growth after 32 warm-up cycles. These are bounded regression budgets, not proof of an absence of leaks or browser-tab peak memory.

An additional AddressSanitizer/libFuzzer run completed **837,065 executions in 301 seconds**, seed 501, with no crash or sanitizer finding (38,113 coverage edges, 67,726 features; not a percentage of the application). The corpus grew to 7,878 entries; observed fuzzer RSS was 573 MiB. This is a five-minute campaign, not an exhaustive fuzz proof. The extended real-WASM run completed **16,384 cycles / 327,680 echo requests** in 103.438 seconds with zero linear-memory growth after warm-up. Browser tests inject quota and denied-storage failures; actual disk exhaustion, all browser storage failures and WebKit/mobile platforms remain unverified. The applicable ASVS Level 1 assessment is complete locally, with hosting requirements explicitly pending.

## Reproduce

```sh
cargo test --workspace --locked
npm run test:protocols
npm run build
npm run test:wasm
npm run test:worker
npm run test:replay
npm run test:browser
RANTERRA_SOAK_CYCLES=16384 npm run test:soak
# See validation/fuzz/README.md for the isolated nightly toolchain setup:
sh scripts/fuzz-protocols.sh 300
python3 scripts/check-validation-licenses.py
python3 scripts/audit-validation.py
```

Run the soak after Playwright: Playwright clears its `test-results` directory at startup. The Go suite uses an uncached test run and a 60-second subprocess deadline. All nine linked validation dependencies, including test-only crypto packages, have reviewed retained notices; the license and vulnerability checks include the test graph. The known isolated APER advisory remains an explicit exception; it is not linked into application assets.

Primary references: [TS 24.501 18.7.0](https://www.etsi.org/deliver/etsi_ts/124500_124599/124501/18.07.00_60/ts_124501v180700p.pdf), §§4.4 and 5.4.2; [TS 38.413 18.4.0](https://www.etsi.org/deliver/etsi_ts/138400_138499/138413/18.04.00_60/ts_138413v180400p.pdf), §§8.2.1 and 9.4.1; [free5GC NAS security source](https://github.com/free5gc/nas/tree/v1.1.5/security); [free5GC MILENAGE source](https://github.com/free5gc/util/tree/v1.0.7/milenage). The supported specification release has not changed.

Additional primary references: [TS 33.102 18.0.0, §6.3.3](https://www.etsi.org/deliver/etsi_ts/133100_133199/133102/18.00.00_60/ts_133102v180000p.pdf) for AUTS construction; [TS 35.208 18.0.0, §4.3.1](https://www.etsi.org/deliver/etsi_ts/135200_135299/135208/18.00.00_60/ts_135208v180000p.pdf) for f1*/f5* test set 1; [free5GC util v1.0.7 AUTS fixture](https://github.com/free5gc/util/blob/v1.0.7/milenage/milenage_test.go). No cryptographic dependency was added.
