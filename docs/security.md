# Security and privacy

RANterra is a local, single-user research application. **Use synthetic laboratory identities only. Never import real SIM credentials, production keys or personal subscriber records.** Public deterministic lab credentials and RAND values are intentionally reproducible and provide no production subscriber security.

## Trust boundaries

The UI validates command shapes; the worker enforces revisions and a bounded 256-command queue; Rust independently validates scenarios and protocol inputs before replacing state. Native engine code has no browser dependency. Imported strings render as React text. Scenario schemas reject unknown fields and secret-bearing extensions. Separate UE/AMF contexts enforce authentication, integrity, counts and state transitions; no frontend flag grants registration. Synthetic cryptographic vectors use zeroizing wrappers, but compiler/runtime/browser copies and crash dumps are not guaranteed to be erased.

The emulated AMF/AUSF/UDM, UE, SMF and UPF share a WASM instance, so this is not a security-isolation testbed. There is no secure element, hosted login, tenant boundary, TLS/SBI/SCTP service or actual internet forwarding. Anyone controlling the browser or source can inspect lab secrets. Logs contain wire payloads and synthetic identity data but do not export K, OPc, derived keys or reusable NAS contexts. Do not put sensitive data in labels, packets or imported geometry.

Bounds include 16 cells, 128 UEs, 600 buildings, 15,000 geometry vertices, 100 vertices per ring, 128 waypoints per UE, bounded packet/ASN.1 parsing, 16,384 log records, and finite numeric/radio ranges. Geographic responses are streamed with byte limits and timeouts. Invalid loads leave the existing engine intact. Session/TEID validation rejects stale traffic. Worker recovery recreates WASM and reloads configuration; it does not resurrect security keys. IndexedDB errors are reported, cache failures are recoverable, and named copies/cache entries are bounded. Storage-quota behavior depends on the browser and has not been stress-tested to exhaustion.

## Browser/deployment controls

`public/_headers` contains CSP, MIME sniffing prevention, referrer and permissions policies. CSP allows WebAssembly compilation, MapLibre worker blobs and inline styles required by the map/UI; it forbids arbitrary inline script. Update provider origins deliberately when configuring another map service. Serve HTTPS, correct JavaScript/WASM MIME types, immutable hashed assets and uncached HTML/config. No telemetry, analytics, cookies or remote subscriber database are added.

Map/search/geometry providers receive the browser IP, requested region and search text. Configuration is public: never put private API secrets in `config.json`. Provider-approved browser tokens must be origin-restricted. Default search is disabled. Disabling all provider URLs leaves already-loaded computation and bundled geometry local. There is no service worker or offline tile prefetching.

[OWASP ASVS 5.0](https://owasp.org/www-project-application-security-verification-standard/) is the verification reference for applicable input-validation, browser, configuration, dependency and privacy controls. This is not an ASVS certification or a completed requirement-by-requirement assessment. There has been no independent penetration test, sustained fuzz campaign or externally validated mobile-core interoperability test.

## Dependencies

Both lockfiles are retained. `npm audit`, `cargo audit`, and `node scripts/check-licenses.mjs` are CI gates. At local verification npm reported zero known vulnerabilities; Rust reported no known vulnerabilities and one unmaintained transitive package warning (`paste` 1.0.15, RUSTSEC-2024-0436). Track upstream replacement; this warning is not hidden by an ignore rule. The license inventory covers 417 locked packages and contains only reviewed permitted expressions. No AGPL source was integrated. License expression checks do not replace retaining upstream notices and a legal review for redistribution; see `docs/DEPENDENCIES.json`.

Report suspected issues privately to the repository maintainer before publishing details. No dedicated security contact or formal response SLA has been established for this candidate.


Portable recordings are bounded (4 MB, 4,096 commands, one hour) and deny unknown fields. Replay occurs in a candidate engine and only replaces the live instance after state-digest verification. Failed command mutations roll back by deterministic reconstruction. The digest is an integrity/compatibility check, not an authenticated signature. Checkpoints reconstruct synthetic lab security state without storing keys. Real credentials remain prohibited. Quota failure is now browser-tested by injecting a QuotaExceededError into the run store and verifying export remains usable; this is not a physical disk-exhaustion test.

The standalone free5GC validation executable adds six linked development-only dependencies, with reviewed Apache-2.0/MIT/BSD-3-Clause notices and hashes under `validation/free5gc`. It never ships in browser assets. Original runtime lockfiles and audits remain unchanged.


Validation-only audit exception: [GO-2022-1083](https://pkg.go.dev/vuln/GO-2022-1083) reports malformed-input panics in free5GC APER, with no fixed release listed. The checker consumes only the 30 locally generated bounded fixtures in a separate process, has no network listener, and turns panics into controlled test failures. Do not use this validator to serve or ingest arbitrary uploaded traffic. It is absent from browser/native application dependencies. `python3 scripts/audit-validation.py` pins Go 1.27.2 and govulncheck 1.9.0, retains the JSON audit, reports this exception visibly and fails on any additional reachable advisory. The initial system-Go 1.23.2 scan also found a standard-library advisory; the pinned 1.27.2 scan removed that finding. This validation tool is not represented as vulnerability-free.
