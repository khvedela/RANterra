# Implementation checklist

Baseline: the original 14 native tests, actual WASM and worker tests passed before the initial upgrade. The current follow-up also ran the existing suite before changes. No repository AGENTS.md was present.

- [x] Preserve React/MapLibre/deck.gl, shared native Rust, Worker and WASM boundaries.
- [x] Synthetic AKA/NAS security, independent protocol peers and AUTS recovery.
- [x] Shared SMF policies, reservations, N2-gated user plane and cleanup.
- [x] Normal registration retransmission/retry timers and unconfirmed-state handling.
- [x] Encoded N2 failed-resource response and network-initiated release.
- [x] Browser timed session signaling, deterministic loss injection and pending controls.
- [x] Profile-3 checkpoint/continuation; profile-2 compatibility retained.
- [x] RF numerical validation and bounded native/WASM/worker benchmarks.
- [x] Local clean build, 66 Rust tests, 13 independent peers, 30 codec frames, 22 browser tests.
- [x] Sanitizer fuzzing, extended WASM soak, input/storage hardening and internal ASVS Level 1 review.
- [x] Repair clean-install peer dependencies, pin CI actions, audit dependencies/licenses and verify reproducible artifacts.
- [ ] Green remote CI for the exact candidate commit; previous remote run failed before the lockfile fix.
- [ ] Authorized HTTPS deployment and actual host/provider verification.
- [ ] Remote candidate validation after authorized commit/push; stable 1.0.0 metadata/tag/publication remain gated.

Checkmarks apply to the explicitly bounded research profile, not every conditional 3GPP requirement. Complete-core/SBI/PFCP interoperability, protocol handover, real credentials, multiple sessions, field RF calibration and a trained controller are not claimed. Candidate metadata remains 1.0.0-rc.1. See [release report](V1_RELEASE_REPORT.md) and [validation boundaries](interop-validation.md).
