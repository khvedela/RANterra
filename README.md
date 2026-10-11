# RANterra

An open-source, browser-native 5G network research playground. Place virtual gNodeBs and UEs on a map, explore approximate coverage, and inspect **real binary NAS and NGAP messages processed by Rust/WebAssembly**. No application backend or simulated JavaScript protocol engine.

The implemented vertical slice is a 5G SA **Registration Request → Registration Reject** exchange. The AMF deliberately rejects registration with cause 7 (5GS services not allowed). Authentication, NAS security, and the remaining registration procedures are **not implemented**. A UE is never presented as registered. 6G and AI are architectural extension points, not implemented radio standards or controllers.

## Run locally

Requirements: Node.js 22.12+ (tested with 24), npm, a Rust toolchain ≥1.88 managed by rustup, wasm-pack, and a desktop browser with WebGL2/WebAssembly/Workers/IndexedDB.

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --locked   # if not already installed
npm ci
npm run build:wasm
npm run dev
```

Open **http://127.0.0.1:5173/**. The build helper puts rustup's `~/.cargo/bin` before Homebrew Rust, which may lack the WASM target. The first wasm-pack build can take several minutes while installing wasm-bindgen and wasm-opt. Network access is required to install dependencies.

Production static build and preview:

```sh
npm run build
npm run preview
```

Serve `dist/` over HTTP, with `.wasm` served as `application/wasm`. Vite is a development/static asset server; no runtime application server is needed. Generated WASM bindings are ignored by Git and must be built on a fresh checkout. A failed WASM load is shown as an error and blocks simulation; there is no mock fallback.

## Try the vertical slice

1. The Paris République example loads 467 actual OSM building footprints. Three UEs have coverage; one illustrates the no-coverage state.
2. Click **Run simulation**. Playback advances 10 simulated milliseconds every 100 real milliseconds, making the short exchange inspectable. It stops when the event queue empties.
3. Select **NAS** or **NGAP** in the timeline. Select an event to inspect decoded information and its exact hexadecimal PDU.
4. Select a UE to see its RSRP, selected cell, AMF context, and **Not registered** status.
5. Change a gNodeB's carrier frequency, power, height, or gain. Leave the field or press Enter to apply. Changes recompute the radio model and reset protocol time.
6. Choose **gNodeB** or **UE**, then click the map. Select and drag a marker, edit its coordinates, or use **Reposition on map**. Names are editable; entities can be deleted.
7. Pause/resume, step 10 ms, reset, or change the seed. Run the same scenario and seed to reproduce event timestamps and bytes.
8. Export/import scenario JSON or export the event log. Scenarios autosave to IndexedDB after Rust validation and reload on the next visit. Browser storage is per origin and browser profile; exporting is the portable backup.

Search submits only when you press Enter or the search button; there is no autocomplete traffic. You can also enter `longitude, latitude`, pan/zoom the map, and use the crosshair to move the study-grid center. Changing the center preserves existing entity coordinates. Load OSM buildings explicitly for the new location, then place or move entities there. Edit the scenario title at the top to name the experiment.

Import [`examples/minimal.json`](examples/minimal.json) for a single-cell, unobstructed exchange or [`examples/paris.json`](examples/paris.json) for the complete demo.

## Architecture

```text
React UI / MapLibre GL / deck.gl
           │ typed commands + versioned snapshots
           ▼
Web Worker ── wasm-bindgen ── BrowserEngine (thin JSON adapter)
                                   │
                      browser-independent Rust Engine
                 ┌─────────────────┼────────────────────┐
                 ▼                 ▼                    ▼
           UE/gNodeB/AMF       RadioModel trait     Transport trait
           state machines      LogDistance         VirtualTransport
                 │                                 ordered queue
                 ▼
        OxiRush NAS 0.4.0 + NGAP 0.4.2 codecs
```

- `engine/src/model.rs`: scenario schema, bounded inputs and geometry validation.
- `engine/src/protocol.rs`: typed initial Registration Request with synthetic lab SUCI and Registration Reject, NAS encoding/decoding.
- `engine/src/ngap.rs`: InitialUEMessage and DownlinkNASTransport APER encoding/decoding, including NAS extraction and UE-ID verification.
- `engine/src/transport.rs`: endpoint/frame abstraction and deterministic timestamp/insertion-order queue. A native transport can implement the trait without changing React or adding browser dependencies to the core.
- `engine/src/simulation.rs`: explicit UE/gNodeB/AMF states, seeded scheduling, event log and RF snapshots. Ordered maps and queue keys remove hash iteration nondeterminism. `Controller` is an experimental proposal interface; it does not execute a controller.
- `engine/src/radio.rs`: replaceable `RadioModel` trait and the current coverage model.
- `engine/src/lib.rs`: WASM-only bindings, compiled out for native builds.
- `src/engine.worker.ts`: serialized commands, actual WASM loading, pause/resume/step timer, transactional scenario replacement and revision checks. Stale commands cannot act on a new scenario.
- `src/NetworkMap.tsx`: OSM basemap, deck.gl RF grid, footprints and associations; draggable entity markers. Rendering does not compute protocols or RF.
- `src/storage.ts`, `src/geography.ts`: IndexedDB scenario and asset caching; external geographic asset requests.

Native execution demonstrates independence from the browser:

```sh
cargo run -p ranterra-engine --example replay -- examples/minimal.json
```

### Protocol path

1. UE encodes a plain 5GMM initial Registration Request (EPD `0x7e`, message `0x41`, ngKSI 7, synthetic SUCI using test PLMN 001/01 and null concealment scheme).
2. Virtual access transport delivers NAS bytes to the gNodeB. There is no RRC or radio waveform encoding.
3. gNodeB encodes an NGAP InitialUEMessage containing the NAS PDU, RAN UE ID, synthetic NR CGI/TAI and RRC establishment cause.
4. Virtual N2 transport delivers APER bytes to the AMF. AMF decodes NGAP, extracts and decodes NAS, and applies the explicit lab rejection policy.
5. AMF encodes Registration Reject `7e 00 44 07`, wraps it in NGAP DownlinkNASTransport and sends it back.
6. gNodeB decodes NGAP and forwards NAS; UE decodes the rejection and enters `RegistrationRejected`. There is no `Registered` state.

Transport delays are an integer LCG-seeded 10–30 ms per hop, unrelated to geographic propagation delay. These are illustrative procedure state machines, not a full standards-compliant UE or core implementation. NG setup is assumed; SCTP, RRC, retransmissions, timers mandated by the standards, authentication, integrity/ciphering, Registration Accept/Complete, sessions and the user plane are absent.

## RF model and limitations

This is a documented **illustrative close-in log-distance model**, not a calibrated 3GPP channel model or a planning-grade RSRP prediction:

```text
PL(dB) = 32.4 + 20 log10(f_GHz) + 30 log10(max(d_3D_metres, 1))
RSRP(dBm) ≈ transmit_power_dBm + gain_dBi
            − 10 log10(12 × 106) − PL − obstruction_loss_dB
```

- Local equirectangular horizontal distance at mean latitude; vertical separation uses configured antenna height and fixed UE height 1.5 m. Local urban-scale use only; no dateline/polar or large-region accuracy guarantee.
- Path-loss exponent 3.0. Height affects only 3D distance, not diffraction or a height-dependent propagation coefficient.
- Transmit power is modeled as total carrier power uniformly allocated across **106 resource blocks × 12 subcarriers**. This fixed allocation is used regardless of frequency. RSRP is a per-resource-element proxy, not an SSB/CSI-RS measurement implementation.
- Fixed omnidirectional gain. No sector azimuth, downtilt, beamforming, bandwidth configuration, MIMO, antenna pattern, interference/SINR, loading, fading, noise, terrain, mobility or throughput model.
- Any accepted OSM footprint intersecting the straight ground-projected link adds **12 dB**, capped at **36 dB**. This deliberately coarse 2D proxy ignores building heights/materials, roof clearance and reflections. The endpoint being inside a footprint also counts. Missing geometry adds no loss.
- Only simple closed building ways are ingested; holes, multipolygon relations, incomplete, degenerate or crossing rings are unsupported. Geometry can be incomplete or stale. Hiding the footprints layer does not remove their RF loss; export/edit/import with an empty `buildings` array to compare unobstructed operation.
- Strongest-cell selection requires ≥−120 dBm; ties prefer the lowest cell ID. Association is not successful registration and is not handover. Radio geometry is static during an exchange; edits reset it.
- Heatmap: 35 × 35 samples at 50 m spacing (1.75 km square including sample cells), centered on the study point. It is a spatial raster of estimated maximum RSRP, not a density heatmap. Outside it, no heatmap claim is made; UE measurements still use their own coordinates.
- Inputs allow 100 MHz–100 GHz for experimentation. Above sub-6 GHz this model is an unsupported extrapolation, **not a 6G or mmWave model**.

Limits: 16 gNodeBs, 128 UEs, 600 building rings, 15,000 vertices, 100 vertices per ring, 2 MB scenario input. Inputs are validated in Rust before replacing the running engine. Event logs are bounded by the one-shot exchange (at most eight records per covered UE), and reset on scenario edits.

## Online assets and local operation

After the application/WASM and current map assets load, simulation, configuration, RF calculation, timeline inspection and import/export run on-device. The production-worker test rejects all external network access and runs the full exchange. This is **not** an offline PWA: a fresh reload needs the local static server; exploring uncached map areas, place search, and new building retrieval need internet. Currently visible raster tiles remain in MapLibre memory and normal browser HTTP caching applies. No bulk/offline tile downloader is provided.

- Basemap: [OSM Standard raster tiles](https://operations.osmfoundation.org/policies/tiles/), visible attribution and normal HTTP caching. Intended for light interactive development; configure an appropriate provider for larger deployments in `src/NetworkMap.tsx`.
- Search: [Nominatim](https://operations.osmfoundation.org/policies/nominatim/), explicit submissions, cached results and ≥1.1 seconds between uncached queries. Search text is sent to that public service.
- Buildings: [Overpass API](https://wiki.openstreetmap.org/wiki/Overpass_API), explicitly requested bounding-box queries, cached in IndexedDB. If Overpass fails, one bounded request to the OSM map read API is attempted. Bounds are sent to the service. Unavailable or timed-out queries retain the existing scenario and show an error.
- Fonts, WASM, workers and demo footprints are bundled locally. No analytics, accounts, remote simulation, application backend, or native networking.

## Verification

```sh
npm run build           # Rust → WASM, TypeScript, production Vite build
npm test                # native Rust, actual WASM execution, production worker
cargo fmt --all --check
npm audit
```

Run `npm run build` before `npm test` because worker tests use the actual production output. Individual commands: `npm run test:rust`, `npm run test:wasm`, `npm run test:worker`.

Tests cover NAS wire header and byte-preserving round trip, known rejection vector, malformed NAS, NGAP APER envelopes and UE-ID mismatch, state transitions including concurrent UEs, no-coverage behavior, deterministic timestamps independent of step size, different seeds, timestamp/insertion-order scheduling, distance/building attenuation, malformed scenario/geometry rejection, and the bundled Paris example. Runtime tests instantiate the real `.wasm`. The production worker test uses a minimal Node Web Worker host with local WASM loading to test pause/resume, step, reset, stale revisions and invalid-load rejection; it is not a fake engine.

Browser verification and observed limits are recorded in [`docs/verification.md`](docs/verification.md).

## Implemented versus planned

| Implemented                                            | Planned / unsupported                                                |
| ------------------------------------------------------ | -------------------------------------------------------------------- |
| React/TypeScript desktop UI, MapLibre + deck.gl        | Mobile-first interface, collaborative scenarios                      |
| Place search, map selection, actual OSM footprints     | Offline map packages, full multipolygon geometry                     |
| Add/move/name/configure/delete entities                | UE trajectories and handover                                         |
| Rust coverage and per-UE RSRP, strongest-cell links    | Calibrated 3GPP models, terrain, interference, beamforming           |
| Real OxiRush NAS + NGAP binary request/reject exchange | Authentication, NAS security, complete registration, PDU sessions    |
| Deterministic virtual transport and explicit states    | Native SCTP/RRC/user-plane stacks and external core interoperability |
| WASM Web Worker; pause/resume/reset/step/seed          | Real-time distributed simulation                                     |
| IndexedDB save; JSON scenario and log export           | Scenario library and migration framework beyond version 1            |
| RadioModel, Transport, Controller extension interfaces | Experimental 6G models and AI controllers                            |

## Licensing and references

Application code: [MIT](LICENSE). OSM-derived datasets retain **ODbL 1.0** terms, separate from the code license; see [`docs/data-attribution.md`](docs/data-attribution.md). Dependencies retain their own licenses. OxiRush NAS and NGAP are Apache-2.0; they are dependencies, not copied source.

Primary implementation references: [OxiRush NAS](https://github.com/linouxis9/oxirush-nas) (TS 24.501), [OxiRush NGAP](https://github.com/linouxis9/oxirush-ngap) (TS 38.413), [MapLibre integration](https://maplibre.org/maplibre-gl-js/docs/), and [deck.gl MapLibre integration](https://deck.gl/docs/developer-guide/base-maps/using-with-maplibre).

`package-lock.json` and `Cargo.lock` provide reproducible dependency inputs. The loaders.gl 4.3.4 overrides avoid an unavailable transitive 4.5.4 package version in the registry used during development.
