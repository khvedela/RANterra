use crate::research::MobilityModel;
use crate::{
    model::*,
    protocol,
    radio::{self, LogDistance, RadioModel, Sample},
    transport::*,
};
use serde::Serialize;
use std::collections::BTreeMap;
#[derive(Clone, Debug, Serialize, PartialEq)]
pub enum UeState {
    Idle,
    NoCoverage,
    RegistrationInitiated,
    Authenticating,
    SecurityMode,
    RegistrationAccepted,
    Registered,
    RegistrationRejected,
    ReassociationRequired,
    Deregistered,
}
#[derive(Clone, Debug, Serialize)]
pub enum GnodebState {
    Ready,
    Relaying,
    Idle,
}
#[derive(Clone, Debug, Serialize)]
pub enum AmfState {
    Listening,
    Evaluating,
    Rejected,
    Registered,
}
#[derive(Clone, Debug, Serialize)]
pub struct UeStatus {
    pub id: u32,
    pub state: UeState,
    pub serving_cell: Option<u32>,
    pub rsrp: Option<f64>,
    pub failure: Option<String>,
    pub registration_latency_ms: Option<u64>,
    pub position: [f64; 2],
    pub sinr_db: Option<f64>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Event {
    pub id: usize,
    pub time_ms: u64,
    pub ue_id: u32,
    pub source: String,
    pub destination: String,
    pub kind: String,
    pub detail: String,
    pub hex: String,
    pub layer: String,
}
#[derive(Serialize)]
pub struct Snapshot {
    pub time_ms: u64,
    pub started: bool,
    pub pending: usize,
    pub ues: Vec<UeStatus>,
    pub gnodebs: BTreeMap<u32, GnodebState>,
    pub amf: BTreeMap<u32, AmfState>,
    pub events: Vec<Event>,
    pub coverage: Vec<Sample>,
    pub sessions: Vec<crate::session::Session>,
    pub event_limit_reached: bool,
}
/// Experimental AI controllers can propose actions against snapshots. The scheduler owns time.
pub trait Controller {
    fn propose(&mut self, snapshot: &Snapshot) -> Vec<String>;
}
pub struct Engine<T: Transport = VirtualTransport> {
    pub scenario: Scenario,
    pub time_ms: u64,
    started: bool,
    rng: u32,
    transport: T,
    pub ues: BTreeMap<u32, UeStatus>,
    gnodebs: BTreeMap<u32, GnodebState>,
    amf: BTreeMap<u32, AmfState>,
    events: Vec<Event>,
    registrations: BTreeMap<u32, crate::registration::Registration>,
    model: Box<dyn RadioModel>,
    sessions: BTreeMap<u32, crate::session::Session>,
    inflight: BTreeMap<(u32, u16), u64>,
    next_session_slot: u32,
    next_mobility_tick: u64,
    cell_candidates: BTreeMap<u32, (u32, u64)>,
    coverage: Vec<Sample>,
}
impl Engine<VirtualTransport> {
    pub fn new(scenario: Scenario) -> Result<Self, String> {
        let model: Box<dyn RadioModel> = if scenario.radio.model == RadioKind::Legacy {
            Box::new(LogDistance)
        } else {
            Box::new(radio::Tr38901::new(&scenario))
        };
        Self::with_adapters(scenario, VirtualTransport::default(), model)
    }
}
impl<T: Transport> Engine<T> {
    pub fn with_adapters(
        scenario: Scenario,
        transport: T,
        model: Box<dyn RadioModel>,
    ) -> Result<Self, String> {
        scenario.validate()?;
        let mut e = Self {
            rng: scenario.seed,
            scenario,
            time_ms: 0,
            started: false,
            transport,
            ues: BTreeMap::new(),
            gnodebs: BTreeMap::new(),
            amf: BTreeMap::new(),
            events: vec![],
            registrations: BTreeMap::new(),
            model,
            coverage: vec![],
            sessions: BTreeMap::new(),
            inflight: BTreeMap::new(),
            next_session_slot: 1,
            next_mobility_tick: 100,
            cell_candidates: BTreeMap::new(),
        };
        for g in &e.scenario.gnodebs {
            e.gnodebs.insert(g.id, GnodebState::Ready);
        }
        for u in &e.scenario.ues {
            e.registrations.insert(
                u.id,
                crate::registration::Registration::new(u.id, e.scenario.seed, u.fault)?,
            );
            let signal = radio::strongest(
                e.model.as_ref(),
                &e.scenario.gnodebs,
                [u.lon, u.lat],
                &e.scenario.buildings,
            );
            e.ues.insert(
                u.id,
                UeStatus {
                    id: u.id,
                    state: UeState::Idle,
                    serving_cell: signal
                        .filter(|s| s.1 >= e.scenario.radio.coverage_dbm)
                        .map(|s| s.0),
                    rsrp: signal.map(|s| s.1),
                    failure: None,
                    registration_latency_ms: None,
                    position: [u.lon, u.lat],
                    sinr_db: signal.and_then(|s| {
                        radio::sinr(
                            e.model.as_ref(),
                            &e.scenario.gnodebs,
                            [u.lon, u.lat],
                            &e.scenario.buildings,
                            s.0,
                        )
                    }),
                },
            );
        }
        for x in 0..35 {
            for y in 0..35 {
                let position = [
                    e.scenario.center[0]
                        + (x as f64 - 17.0) * 50.0
                            / (111320.0 * e.scenario.center[1].to_radians().cos()),
                    e.scenario.center[1] + (y as f64 - 17.0) * 50.0 / 111320.0,
                ];
                if let Some((cell_id, rsrp)) = radio::strongest(
                    e.model.as_ref(),
                    &e.scenario.gnodebs,
                    position,
                    &e.scenario.buildings,
                ) {
                    e.coverage.push(Sample {
                        position,
                        rsrp,
                        cell_id,
                    });
                }
            }
        }
        Ok(e)
    }
    fn delay(&mut self) -> u64 {
        self.rng = self.rng.wrapping_mul(1664525).wrapping_add(1013904223);
        10 + (self.rng % 21) as u64
    }
    fn send(&mut self, frame: Frame) {
        let at = self.time_ms + self.delay();
        self.transport.send(at, frame);
    }
    #[allow(clippy::too_many_arguments)] // Preserved event boundary: explicit endpoints and exact wire bytes.
    fn log(
        &mut self,
        ue_id: u32,
        source: String,
        destination: String,
        kind: String,
        detail: String,
        bytes: &[u8],
        layer: &str,
    ) {
        if self.events.len() >= 16384 {
            return;
        }
        self.events.push(Event {
            id: self.events.len(),
            time_ms: self.time_ms,
            ue_id,
            source,
            destination,
            kind,
            detail,
            hex: bytes
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(" "),
            layer: layer.into(),
        });
    }
    pub fn start(&mut self) -> Result<(), String> {
        if self.started {
            return Ok(());
        }
        self.started = true;
        let ids: Vec<_> = self.ues.keys().copied().collect();
        for id in ids {
            if let Some(cell_id) = self.ues[&id].serving_cell {
                let bytes = protocol::registration_request(id)?;
                let (kind, detail) = protocol::inspect(&bytes)?;
                self.ues.get_mut(&id).unwrap().state = UeState::RegistrationInitiated;
                self.log(
                    id,
                    format!("UE-{id}"),
                    format!("gNB-{cell_id}"),
                    kind,
                    detail,
                    &bytes,
                    "NAS",
                );
                self.send(Frame {
                    from: Endpoint::Ue(id),
                    to: Endpoint::Gnodeb(cell_id),
                    ue_id: id,
                    cell_id,
                    bytes,
                    ngap: false,
                });
            } else {
                self.ues.get_mut(&id).unwrap().state = UeState::NoCoverage;
                self.log(
                    id,
                    format!("UE-{id}"),
                    "—".into(),
                    "No coverage".into(),
                    "Strongest RSRP is below −120 dBm, or no cell exists. No NAS request sent."
                        .into(),
                    &[],
                    "RADIO",
                );
            }
        }
        Ok(())
    }
    pub fn advance(&mut self, ms: u64) -> Result<(), String> {
        if ms > 60_000 {
            return Err("Step must be ≤60000 ms".into());
        }
        self.start()?;
        let until = self
            .time_ms
            .checked_add(ms)
            .ok_or("Simulation clock overflow")?;
        loop {
            let tick = if self.has_mobility() {
                self.next_mobility_tick.min(until)
            } else {
                until
            };
            while let Some((time, frame)) = self.transport.receive(tick) {
                self.time_ms = time;
                self.deliver(frame)?;
            }
            if self.has_mobility() && self.next_mobility_tick <= until {
                self.time_ms = self.next_mobility_tick;
                self.update_mobility();
                self.next_mobility_tick += 100;
            } else {
                break;
            }
        }
        // A single bounded procedure deadline; retransmission timers are not claimed.
        if self.time_ms < 6000 && until >= 6000 {
            self.time_ms = 6000;
            let ids: Vec<_> = self
                .ues
                .values()
                .filter(|u| {
                    !matches!(
                        u.state,
                        UeState::Registered
                            | UeState::RegistrationRejected
                            | UeState::NoCoverage
                            | UeState::ReassociationRequired
                            | UeState::Deregistered
                    )
                })
                .map(|u| u.id)
                .collect();
            for id in ids {
                self.fail_ue(
                    id,
                    "Registration timeout (6000 ms laboratory deadline)".into(),
                );
            }
        }
        self.time_ms = until;
        Ok(())
    }
    fn deliver(&mut self, frame: Frame) -> Result<(), String> {
        let Frame {
            from,
            to,
            ue_id,
            cell_id,
            bytes,
            ngap,
        } = frame;
        if matches!(to, Endpoint::Upf | Endpoint::DataGnodeb(_)) {
            return self.deliver_data(from, to, ue_id, cell_id, bytes);
        }
        if matches!(
            self.ues[&ue_id].state,
            UeState::NoCoverage | UeState::ReassociationRequired | UeState::Deregistered
        ) {
            return Ok(());
        }
        let bytes = if ngap {
            let (name, nas) = crate::ngap::unwrap(&bytes, ue_id)?;
            self.log(
                ue_id,
                format!("{from:?}"),
                format!("{to:?}"),
                name,
                format!(
                    "APER decoded; RAN UE ID {ue_id}; NAS payload {} bytes",
                    nas.len()
                ),
                &bytes,
                "NGAP",
            );
            nas
        } else {
            bytes
        };
        match to {
            Endpoint::Gnodeb(_) => {
                self.gnodebs.insert(cell_id, GnodebState::Relaying);
                let uplink = matches!(from, Endpoint::Ue(_));
                let (kind, detail) = if bytes.get(1) == Some(&0) {
                    protocol::inspect(&bytes)?
                } else {
                    (
                        "Protected NAS relay".into(),
                        "Opaque NAS security envelope; gNodeB has no NAS keys".into(),
                    )
                };
                self.log(
                    ue_id,
                    format!("{from:?}"),
                    format!("{to:?}"),
                    kind,
                    detail,
                    &bytes,
                    "NAS",
                );
                let bytes = if uplink {
                    if bytes.get(2) == Some(&0x41) && bytes.get(1) == Some(&0) {
                        crate::ngap::initial(bytes, ue_id, cell_id)?
                    } else {
                        crate::ngap::uplink(bytes, ue_id, cell_id)?
                    }
                } else {
                    bytes
                };
                self.send(Frame {
                    from: to,
                    to: if uplink {
                        Endpoint::Amf
                    } else {
                        Endpoint::Ue(ue_id)
                    },
                    ue_id,
                    cell_id,
                    bytes,
                    ngap: uplink,
                });
            }
            Endpoint::Upf | Endpoint::DataGnodeb(_) => unreachable!(),
            Endpoint::Amf | Endpoint::Ue(_) => {
                let amf = to == Endpoint::Amf;
                let registration = self
                    .registrations
                    .get_mut(&ue_id)
                    .ok_or("Unknown UE context")?;
                let result = if amf {
                    registration.at_amf(&bytes)
                } else {
                    registration.at_ue(&bytes)
                };
                match result {
                    Err(reason) => self.fail_ue(ue_id, reason),
                    Ok(result) => {
                        let (kind, detail) = protocol::inspect(&result.plain)?;
                        self.log(
                            ue_id,
                            format!("{from:?}"),
                            format!("{to:?}"),
                            kind,
                            detail,
                            &bytes,
                            "NAS",
                        );
                        if let Some(failure) = self.registrations[&ue_id].failure.clone() {
                            self.ues.get_mut(&ue_id).unwrap().failure = Some(failure);
                            self.ues.get_mut(&ue_id).unwrap().state = UeState::RegistrationRejected;
                            self.amf.insert(ue_id, AmfState::Rejected);
                        } else {
                            let state = if result.registered {
                                UeState::Registered
                            } else {
                                match result.plain[2] {
                                    0x41 | 0x56 | 0x57 => UeState::Authenticating,
                                    0x5d | 0x5e => UeState::SecurityMode,
                                    0x42 => UeState::RegistrationAccepted,
                                    _ => self.ues[&ue_id].state.clone(),
                                }
                            };
                            self.ues.get_mut(&ue_id).unwrap().state = state;
                            self.amf.insert(
                                ue_id,
                                if result.registered {
                                    AmfState::Registered
                                } else {
                                    AmfState::Evaluating
                                },
                            );
                            if result.registered {
                                self.ues.get_mut(&ue_id).unwrap().registration_latency_ms =
                                    Some(self.time_ms);
                            }
                        }
                        if let Some(reply) = result.reply {
                            self.send(Frame {
                                from: to,
                                to: Endpoint::Gnodeb(cell_id),
                                ue_id,
                                cell_id,
                                bytes: if amf {
                                    crate::ngap::downlink(reply, ue_id)?
                                } else {
                                    reply
                                },
                                ngap: amf,
                            });
                        }
                    }
                }
                let active = self.ues.values().any(|u| {
                    u.serving_cell == Some(cell_id)
                        && !matches!(
                            u.state,
                            UeState::Registered
                                | UeState::RegistrationRejected
                                | UeState::NoCoverage
                                | UeState::ReassociationRequired
                                | UeState::Deregistered
                        )
                });
                self.gnodebs.insert(
                    cell_id,
                    if active {
                        GnodebState::Relaying
                    } else {
                        GnodebState::Idle
                    },
                );
            }
        }
        Ok(())
    }
    fn has_mobility(&self) -> bool {
        self.next_mobility_tick <= self.scenario.radio.duration_ms
            && self
                .scenario
                .ues
                .iter()
                .any(|u| !u.mobility.waypoints.is_empty() && u.mobility.speed_mps > 0.0)
    }
    fn update_mobility(&mut self) {
        for ue in self.scenario.ues.clone() {
            if ue.mobility.waypoints.is_empty() || ue.mobility.speed_mps == 0.0 {
                continue;
            }
            let position = crate::research::Waypoints.position(&ue, self.time_ms);
            let best = radio::strongest(
                self.model.as_ref(),
                &self.scenario.gnodebs,
                position,
                &self.scenario.buildings,
            );
            let candidate = best
                .filter(|s| s.1 >= self.scenario.radio.coverage_dbm)
                .map(|s| s.0);
            let old = self.ues[&ue.id].serving_cell;
            let current = old
                .and_then(|id| self.scenario.gnodebs.iter().find(|g| g.id == id))
                .map(|g| self.model.rsrp(g, position, &self.scenario.buildings));
            let mut switch = false;
            if candidate != old {
                if let (Some(candidate_id), Some(_)) = (candidate, old) {
                    if best.unwrap().1 - current.unwrap_or(-300.0)
                        >= self.scenario.radio.hysteresis_db
                    {
                        let entry = self
                            .cell_candidates
                            .entry(ue.id)
                            .or_insert((candidate_id, self.time_ms));
                        if entry.0 != candidate_id {
                            *entry = (candidate_id, self.time_ms);
                        }
                        switch = self.time_ms - entry.1 >= self.scenario.radio.time_to_trigger_ms;
                    } else {
                        self.cell_candidates.remove(&ue.id);
                    }
                } else {
                    switch = true;
                }
            } else {
                self.cell_candidates.remove(&ue.id);
            }
            if switch {
                self.cell_candidates.remove(&ue.id);
                let reason = if candidate.is_none() {
                    "Radio link lost; local context cleanup"
                } else {
                    "A3-style serving-cell reassociation; protocol handover unsupported; reset to register"
                };
                self.registrations
                    .get_mut(&ue.id)
                    .unwrap()
                    .fail(reason.into());
                if let Some(s) = self.sessions.get_mut(&ue.id) {
                    s.active = false;
                    s.dropped += self.inflight.keys().filter(|(id, _)| *id == ue.id).count() as u64;
                }
                self.inflight.retain(|(id, _), _| *id != ue.id);
                let u = self.ues.get_mut(&ue.id).unwrap();
                u.serving_cell = candidate;
                u.state = if candidate.is_none() {
                    UeState::NoCoverage
                } else {
                    UeState::ReassociationRequired
                };
                u.failure = Some(reason.into());
                self.amf.remove(&ue.id);
                self.log(
                    ue.id,
                    format!("Cell {old:?}"),
                    format!("Cell {candidate:?}"),
                    "Serving cell changed".into(),
                    reason.into(),
                    &[],
                    "RADIO",
                );
            }
            let cell = self.ues[&ue.id].serving_cell;
            let sinr = cell.and_then(|id| {
                radio::sinr(
                    self.model.as_ref(),
                    &self.scenario.gnodebs,
                    position,
                    &self.scenario.buildings,
                    id,
                )
            });
            let u = self.ues.get_mut(&ue.id).unwrap();
            u.position = position;
            u.rsrp = if switch {
                best.map(|s| s.1)
            } else {
                current.or(best.map(|s| s.1))
            };
            u.sinr_db = sinr;
        }
    }
    fn session_nas(
        &mut self,
        id: u32,
        sm: Vec<u8>,
        uplink: bool,
        initial: bool,
    ) -> Result<Vec<u8>, String> {
        let plain = crate::session::n1(sm, uplink, initial)?;
        let wire = self
            .registrations
            .get_mut(&id)
            .ok_or("Unknown UE")?
            .protect_session(plain, uplink)?;
        let cell = self.ues[&id].serving_cell.ok_or("No serving cell")?;
        let n2 = if uplink {
            crate::ngap::uplink(wire.clone(), id, cell)?
        } else {
            crate::ngap::downlink(wire.clone(), id)?
        };
        let (name, received) = crate::ngap::unwrap(&n2, id)?;
        self.log(
            id,
            if uplink {
                format!("gNB-{cell}")
            } else {
                "AMF".into()
            },
            if uplink {
                "AMF".into()
            } else {
                format!("gNB-{cell}")
            },
            name,
            "N1 SM transport".into(),
            &n2,
            "NGAP",
        );
        let decoded = self
            .registrations
            .get_mut(&id)
            .unwrap()
            .receive_session(&received, uplink)?;
        self.log(
            id,
            if uplink {
                format!("UE-{id}")
            } else {
                "SMF via AMF".into()
            },
            if uplink {
                "SMF via AMF".into()
            } else {
                format!("UE-{id}")
            },
            if uplink {
                "UL NAS Transport".into()
            } else {
                "DL NAS Transport".into()
            },
            oxirush_nas::nas_5gs::decode_nas_5gs_message(&decoded)
                .map_err(|e| e.to_string())?
                .to_string(),
            &wire,
            "NAS",
        );
        let sm = crate::session::unwrap_n1(&decoded, uplink)?;
        self.log(
            id,
            if uplink {
                format!("UE-{id}")
            } else {
                "SMF".into()
            },
            if uplink {
                "SMF".into()
            } else {
                format!("UE-{id}")
            },
            format!("5GSM 0x{:02x}", sm[3]),
            oxirush_nas::nas_5gs::decode_nas_5gs_message(&sm)
                .map_err(|e| e.to_string())?
                .to_string(),
            &sm,
            "5GSM",
        );
        Ok(sm)
    }
    /// One IPv4/SST1/DNN lab session per UE, atomically run at current virtual time.
    pub fn establish_session(&mut self, id: u32) -> Result<(), String> {
        if self.ues.get(&id).map(|u| &u.state) != Some(&UeState::Registered) {
            return Err("Register the UE first".into());
        }
        if self.sessions.get(&id).is_some_and(|s| s.active) {
            return Err("Session already active".into());
        }
        if self.next_session_slot > 254 {
            return Err("Laboratory IP pool exhausted; reset experiment".into());
        }
        let mut s = crate::session::Session::new(id, self.next_session_slot);
        let sm = self.session_nas(id, crate::session::request()?, true, true)?;
        if sm != crate::session::request()? {
            return Err("Unsupported SMF request".into());
        }
        let plain = crate::session::n1(crate::session::accept(s.ip)?, false, false)?;
        let protected = self
            .registrations
            .get_mut(&id)
            .unwrap()
            .protect_session(plain, false)?;
        let n2 = crate::session::setup(id, s.uplink_teid, protected)?;
        let wire = crate::session::decode_setup(&n2, id, s.uplink_teid)?;
        self.log(
            id,
            "AMF / SMF".into(),
            "gNodeB".into(),
            "PDUSessionResourceSetupRequest".into(),
            format!("Session 1; SST 1; QFI 1; 5QI 9; UL TEID {}", s.uplink_teid),
            &n2,
            "NGAP",
        );
        let response = crate::session::setup_response(id, s.downlink_teid)?;
        crate::session::validate_response(&response, id, s.downlink_teid)?;
        self.log(
            id,
            "gNodeB".into(),
            "AMF / SMF".into(),
            "PDUSessionResourceSetupResponse".into(),
            format!("Session 1; DL TEID {}", s.downlink_teid),
            &response,
            "NGAP",
        );
        let plain = self
            .registrations
            .get_mut(&id)
            .unwrap()
            .receive_session(&wire, false)?;
        let sm = crate::session::unwrap_n1(&plain, false)?;
        if sm != crate::session::accept(s.ip)? {
            return Err("UE session accept mismatch".into());
        }
        let detail = oxirush_nas::nas_5gs::decode_nas_5gs_message(&sm)
            .map_err(|e| e.to_string())?
            .to_string();
        self.log(
            id,
            "SMF via AMF".into(),
            format!("UE-{id}"),
            "PDU Session Establishment Accept".into(),
            detail.clone(),
            &wire,
            "NAS",
        );
        self.log(
            id,
            "SMF".into(),
            format!("UE-{id}"),
            "PDU Session Establishment Accept".into(),
            detail,
            &sm,
            "5GSM",
        );
        s.active = true;
        self.sessions.insert(id, s);
        self.next_session_slot += 1;
        Ok(())
    }
    pub fn deregister(&mut self, id: u32) -> Result<(), String> {
        if self.ues.get(&id).map(|u| &u.state) != Some(&UeState::Registered) {
            return Err("UE is not registered".into());
        }
        if self.sessions.get(&id).is_some_and(|s| s.active) {
            self.release_session(id)?;
        }
        let guti = oxirush_security::nas_5gs::build_guti_bytes(&[0, 0xf1, 0x10], 1, 1, 0, id);
        let mut request = vec![0x7e, 0, 0x45, 1, 0, 11];
        request.extend(guti);
        let cell = self.ues[&id].serving_cell.ok_or("No serving cell")?;
        for (uplink, plain, kind) in [
            (true, request, "Deregistration Request"),
            (false, vec![0x7e, 0, 0x46], "Deregistration Accept"),
        ] {
            let wire = self
                .registrations
                .get_mut(&id)
                .unwrap()
                .protect_session(plain.clone(), uplink)?;
            let n2 = if uplink {
                crate::ngap::uplink(wire.clone(), id, cell)?
            } else {
                crate::ngap::downlink(wire.clone(), id)?
            };
            let (name, received) = crate::ngap::unwrap(&n2, id)?;
            let decoded = self
                .registrations
                .get_mut(&id)
                .unwrap()
                .receive_session(&received, uplink)?;
            if decoded != plain {
                return Err("Deregistration context mismatch".into());
            }
            self.log(
                id,
                if uplink {
                    format!("UE-{id}")
                } else {
                    "AMF".into()
                },
                if uplink {
                    "AMF".into()
                } else {
                    format!("UE-{id}")
                },
                kind.into(),
                oxirush_nas::nas_5gs::decode_nas_5gs_message(&decoded)
                    .map_err(|e| e.to_string())?
                    .to_string(),
                &wire,
                "NAS",
            );
            self.log(
                id,
                "gNodeB / AMF".into(),
                "AMF / gNodeB".into(),
                name,
                kind.into(),
                &n2,
                "NGAP",
            );
        }
        for (i, wire) in crate::ngap::release_context(id)?.into_iter().enumerate() {
            self.log(
                id,
                if i == 0 {
                    "AMF".into()
                } else {
                    "gNodeB".into()
                },
                if i == 0 {
                    "gNodeB".into()
                } else {
                    "AMF".into()
                },
                if i == 0 {
                    "UEContextReleaseCommand".into()
                } else {
                    "UEContextReleaseComplete".into()
                },
                "UE-associated logical N2 context released".into(),
                &wire,
                "NGAP",
            );
        }
        self.registrations.get_mut(&id).unwrap().clear();
        self.amf.remove(&id);
        self.ues.get_mut(&id).unwrap().state = UeState::Deregistered;
        Ok(())
    }
    pub fn release_session(&mut self, id: u32) -> Result<(), String> {
        if !self.sessions.get(&id).is_some_and(|s| s.active) {
            return Err("No active PDU session".into());
        }
        self.session_nas(
            id,
            crate::session::nas(vec![0x2e, 1, 1, 0xd1])?,
            true,
            false,
        )?;
        self.session_nas(
            id,
            crate::session::nas(vec![0x2e, 1, 1, 0xd3, 36])?,
            false,
            false,
        )?;
        for (i, b) in crate::session::release_n2(id)?.into_iter().enumerate() {
            self.log(
                id,
                if i == 0 {
                    "AMF".into()
                } else {
                    "gNodeB".into()
                },
                if i == 0 {
                    "gNodeB".into()
                } else {
                    "AMF".into()
                },
                if i == 0 {
                    "PDUSessionResourceReleaseCommand".into()
                } else {
                    "PDUSessionResourceReleaseResponse".into()
                },
                "Session 1; normal release".into(),
                &b,
                "NGAP",
            );
        }
        self.session_nas(
            id,
            crate::session::nas(vec![0x2e, 1, 1, 0xd4])?,
            true,
            false,
        )?;
        self.sessions.get_mut(&id).unwrap().active = false;
        let pending = self.inflight.keys().filter(|(u, _)| *u == id).count();
        self.sessions.get_mut(&id).unwrap().dropped += pending as u64;
        self.inflight.retain(|(u, _), _| *u != id);
        Ok(())
    }
    pub fn echo(&mut self, id: u32, loss_percent: u8) -> Result<(), String> {
        if loss_percent > 100 {
            return Err("Loss must be 0–100 percent".into());
        }
        if self.inflight.len() >= 128 {
            return Err("128 packets already in flight".into());
        }
        let s = self
            .sessions
            .get_mut(&id)
            .filter(|s| s.active)
            .ok_or("Establish a PDU session first")?;
        if s.sent >= 65535 {
            return Err("Echo sequence exhausted; reset experiment".into());
        }
        let seq = s.sent as u16;
        s.sent += 1;
        let ip = s.ip;
        let teid = s.uplink_teid;
        let packet = crate::session::echo_packet(ip, seq);
        let cell = self.ues[&id].serving_cell.ok_or("No serving cell")?;
        self.log(
            id,
            format!("UE-{id}"),
            format!("gNB-{cell}"),
            "ICMP Echo Request".into(),
            "IPv4 virtual destination 10.45.255.1; no internet routing".into(),
            &packet,
            "IPv4",
        );
        self.delay();
        let loss_draw = (self.rng % 100) as u64; // deterministic, explicitly injected packet loss
        if loss_percent > 0 && (loss_percent == 100 || loss_draw < loss_percent as u64) {
            self.sessions.get_mut(&id).unwrap().dropped += 1;
            self.log(
                id,
                "gNodeB".into(),
                "UPF".into(),
                "Injected packet loss".into(),
                format!("Configured loss {loss_percent}%"),
                &[],
                "STATE",
            );
            return Ok(());
        }
        self.inflight.insert((id, seq), self.time_ms);
        self.send(Frame {
            from: Endpoint::Gnodeb(cell),
            to: Endpoint::Upf,
            ue_id: id,
            cell_id: cell,
            bytes: crate::session::gtpu(teid, &packet)?,
            ngap: false,
        });
        Ok(())
    }
    fn deliver_data(
        &mut self,
        from: Endpoint,
        to: Endpoint,
        id: u32,
        cell: u32,
        wire: Vec<u8>,
    ) -> Result<(), String> {
        let Some(s) = self.sessions.get(&id).filter(|s| s.active) else {
            return Ok(());
        };
        let uplink = to == Endpoint::Upf;
        let packet = match crate::session::decode_gtpu(
            &wire,
            if uplink {
                s.uplink_teid
            } else {
                s.downlink_teid
            },
        ) {
            Ok(p) => p.to_vec(),
            Err(reason) => {
                self.log(
                    id,
                    format!("{from:?}"),
                    format!("{to:?}"),
                    "Dropped stale GTP-U".into(),
                    reason,
                    &wire,
                    "GTP-U",
                );
                return Ok(());
            }
        };
        let ip = s.ip;
        let teid = s.downlink_teid;
        self.log(
            id,
            format!("{from:?}"),
            format!("{to:?}"),
            "GTP-U G-PDU".into(),
            format!("TEID validated; {} octets IPv4", packet.len()),
            &wire,
            "GTP-U",
        );
        if uplink {
            let reply = crate::session::echo_reply(&packet, ip)?;
            self.log(
                id,
                "UPF".into(),
                "Virtual echo / UPF".into(),
                "ICMP Echo Reply".into(),
                "Local virtual data network".into(),
                &reply,
                "IPv4",
            );
            self.send(Frame {
                from: Endpoint::Upf,
                to: Endpoint::DataGnodeb(cell),
                ue_id: id,
                cell_id: cell,
                bytes: crate::session::gtpu(teid, &reply)?,
                ngap: false,
            });
        } else {
            if packet.len() != 36 || packet[16..20] != ip || packet[20] != 0 {
                return Err("Invalid downlink echo packet".into());
            }
            let seq = u16::from_be_bytes([packet[26], packet[27]]);
            if let Some(sent) = self.inflight.remove(&(id, seq)) {
                let s = self.sessions.get_mut(&id).unwrap();
                s.delivered += 1;
                s.last_rtt_ms = Some(self.time_ms - sent);
                self.log(
                    id,
                    format!("gNB-{cell}"),
                    format!("UE-{id}"),
                    "Echo delivered".into(),
                    format!("RTT {} ms virtual", self.time_ms - sent),
                    &packet,
                    "IPv4",
                );
            }
        }
        Ok(())
    }
    fn fail_ue(&mut self, id: u32, reason: String) {
        self.registrations
            .get_mut(&id)
            .unwrap()
            .fail(reason.clone());
        let u = self.ues.get_mut(&id).unwrap();
        u.state = UeState::RegistrationRejected;
        u.failure = Some(reason.clone());
        self.amf.insert(id, AmfState::Rejected);
        self.log(
            id,
            "ENGINE".into(),
            format!("UE-{id}"),
            "Registration failed".into(),
            reason,
            &[],
            "STATE",
        );
    }
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            time_ms: self.time_ms,
            started: self.started,
            pending: self.transport.pending()
                + usize::from(self.has_mobility())
                + usize::from(
                    self.started
                        && self.ues.values().any(|u| {
                            !matches!(
                                u.state,
                                UeState::Registered
                                    | UeState::RegistrationRejected
                                    | UeState::NoCoverage
                                    | UeState::ReassociationRequired
                                    | UeState::Deregistered
                            )
                        }),
                ),
            ues: self.ues.values().cloned().collect(),
            gnodebs: self.gnodebs.clone(),
            amf: self.amf.clone(),
            events: self.events.clone(),
            coverage: self.coverage.clone(),
            sessions: self.sessions.values().cloned().collect(),
            event_limit_reached: self.events.len() >= 16384,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn scenario() -> Scenario {
        serde_json::from_str(r#"{"version":1,"name":"test","seed":42,"center":[2.35,48.85],"gnodebs":[{"id":1,"name":"g","lon":2.35,"lat":48.85,"frequency_mhz":3500,"tx_power_dbm":46,"height_m":25,"gain_dbi":16}],"ues":[{"id":2,"name":"u","lon":2.351,"lat":48.85}],"buildings":[]}"#).unwrap()
    }
    #[test]
    fn exchange_has_real_binary_and_authenticated_registration() {
        let mut e = Engine::new(scenario()).unwrap();
        e.start().unwrap();
        assert_eq!(e.ues[&2].state, UeState::RegistrationInitiated);
        e.advance(1000).unwrap();
        assert_eq!(e.ues[&2].state, UeState::Registered);
        assert_eq!(e.snapshot().pending, 0);
        assert!(e.events.iter().any(|e| e.kind == "Security Mode Complete"));
        assert_eq!(e.events.last().unwrap().kind, "Registration Complete");
    }
    #[test]
    fn deterministic_across_step_sizes() {
        let mut a = Engine::new(scenario()).unwrap();
        let mut b = Engine::new(scenario()).unwrap();
        a.advance(500).unwrap();
        for _ in 0..50 {
            b.advance(10).unwrap();
        }
        assert_eq!(
            serde_json::to_string(&a.snapshot()).unwrap(),
            serde_json::to_string(&b.snapshot()).unwrap()
        );
    }
    #[test]
    fn seed_changes_delivery_schedule() {
        let mut s = scenario();
        s.seed = 43;
        let mut a = Engine::new(scenario()).unwrap();
        let mut b = Engine::new(s).unwrap();
        a.advance(500).unwrap();
        b.advance(500).unwrap();
        assert_ne!(a.events[1].time_ms, b.events[1].time_ms);
    }
    #[test]
    fn no_cell_no_request() {
        let mut s = scenario();
        s.gnodebs.clear();
        let mut e = Engine::new(s).unwrap();
        e.advance(100).unwrap();
        assert_eq!(e.ues[&2].state, UeState::NoCoverage);
        assert_eq!(e.snapshot().pending, 0);
    }
    #[test]
    fn concurrent_ues_keep_cell_relaying_until_last_delivery() {
        let mut s = scenario();
        s.ues.push(Ue {
            mobility: Default::default(),
            fault: LabFault::None,
            id: 3,
            name: "second".into(),
            lon: 2.351,
            lat: 48.85,
        });
        let mut e = Engine::new(s).unwrap();
        let mut saw_partial = false;
        for _ in 0..1000 {
            e.advance(1).unwrap();
            let rejected = e
                .ues
                .values()
                .filter(|u| u.state == UeState::Registered)
                .count();
            if rejected == 1 {
                saw_partial = true;
                assert!(matches!(e.gnodebs[&1], GnodebState::Relaying));
            }
        }
        assert!(saw_partial);
        assert!(e.ues.values().all(|u| u.state == UeState::Registered));
        assert!(matches!(e.gnodebs[&1], GnodebState::Idle));
    }
    #[test]
    fn bundled_paris_geometry_and_round_trip_are_valid() {
        let s: Scenario = serde_json::from_str(include_str!("../../examples/paris.json")).unwrap();
        let mut e = Engine::new(s).unwrap();
        e.advance(1000).unwrap();
        assert_eq!(
            e.ues
                .values()
                .filter(|u| u.state == UeState::Registered)
                .count(),
            3
        );
        assert_eq!(
            e.ues
                .values()
                .filter(|u| u.state == UeState::NoCoverage)
                .count(),
            1
        );
    }
    #[test]
    fn negative_registration_profiles_and_isolation() {
        for fault in [
            LabFault::UnknownSubscriber,
            LabFault::InvalidCredentials,
            LabFault::InvalidAutn,
            LabFault::IncorrectResponse,
            LabFault::InvalidMac,
            LabFault::InvalidCounter,
            LabFault::Timeout,
        ] {
            let mut s = scenario();
            s.ues[0].fault = fault;
            let mut good = s.ues[0].clone();
            good.id = 3;
            good.fault = LabFault::None;
            s.ues.push(good);
            let mut e = Engine::new(s).unwrap();
            e.advance(7000).unwrap();
            assert_eq!(e.ues[&2].state, UeState::RegistrationRejected, "{fault:?}");
            assert!(e.ues[&2].failure.is_some());
            assert_eq!(e.ues[&3].state, UeState::Registered, "{fault:?}");
            assert_eq!(e.snapshot().pending, 0);
        }
    }
    #[test]
    fn reject_unknown_fields_including_credentials() {
        let mut value = serde_json::to_value(scenario()).unwrap();
        value["ues"][0]["key"] = serde_json::json!("real credentials");
        assert!(serde_json::from_value::<Scenario>(value).is_err());
    }
    #[test]
    fn registration_session_echo_release() {
        let mut e = Engine::new(scenario()).unwrap();
        assert!(e.establish_session(2).is_err());
        e.advance(1000).unwrap();
        e.establish_session(2).unwrap();
        assert!(e.establish_session(2).is_err());
        assert_eq!(e.sessions[&2].ip, [10, 45, 0, 1]);
        e.echo(2, 0).unwrap();
        e.advance(1000).unwrap();
        assert_eq!(e.sessions[&2].delivered, 1);
        assert!((20..=60).contains(&e.sessions[&2].last_rtt_ms.unwrap()));
        e.echo(2, 100).unwrap();
        assert_eq!(e.sessions[&2].dropped, 1);
        e.release_session(2).unwrap();
        assert!(!e.sessions[&2].active);
        assert!(e.echo(2, 0).is_err());
        e.establish_session(2).unwrap();
        assert_eq!(e.sessions[&2].ip, [10, 45, 0, 2]);
        e.deregister(2).unwrap();
        assert_eq!(e.ues[&2].state, UeState::Deregistered);
        assert!(e.establish_session(2).is_err());
    }
    #[test]
    fn mobility_is_step_invariant_and_loses_coverage() {
        let mut s = scenario();
        s.ues[0].mobility = Mobility {
            waypoints: vec![[2.45, 48.85]],
            speed_mps: 100.0,
        };
        s.radio.duration_ms = 60000;
        s.radio.model = RadioKind::UMa;
        let mut a = Engine::new(s.clone()).unwrap();
        let mut b = Engine::new(s).unwrap();
        a.advance(60000).unwrap();
        for _ in 0..600 {
            b.advance(100).unwrap();
        }
        assert_eq!(
            serde_json::to_string(&a.snapshot()).unwrap(),
            serde_json::to_string(&b.snapshot()).unwrap()
        );
        assert_ne!(a.ues[&2].position, [2.351, 48.85]);
        assert!(a.ues[&2].rsrp.unwrap() < -100.0);
        assert_eq!(a.ues[&2].state, UeState::NoCoverage);
    }
    #[test]
    fn a3_reassociation_is_not_protocol_handover() {
        let mut s = scenario();
        let mut second = s.gnodebs[0].clone();
        second.id = 3;
        second.lon += 0.005;
        s.gnodebs.push(second);
        s.ues[0].mobility = Mobility {
            waypoints: vec![[2.355, 48.85]],
            speed_mps: 20.0,
        };
        s.radio.duration_ms = 30000;
        let mut e = Engine::new(s).unwrap();
        e.advance(30000).unwrap();
        assert_eq!(e.ues[&2].serving_cell, Some(3));
        assert_eq!(e.ues[&2].state, UeState::ReassociationRequired);
        assert!(
            e.events
                .iter()
                .any(|e| e.kind == "Serving cell changed"
                    && e.detail.contains("handover unsupported"))
        );
    }
    #[test]
    fn released_inflight_packet_cannot_reach_reused_session() {
        let mut e = Engine::new(scenario()).unwrap();
        e.advance(1000).unwrap();
        e.establish_session(2).unwrap();
        e.echo(2, 0).unwrap();
        e.release_session(2).unwrap();
        e.establish_session(2).unwrap();
        // Old TEID is invalid after new allocation; stale packets are dropped.
        e.advance(1000).unwrap();
        assert_eq!(e.sessions[&2].delivered, 0);
    }
    #[test]
    fn malformed_building_rejected() {
        let mut s = scenario();
        s.buildings.push(Building {
            id: "bad".into(),
            ring: vec![[0.0, 0.0], [1.0, 1.0], [0.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
        });
        assert!(Engine::new(s).is_err());
    }
    #[test]
    fn invalid_scenario_rejected() {
        let mut s = scenario();
        s.gnodebs[0].frequency_mhz = -1.0;
        assert!(Engine::new(s).is_err());
        let mut s = scenario();
        s.ues[0].id = 1;
        assert!(Engine::new(s).is_err());
    }
}
