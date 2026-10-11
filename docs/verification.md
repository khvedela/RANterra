# Verification record

Validated locally on 2026-10-11. This record describes this MVP's tests, not standards conformance certification.

- Native Rust unit tests: NAS request round trip and header, known rejection vector, malformed NAS, NGAP APER round trips and UE-ID checking, deterministic scheduling, state transitions, multiple UEs, no coverage, RF attenuation and input/geometry validation.
- Actual WASM runtime: request/reject exchange, NGAP events, deterministic output across step sizes, 1,225 RF samples and invalid-input rejection.
- Production worker: built Vite worker with actual WASM, external fetches forbidden, step, pause/resume, queue completion, reset and deterministic replay, stale-revision isolation and rejected scenario load.
- TypeScript and Vite production compilation, including separate MapLibre and network-engine workers.
- Browser UI: rendered OSM map and building/coverage layers, NAS/NGAP timeline and hex inspector, pause/resume/step/reset, UE placement/movement, radio-power editing with recomputed RSRP, reload persistence, scenario JSON import, rejected invalid-version import, entity deletion, and event-log download. Live Nominatim search selected Lyon and the OSM retrieval path loaded 600 bounded building footprints there.
- Browser exchange ended in `RegistrationRejected`, while inspector explicitly displayed `Not registered`. The exported rejection bytes were `7e 00 44 07`.

The initial Carto tile URL returned API-key images and was replaced with OSM Standard. MapLibre's explicit Vite worker URL was added after a browser worker-load error. The final app uses MapLibre 6 and deck.gl's dedicated MapLibre integration. The npm dependency audit reported zero known vulnerabilities.

Public Overpass endpoints were unreliable during development. The bundled Paris geometry was obtained directly from OSM's map API. Optional live building retrieval can still fail and is reported to the user. Browser tests exercised both the built-in Paris geometry and live Lyon geometry. The app retries an Overpass failure with one bounded OSM map API request; public services are not mocked in application code.

Limits: no full registration, authentication/security, real radio PHY, calibrated RF accuracy, external mobile-core interoperability, or browser compatibility matrix was validated. The worker test is a Node host of the built browser worker, complemented by interactive testing in the desktop app's actual browser. It does not claim an automated cross-browser end-to-end suite.

![Verified desktop application with decoded NGAP packet and unregistered UE](preview.jpg)
