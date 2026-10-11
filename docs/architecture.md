# Architecture

The React/MapLibre/deck.gl application still communicates only with a serialized Web Worker. `BrowserEngine` remains a thin WASM adapter. All protocol, radio, transport and session modules compile natively without browser APIs.

- `model.rs`: bounded version-2 schema with explicit version-1 migration, denied unknown fields, explicit defaults for old files. Each scenario UE provisions a synthetic lab subscriber unless its unknown-subscriber fault is selected. Identity is also checked from the actual SUCI in received NAS.
- `experiment.rs`: application/native mutation boundary, bounded ordered command journal, portable checkpoint digest, transactional replay/rollback. Reconstructs contexts rather than serializing keys.
- `auth.rs`: deterministic synthetic USIM credentials/RAND, MILENAGE network vector and independent USIM verification. OxiRush Security handles 5G KDFs; no AES or HMAC primitive was written here.
- `registration.rs`: separate UE and AMF phases and security contexts. Stage, security header, capabilities, original NAS container, MAC and COUNT checks precede transitions. Contexts never serialize.
- `protocol.rs`, `ngap.rs`: NAS inspection and NGAP APER adaptation. Both NGAP UE identifiers are checked on associated transport. The gNodeB cannot decipher NAS.
- `session.rs`: configurable DNN/S-NSSAI/capacity policies and one IPv4 session per UE with actual 5GSM, N1, N2 resource transfers, GTP-U, IPv4 and ICMP. The SMF/UPF interface is local; PFCP is absent and not claimed.
- `simulation.rs`: deterministic event scheduler, explicit UE/cell/AMF states, session lifecycle, echo routing, metrics and 100 ms mobility ticks. Registration and echo use the virtual scheduler; session control/deregistration are synchronous atomic procedures at the current virtual time. This avoids fabricating measured control-plane latency.
- `radio.rs`: retained legacy model plus a TR 38.901 path-loss subset. The research model constructs a sorted bounding-box index once; movement recomputes UE links, not the static heatmap.
- `research.rs`: `MobilityModel`, `NetworkController`, typed `ControllerAction`, `MetricsCollector`, `ScenarioExecutor`. `HoldPolicy` is an explicit no-op reference baseline, not trained AI. Controllers are advisory; unrestricted controller-driven protocol mutation is not exposed. `RadioModel` and `Transport` remain in their dedicated modules.
- `engine.worker.ts`: WASM initialization, bounded serialized mailbox, revision guard, transactional engine replacement, timers and error responses. UI worker recovery recreates the worker and loads saved configuration; it does not resume secret runtime contexts.
- `storage.ts`: IndexedDB current scenario, named copies, one explicit run checkpoint and bounded optional geographic cache. IndexedDB schema 2 preserves old stores. Failed caches do not discard successful data retrieval.
- `config.ts`, `geography.ts`: bounded geographic responses, timeouts and public deployment configuration. No simulation path needs these services.

Determinism uses ordered maps, integer virtual time, insertion-order ties and seeded scheduling. Floating-point RF results can vary in the final bits across platforms; wire bytes and event order are the primary replay contract for the same engine build.

Bounds: 16 cells, 128 UEs, 600 polygons/15,000 vertices, 128 waypoints per UE, 2 MB scenario JSON, 256 queued worker commands, 128 echo packets in flight, 16,384 retained events. The UI renders only the latest 500 filtered events and announces that reduction. No radio fidelity is automatically reduced. The IP pool intentionally stops after 254 allocations until reset; this prevents stale TEID reuse.

Outstanding architecture work: multi-session/QoS policy expansion, richer controller observations/rewards, cancellation within synchronous RF construction/replay (worker termination remains available), and true N2 handover/reconnection. Version-1 migration, portable command replay and DNN/slice/capacity policies are implemented.
