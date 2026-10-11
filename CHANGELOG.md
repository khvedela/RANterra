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
