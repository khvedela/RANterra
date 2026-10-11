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
    RequestAtAmf,
    RegistrationRejected,
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
}
#[derive(Clone, Debug, Serialize)]
pub struct UeStatus {
    pub id: u32,
    pub state: UeState,
    pub serving_cell: Option<u32>,
    pub rsrp: Option<f64>,
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
    model: Box<dyn RadioModel>,
    coverage: Vec<Sample>,
}
impl Engine<VirtualTransport> {
    pub fn new(scenario: Scenario) -> Result<Self, String> {
        Self::with_adapters(scenario, VirtualTransport::default(), Box::new(LogDistance))
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
            model,
            coverage: vec![],
        };
        for g in &e.scenario.gnodebs {
            e.gnodebs.insert(g.id, GnodebState::Ready);
        }
        for u in &e.scenario.ues {
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
                    serving_cell: signal.filter(|s| s.1 >= -120.0).map(|s| s.0),
                    rsrp: signal.map(|s| s.1),
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
        while let Some((time, frame)) = self.transport.receive(until) {
            self.time_ms = time;
            self.deliver(frame)?;
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
        let (kind, detail) = protocol::inspect(&bytes)?;
        match to {
            Endpoint::Gnodeb(_) => {
                self.gnodebs.insert(cell_id, GnodebState::Relaying);
                let target = if matches!(from, Endpoint::Ue(_)) {
                    Endpoint::Amf
                } else {
                    Endpoint::Ue(ue_id)
                };
                self.log(
                    ue_id,
                    format!("gNB-{cell_id}"),
                    if target == Endpoint::Amf {
                        "AMF".into()
                    } else {
                        format!("UE-{ue_id}")
                    },
                    format!("Relay · {kind}"),
                    detail,
                    &bytes,
                    "NAS",
                );
                let ngap = target == Endpoint::Amf;
                let bytes = if ngap {
                    crate::ngap::initial(bytes, ue_id, cell_id)?
                } else {
                    bytes
                };
                self.send(Frame {
                    from: Endpoint::Gnodeb(cell_id),
                    to: target,
                    ue_id,
                    cell_id,
                    bytes,
                    ngap,
                });
            }
            Endpoint::Amf => {
                if kind != "Registration Request" {
                    return Err("AMF expected Registration Request".into());
                }
                self.ues.get_mut(&ue_id).ok_or("Unknown UE")?.state = UeState::RequestAtAmf;
                self.amf.insert(ue_id, AmfState::Evaluating);
                self.log(
                    ue_id,
                    format!("gNB-{cell_id}"),
                    "AMF".into(),
                    "Request decoded".into(),
                    detail,
                    &bytes,
                    "NAS",
                );
                let reject = protocol::registration_reject()?;
                self.amf.insert(ue_id, AmfState::Rejected);
                self.log(ue_id,"AMF".into(),format!("gNB-{cell_id}"),"Registration Reject".into(),"Cause 7: 5GS services not allowed. Lab policy: authentication and security are not implemented; UE remains unregistered.".into(),&reject,"NAS");
                self.send(Frame {
                    from: Endpoint::Amf,
                    to: Endpoint::Gnodeb(cell_id),
                    ue_id,
                    cell_id,
                    bytes: crate::ngap::downlink(reject, ue_id)?,
                    ngap: true,
                });
            }
            Endpoint::Ue(_) => {
                if kind != "Registration Reject" || self.ues[&ue_id].state != UeState::RequestAtAmf
                {
                    return Err("Unexpected UE transition".into());
                }
                self.ues.get_mut(&ue_id).unwrap().state = UeState::RegistrationRejected;
                let has_pending = self.ues.values().any(|u| {
                    u.serving_cell == Some(cell_id)
                        && matches!(
                            u.state,
                            UeState::RegistrationInitiated | UeState::RequestAtAmf
                        )
                });
                self.gnodebs.insert(
                    cell_id,
                    if has_pending {
                        GnodebState::Relaying
                    } else {
                        GnodebState::Idle
                    },
                );
                self.log(
                    ue_id,
                    format!("gNB-{cell_id}"),
                    format!("UE-{ue_id}"),
                    "Registration rejected".into(),
                    detail,
                    &bytes,
                    "NAS",
                );
            }
        }
        Ok(())
    }
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            time_ms: self.time_ms,
            started: self.started,
            pending: self.transport.pending(),
            ues: self.ues.values().cloned().collect(),
            gnodebs: self.gnodebs.clone(),
            amf: self.amf.clone(),
            events: self.events.clone(),
            coverage: self.coverage.clone(),
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
    fn exchange_has_real_binary_and_never_registers() {
        let mut e = Engine::new(scenario()).unwrap();
        e.start().unwrap();
        assert_eq!(e.ues[&2].state, UeState::RegistrationInitiated);
        e.advance(200).unwrap();
        assert_eq!(e.ues[&2].state, UeState::RegistrationRejected);
        assert_eq!(e.snapshot().pending, 0);
        assert_eq!(e.events.len(), 8);
        assert!(e.events.last().unwrap().hex.ends_with("44 07"));
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
            id: 3,
            name: "second".into(),
            lon: 2.351,
            lat: 48.85,
        });
        let mut e = Engine::new(s).unwrap();
        let mut saw_partial = false;
        for _ in 0..150 {
            e.advance(1).unwrap();
            let rejected = e
                .ues
                .values()
                .filter(|u| u.state == UeState::RegistrationRejected)
                .count();
            if rejected == 1 {
                saw_partial = true;
                assert!(matches!(e.gnodebs[&1], GnodebState::Relaying));
            }
        }
        assert!(saw_partial);
        assert!(e
            .ues
            .values()
            .all(|u| u.state == UeState::RegistrationRejected));
        assert!(matches!(e.gnodebs[&1], GnodebState::Idle));
    }
    #[test]
    fn bundled_paris_geometry_and_round_trip_are_valid() {
        let s: Scenario = serde_json::from_str(include_str!("../../examples/paris.json")).unwrap();
        let mut e = Engine::new(s).unwrap();
        e.advance(500).unwrap();
        assert_eq!(
            e.ues
                .values()
                .filter(|u| u.state == UeState::RegistrationRejected)
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
