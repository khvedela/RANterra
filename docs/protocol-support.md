# Protocol support and validation boundary

Target **Release 18 subset**:

| Reference | Exact version | Use |
|---|---|---|
| [TS 24.501](https://www.etsi.org/deliver/etsi_ts/124500_124599/124501/18.07.00_60/ts_124501v180700p.pdf) | 18.7.0 | NAS registration, security mode, deregistration, N1 SM and session messages |
| [TS 33.501](https://www.etsi.org/deliver/etsi_ts/133500_133599/133501/18.05.00_60/ts_133501v180500p.pdf) | 18.5.0 | 5G-AKA and Annex A KDFs; NAS security |
| [TS 23.502](https://www.etsi.org/deliver/etsi_ts/123500_123599/123502/18.05.00_60/ts_123502v180500p.pdf) | 18.5.0 | Registration/session procedural reference |
| [TS 38.413](https://www.etsi.org/deliver/etsi_ts/138400_138499/138413/18.04.00_60/ts_138413v180400p.pdf) | 18.4.0 | Logical N2 NAS transport/resource/context release |
| [TS 33.102](https://www.etsi.org/deliver/etsi_ts/133100_133199/133102/18.00.00_60/ts_133102v180000p.pdf) | 18.0.0 | AUTS construction with f1*/f5* and zero AMF |
| [TS 35.208](https://www.etsi.org/deliver/etsi_ts/135200_135299/135208/18.00.00_60/ts_135208v180000p.pdf) | 18.0.0 | Published MILENAGE values |
| [TS 33.401](https://www.etsi.org/deliver/etsi_ts/133400_133499/133401/18.00.00_60/ts_133401v180000p.pdf) | 18.0.0 | AES-CTR algorithm test vector |

These are implementation references, not a claim that every conditional requirement is satisfied. Codec libraries contain additional messages outside the supported engine profile.

| Procedure | Actual support | Material omissions |
|---|---|---|
| Initial registration | Encoded request, Authentication Request/Response, Security Mode Command/Complete, Registration Accept/Complete; REGISTERED only after AMF verification | NG setup/RRC/AS security assumed outside the virtual access boundary; normal WB-N1 initial-registration timers supported; no emergency/NB-N1 variants, mobility registration update or full RRC connection management |
| Identity/subscriber | PLMN 001/01, decimal synthetic SUPI, null-scheme IMSI SUCI, provisioned per UE | No real keys, concealed SUCI profiles, identity retrieval or external UDM/SBI |
| AKA | MILENAGE, AUTN MAC/separation/SQN verification, RES*/XRES*, HRES*/HXRES*, KAUSF/KSEAF/KAMF and NAS keys | Bounded AUTS resynchronization supported; no IND array/operator SQN window or EAP-AKA'; deterministic RAND is laboratory-only |
| NAS security | NIA2 AES-CMAC, NEA2 AES-CTR, separate UL/DL counters, new-context headers, capability replay and original NAS container checks | No other negotiated suites; keys are not exportable/restorable |
| Negative registration | Illegal UE cause 3; AUTN MAC failure cause 20; Authentication Reject for wrong response; MAC/counter failure never registers; independent contexts | T3560/T3550=6 s, T3510=15 s, T3511=10 s, default T3502=720 s; T3550 exhaustion is shown as unconfirmed, never completed registration |
| Deregistration | Protected UE request/accept, active session release first, encoded UEContextReleaseCommand/Complete, key/context cleanup | Atomic at current virtual time; no network-initiated deregistration or automatic re-registration |
| PDU session | Protected UL/DL N1, IPv4 request/accept, NGAP setup transfer/request/response, IP/TEIDs, release request/command/complete and N2 release | One session ID 1, UE PTI 1 / network-initiated release PTI 0, QFI 1/5QI9; configurable DNN/SST/SD and capacity; no multiple simultaneous sessions per UE, external SMF/UPF or PFCP; failed N2 cause 22 and T3592 are supported, not all N2 causes or SM procedures |
| User plane | Bounded GTP-U G-PDU profile with checked length/TEID, IPv4 ICMP echo, checksum/routing validation, delivery/loss/virtual RTT | No GTP-U extensions/QFI container, real UDP/internet, fragmentation, PHY scheduling or actual throughput/AMBR enforcement |
| Mobility | Waypoint measurements, A3-style reassociation/coverage loss and local context invalidation | No NGAP handover, RRC reconfiguration or automatic reconnection; not labeled successful handover |

Security header types used: 3 for the Security Mode Command, 4 for Complete, 2 for subsequent protected NAS. Plaintext or unexpected protected messages cannot skip the state machine. The uplink replayed initial message must match the AMF's original request. Initial Registration Request and reject vectors retain the original tests.

Validation includes published MILENAGE outputs, a 253-bit NEA2 vector, a byte-aligned [free5GC NIA2 test](https://github.com/free5gc/nas/blob/main/security/security_test.go), and independent Python HMAC-derived 5G key fixtures. The Python fixture is **independently computed**, not falsely described as a published complete 5G-AKA vector. NAS/NGAP round trips and malformed corpora supplement these vectors. A standalone pinned free5GC NAS/NGAP validator now independently decodes/re-encodes 30 frames, including the inner N2 setup transfer and required non-GBR AMBR IE. It detected and led to correction of an OxiRush RRCEstablishmentCause IE-ID defect. See [validation scope](../validation/free5gc/README.md). No live UERANSIM/Open5GS differential state-machine session or external interoperability certification has run.


## Configurable service selection

Version-2 scenarios carry up to eight unique DNN/S-NSSAI policies and per-UE requests. DNNs use validated lower-case DNS labels; SD is optional three-octet data (ffffff is rejected). Registration Accept advertises the configured slices. Initial UL N1 carries the actual requested DNN/S-NSSAI; SM accept and N2 setup carry the selected values, checked at receiving boundaries.

When no service policy matches, the AMF returns the unforwarded SM request in protected DL NAS Transport with 5GMM cause 91 (TS 24.501 §5.4.5.3). No N2 resources, IP or TEID are allocated. The laboratory UE requires reset/configuration change before retrying that selection. Exhausted policy capacity/pool produces an encoded 5GSM PDU Session Establishment Reject, cause 26, T3396=10 seconds. The UE cannot retry before the virtual timer expires; rejected commands consume neither IP nor TEID. Resource availability is checked again at retry. The virtual scheduler remains runnable during backoff.

Capacity is a local admission policy, not throughput enforcement. NAS and N2 signal a fixed 1 Mbit/s AMBR, QFI 1/5QI9 and IPv4/SSC1; no packet-rate QoS scheduler is claimed. Session control uses configurable 1–1000 ms virtual delivery stages in the browser; registration transport retains seeded 10–30 ms per-hop delays. Shared `smf::Smf` separates pending setup, active forwarding and release acknowledgement states; pending resources reserve capacity. A valid N2 response supplies the gNodeB-allocated DL TEID. Both browser and direct callers drive T3592 (16 s, four retries) and laboratory N2 timeouts. A failed resource setup response carries the failed-session list and cause `radio_resources_not_available` (22); protected network-initiated NAS release uses PTI 0 and cause 26. No tunnel is activated for this failed setup.

## Independent endpoint follow-up

Thirteen Go/free5GC peer tests validate both Rust registration roles, independent AUTS in both directions, NAS counters and the shared SMF allocation/policy/traffic/release lifecycle. Published f1*/f5* and independent AUTS fixtures supplement these tests. `SqnOutOfSync` is a reproducible browser experiment: cause 21/AUTS, fresh network challenge, secured registration and usable PDU session. A repeated outstanding valid Authentication Request reuses RES* without resetting the USIM or NAS context; this cache is cleared on Security Mode success. It does not bypass protection or admit challenge replay after security establishment.

The SMF T3592 boundary is independently tested for retransmission, retry counters, abort and cleanup; native tests compare large and small time advances. The independent peer also checks failed N2 cleanup, AMF T3560/T3550 and fresh protected Security Mode/Accept retransmissions. Native and browser tests cover timed loss recovery and T3510/T3511/T3502. This normal initial-registration subset does not implement every conditional timer variant. Echo transport retains its 2,000 ms laboratory deadline. See [precise validation scope and supplementary TS 33.102 reference](interop-validation.md).
