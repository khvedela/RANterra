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

## 0.1.0

Original Rust/WASM registration-request/reject MVP with map workspace, approximate RF, OSM geometry, deterministic transport and local persistence.


### Follow-up: configurable sessions and portable replay

- Added scenario-v2 DNN/S-NSSAI/capacity policies and per-UE service requests, protected cause-91 failures, 5GSM resource rejects and enforced virtual backoff.
- Added bounded Rust command recording, checked portable replay, active session/in-flight packet restoration, explicit local checkpoints and imported-run comparison; retained scenario-v1 and IndexedDB migration.
- Fixed mobility/registration timeout ordering, unchanged-velocity resets and playback-speed retention.
- Independent free5GC checking detected and fixed the upstream NGAP RRC establishment IE ID; added N2 session AMBR. Added 16 ns-3 numerical RF reference checks and native↔WASM replay validation.
- Candidate remains unreleased. No external deployment or publishing.
