# Radio model and validation

The selectable research models implement the path-loss equations in Table 7.4.1-1 of [3GPP TR 38.901 v18.0.0](https://www.etsi.org/deliver/etsi_tr/138900_138999/138901/18.00.00_60/tr_138901v180000p.pdf). They are a bounded subset, not a full channel simulator or calibrated coverage predictor.

- UMi street canyon: base station height 10 m. UMa: 25 m. Both fix terminal height to 1.5 m and effective environment height to 1 m.
- Frequency: 0.5–100 GHz; horizontal distance: 10–5000 m. Inputs outside the supported height/frequency domain are rejected. Links outside the distance domain return an unavailable signal sentinel (-300 dBm), not extrapolated coverage. In particular, points within 10 m are unavailable in this profile.
- LOS follows the two-slope breakpoint equation; NLOS takes the maximum of LOS and the NLOS equation. Footprint intersections classify NLOS. Missing footprints mean assumed LOS. No probabilistic LOS model, terrain, diffraction, building heights, materials, indoor penetration or atmospheric attenuation is claimed.
- Coordinates use a local equirectangular distance approximation. This is suitable for small local experiments, not global/antimeridian trajectories.
- RSRP approximates equal power per subcarrier: total configured transmit power + gain - antenna loss - path loss - shadowing - 10 log10(12 × resource blocks). This is not a complete SSB/resource-element allocation model. Occupied bandwidth is 12 × resource blocks × 15 kHz × 2^numerology; guard bands are excluded.
- Optional sector attenuation uses 65-degree horizontal/vertical widths, quadratic roll-off and a combined 30 dB cap. Azimuth and downtilt are configurable; it is not an antenna-array/beamforming implementation.
- Optional seeded Gaussian shadow values use 0.00025-degree longitude/latitude bins and per-cell seeds. Standard deviations are 4 dB LOS, 7.82 dB UMi NLOS and 6 dB UMa NLOS. Bin dimensions vary with latitude; values are not the spatially correlated TR 38.901 process. Shadowing is disabled by default.
- Thermal noise is -174 dBm/Hz with a fixed 7 dB receiver noise figure. SINR sums full-load powers from cells at the same carrier frequency (within 0.001 MHz). Partial spectral overlap, traffic-dependent load and inter-numerology effects are omitted. RSRQ is not reported.
- Coverage uses configurable RSRP threshold (default -120 dBm); no SINR admission threshold or BLER mapping is applied. Loss injection in echo traffic is explicit, not an RF-derived packet-error model.

Old scenario files select the labeled legacy illustrative log-distance model. Its fixed attenuation assumptions are retained for backward compatibility, not upgraded silently. Choose UMi or UMa explicitly for the research subset.

## Numerical evidence

Unit fixtures independently evaluate the equations at horizontal distance 100 m, frequency 3.5 GHz and terminal height 1.5 m: UMi LOS **85.314188 dB**, UMa LOS **83.1381567 dB**, tolerance 0.00001 dB. Tests also check NLOS >= LOS, domain edges, footprint attenuation, deterministic shadow values, sector orientation and co-channel interference. Mobility tests check identical snapshots across time-step sizes, coverage loss and the A3 timer.

These are equation and behavioral checks. No measured drive-test calibration or live independent RF simulator execution has run. These remain validation gaps for stronger scientific claims.

## Mobility and computational scope

Waypoints interpolate by distance at configured speed in m/s, then remain at the endpoint. Static means no waypoints or zero speed. Pedestrian and vehicle experiments use different speeds; there is no road-routing model. Measurements update every 100 ms of virtual time. A3-style reassociation requires a candidate to exceed the current cell by the hysteresis margin for the configured time-to-trigger, quantized to those ticks. Reassociation invalidates the old session/registration; it is explicitly not an NGAP handover. Reset is required to initiate fresh registration.

A sorted footprint bounding-box index avoids checking every polygon edge per sample. The 35×35 coverage grid is computed when loading a scenario; motion updates UE measurements without recomputing the static coverage grid. Edits currently rebuild the engine/grid. RF runs in the worker; no hidden fidelity reduction occurs under load. See the release report for measured bounded-workload results.


## Independent numerical reference comparison

`engine/tests/radio_references.rs` now checks 16 numerical points published in [ns-3.47 propagation tests](https://raw.githubusercontent.com/nsnam/ns-3-dev-git/ns-3.47/src/propagation/test/three-gpp-propagation-loss-model-test-suite.cc): both UMi/UMa, LOS/NLOS, 10/100/1000/5000 m, 5 GHz, terminal height 1.5 m, base heights 10/25 m. Every point agrees within 0.0001 dB. Separate checks cover breakpoint continuity and frequency bounds 0.5/100 GHz. This uses published numerical reference facts from SIGNET Lab / University of Padova, not copied ns-3 GPL implementation code; ns-3 is neither linked nor distributed. Some UMi reference values are assigned but not added to the upstream test vector; RANterra checks all 16 listed values. This is a numerical reference comparison, not a claim that ns-3 itself was executed here or that either model predicts measured field coverage.
