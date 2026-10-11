# Implementation checklist

Baseline 2026-10-11: clean checkout; 14 native tests, actual WASM runtime and production worker tests passed before edits. Audited Rust core, browser adapters, React/map UI, storage, geography, scripts, manifests, examples and existing documentation. No repository AGENTS.md found.

- [x] A: audit, architecture review and baseline.
- [x] B: real lab AKA, independent NIA2/NEA2 contexts, encoded registration/deregistration and negative tests.
- [x] C: bounded encoded PDU resource lifecycle and virtual GTP-U/IP echo.
- [x] D: TR path-loss subset, waypoint mobility/reassociation, equation checks and native/WASM benchmarks.
- [x] E: session/entity inspector, protocol filters, experiment controls, dashboard/exports and local copies.
- [x] F: input/provider/security hardening, browser automation, CI, static deployment configuration and release report.

These checkmarks record implementation passes, not satisfaction of every requirement in each phase. [The release report](V1_RELEASE_REPORT.md) lists reduced scope and unmet gates: full external state-machine interoperability, field RF calibration, extended reliability verification and hosted deployment. The follow-up implements configurable session policies, portable replay/migrations, independent codec validation and external RF numerical reference checks. Candidate metadata deliberately remains 1.0.0-rc.1. No publication or external deployment performed.
