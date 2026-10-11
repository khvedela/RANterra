# Changelog

## 1.0.0-rc.1 — 2026-10-11 (unreleased)

Target release: 1.0.0. Release eligibility remains blocked; see `docs/V1_RELEASE_REPORT.md`.

- Replace unconditional registration rejection with real synthetic 5G-AKA, NIA2/NEA2 security, Registration Accept/Complete and protected deregistration.
- Add independent subscriber contexts, laboratory failure injection and cryptographic/reference tests.
- Add protected N1 session signaling, NGAP resource setup/release, virtual IPv4/ICMP echo over GTP-U and logical context cleanup.
- Add selectable UMi/UMa path loss, SINR, sector configuration, seeded shadowing and deterministic waypoint mobility with explicitly labeled reassociation.
- Extend inspectors, playback, results/CSV, named local copies and negative-case controls.
- Add configurable geography, bounded responses/caches/command queues, worker recovery, strict scenario fields, CSP/static hosting headers, production browser automation and CI.
- Preserve legacy version-1 RF semantics and original architecture. No external deployment or release publication.

### Follow-up: configurable sessions and portable replay

- Added scenario-v2 DNN/S-NSSAI/capacity policies and per-UE service requests, protected cause-91 failures, 5GSM resource rejects and enforced virtual backoff.
- Added bounded Rust command recording, checked portable replay, active session/in-flight packet restoration, explicit local checkpoints and imported-run comparison; retained scenario-v1 and IndexedDB migration.
- Fixed mobility/registration timeout ordering, unchanged-velocity resets and playback-speed retention.
- Independent free5GC checking detected and fixed the upstream NGAP RRC establishment IE ID; added N2 session AMBR. Added 16 ns-3 numerical RF reference checks and native↔WASM replay validation.
- Candidate remains unreleased. No external deployment or publishing.

### Independent endpoint and reliability follow-up

- Add an independent Go/free5GC peer for bidirectional registration/security, concurrent subscriber isolation, COUNT rollover, negative authentication and session/packet primitive validation.
- Fix plaintext Security Mode Command acceptance; require endpoint-local registration state for protected session operations.
- Validate N1 payload type, packet size, IPv4 fragmentation/TTL and downlink checksums; isolate malformed/unknown transport frames and expire lost echoes deterministically.
- Reject worker UE IDs outside u32 and stale messages before queue accounting.
- Add deterministic packet mutation, address-pool lifecycle, queue-overflow and real-WASM allocation/free soak checks. Include validation test dependencies in audit/license review.
- Stable v1 remains blocked; no deployment/publication or Git push.

### SMF interoperability and AUTS follow-up

- Share the production SMF state machine with the independent peer adapter; validate allocation, policy rejects/backoff, pending resources, gNodeB-selected DL TEIDs, traffic and release acknowledgements.
- Add deterministic SMF T3592 retries/abort and bounded N2 timeout cleanup at the directly driven protocol boundary. The initial follow-up kept browser commands atomic; the timed-signaling follow-up below replaces this for new runs.
- Implement AUTS generation/verification using MILENAGE f1*/f5*, fresh challenge issuance, bounded SQN recovery and replay/invalid-MAC protection. Add the browser `SqnOutOfSync` experiment and preserve deterministic replay.
- Cache only an outstanding authentication response for request retransmission; clear it on Security Mode success. The follow-up below adds normal initial-registration retransmission timers.

### Registration recovery, timed signaling and release validation

- Implement normal WB-N1 T3560/T3550 retransmission, T3510/T3511/T3502 retry, fresh NAS counters and explicit unconfirmed-registration state.
- Handle encoded failed N2 resource setup with protected PTI-0 release and deterministic reservation cleanup.
- Schedule browser session N1/N2 messages with configurable delay/loss; expose pending controls and T3592 recovery.
- Introduce replay profile 3 including future-affecting configuration in its digest; preserve profile-2 recordings and legacy execution.
- Fix import/control acknowledgement races and update the reference experiment for asynchronous session completion.
- Harden provider validation/cache identity, IndexedDB transactions, static headers and artifact checks; assess ASVS Level 1 with host gates pending.
- Add sanitizer fuzzing and extended real-WASM soak; pass 66 native tests, 13 independent peers, 30 codec fixtures and 22 production browser tests.
- Repair missing npm peer dependencies and enforce normal peer resolution; pin CI actions and add isolated fuzz CI.
- Keep 1.0.0-rc.1: remote candidate CI and authorized HTTPS deployment verification remain open. No commit/push/deployment/publication.

## 0.1.0

Original Rust/WASM registration-request/reject MVP with map workspace, approximate RF, OSM geometry, deterministic transport and local persistence.
