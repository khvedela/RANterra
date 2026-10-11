use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gnodeb {
    pub id: u32,
    pub name: String,
    pub lon: f64,
    pub lat: f64,
    pub frequency_mhz: f64,
    pub tx_power_dbm: f64,
    pub height_m: f64,
    pub gain_dbi: f64,
    #[serde(default)]
    pub antenna: Antenna,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ue {
    #[serde(default)]
    pub session: SessionRequest,
    #[serde(default)]
    pub fault: LabFault,
    #[serde(default)]
    pub mobility: Mobility,
    pub id: u32,
    pub name: String,
    pub lon: f64,
    pub lat: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Building {
    pub id: String,
    pub ring: Vec<[f64; 2]>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    #[serde(default = "default_policies")]
    pub session_policies: Vec<SessionPolicy>,
    pub version: u32,
    #[serde(default)]
    pub radio: RadioConfig,
    pub name: String,
    pub seed: u32,
    pub center: [f64; 2],
    pub gnodebs: Vec<Gnodeb>,
    pub ues: Vec<Ue>,
    pub buildings: Vec<Building>,
}
fn coordinate(p: [f64; 2]) -> bool {
    p[0].is_finite() && p[1].is_finite() && p[0].abs() <= 180.0 && p[1].abs() <= 85.0
}
impl Scenario {
    pub fn validate(&self) -> Result<(), String> {
        if ![1, 2].contains(&self.version) {
            return Err("Unsupported scenario version (expected 1 or 2)".into());
        }
        if !coordinate(self.center) || self.name.len() > 120 {
            return Err("Invalid scenario center or name".into());
        }
        if self.gnodebs.len() > 16 || self.ues.len() > 128 || self.buildings.len() > 600 {
            return Err("Scenario exceeds limits: 16 cells, 128 UEs, 600 buildings".into());
        }
        self.radio.validate()?;
        if self.session_policies.is_empty() || self.session_policies.len() > 8 {
            return Err("Configure 1–8 session policies".into());
        }
        let mut policies = BTreeSet::new();
        for p in &self.session_policies {
            p.request.validate()?;
            if p.max_sessions > 128 || !policies.insert((&p.request.dnn, &p.request.snssai)) {
                return Err("Duplicate session policy or capacity above 128".into());
            }
        }
        let mut ids = BTreeSet::new();
        for g in &self.gnodebs {
            if !(0.0..=360.0).contains(&g.antenna.azimuth_deg)
                || !(-20.0..=90.0).contains(&g.antenna.downtilt_deg)
                || !(1..=275).contains(&g.antenna.resource_blocks)
                || g.antenna.numerology > 4
            {
                return Err("Invalid antenna configuration".into());
            }
            if self.radio.model != RadioKind::Legacy
                && (!(500.0..=100000.0).contains(&g.frequency_mhz)
                    || (g.height_m
                        - if self.radio.model == RadioKind::UMi {
                            10.0
                        } else {
                            25.0
                        })
                    .abs()
                        > 0.01)
            {
                return Err(
                    "TR 38.901 profile requires 0.5–100 GHz and UMi height 10 m / UMa height 25 m"
                        .into(),
                );
            }

            if !ids.insert(g.id)
                || g.id == 0
                || !coordinate([g.lon, g.lat])
                || g.name.len() > 80
                || !(100.0..=100_000.0).contains(&g.frequency_mhz)
                || !(-20.0..=80.0).contains(&g.tx_power_dbm)
                || !(1.5..=300.0).contains(&g.height_m)
                || !(-10.0..=40.0).contains(&g.gain_dbi)
            {
                return Err("Invalid gNodeB: unique positive ID, valid coordinates and radio parameters required".into());
            }
        }
        for u in &self.ues {
            u.session.validate()?;
            if u.mobility.waypoints.len() > 128
                || !u.mobility.waypoints.iter().all(|p| coordinate(*p))
                || !(0.0..=100.0).contains(&u.mobility.speed_mps)
            {
                return Err("Invalid mobility: ≤128 finite waypoints, speed 0–100 m/s".into());
            }

            if !ids.insert(u.id) || u.id == 0 || !coordinate([u.lon, u.lat]) || u.name.len() > 80 {
                return Err("Invalid UE or duplicate entity ID".into());
            }
        }
        let mut vertices = 0;
        for b in &self.buildings {
            vertices += b.ring.len();
            if b.id.len() > 120
                || b.ring.len() < 4
                || b.ring.len() > 100
                || b.ring.first() != b.ring.last()
                || !b.ring.iter().all(|p| coordinate(*p))
                || !simple_ring(&b.ring)
            {
                return Err(
                    "Building requires a closed, finite polygon with 4–100 coordinates".into(),
                );
            }
        }
        if vertices > 15000 {
            return Err("Too many building vertices (maximum 15000)".into());
        }
        Ok(())
    }
}

/// Reject degenerate or self-crossing footprints; no holes or multipolygons.
fn simple_ring(ring: &[[f64; 2]]) -> bool {
    let area: f64 = ring
        .windows(2)
        .map(|e| e[0][0] * e[1][1] - e[1][0] * e[0][1])
        .sum();
    if area.abs() < 1e-12 {
        return false;
    }
    let orient = |a: [f64; 2], b: [f64; 2], c: [f64; 2]| {
        (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    };
    for (i, a) in ring.windows(2).enumerate() {
        if a[0] == a[1] {
            return false;
        }
        for (j, b) in ring.windows(2).enumerate().skip(i + 2) {
            if i == 0 && j == ring.len() - 2 {
                continue;
            }
            if orient(a[0], a[1], b[0]) * orient(a[0], a[1], b[1]) < 0.0
                && orient(b[0], b[1], a[0]) * orient(b[0], b[1], a[1]) < 0.0
            {
                return false;
            }
        }
    }
    true
}

/// Reproducible negative experiments, never a credential import facility.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LabFault {
    #[default]
    None,
    UnknownSubscriber,
    InvalidCredentials,
    InvalidAutn,
    IncorrectResponse,
    InvalidMac,
    InvalidCounter,
    Timeout,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Antenna {
    pub sector: bool,
    pub azimuth_deg: f64,
    pub downtilt_deg: f64,
    pub resource_blocks: u16,
    pub numerology: u8,
}
impl Default for Antenna {
    fn default() -> Self {
        Self {
            sector: false,
            azimuth_deg: 0.0,
            downtilt_deg: 0.0,
            resource_blocks: 106,
            numerology: 1,
        }
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Mobility {
    pub waypoints: Vec<[f64; 2]>,
    pub speed_mps: f64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RadioKind {
    #[default]
    Legacy,
    UMi,
    UMa,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RadioConfig {
    pub model: RadioKind,
    pub shadow_fading: bool,
    pub coverage_dbm: f64,
    pub hysteresis_db: f64,
    pub time_to_trigger_ms: u64,
    pub duration_ms: u64,
}
impl Default for RadioConfig {
    fn default() -> Self {
        Self {
            model: RadioKind::Legacy,
            shadow_fading: false,
            coverage_dbm: -120.0,
            hysteresis_db: 3.0,
            time_to_trigger_ms: 320,
            duration_ms: 10000,
        }
    }
}
impl RadioConfig {
    fn validate(&self) -> Result<(), String> {
        if !(-150.0..=-50.0).contains(&self.coverage_dbm)
            || !(0.0..=20.0).contains(&self.hysteresis_db)
            || self.time_to_trigger_ms > 10000
            || !(1000..=3600000).contains(&self.duration_ms)
        {
            return Err("Invalid radio thresholds, TTT or experiment duration (1–3600 s)".into());
        }
        Ok(())
    }
}

/// Laboratory IPv4 service selection. SD uses its exact three on-wire octets.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct Snssai {
    pub sst: u8,
    #[serde(default)]
    pub sd: Option<[u8; 3]>,
}
impl Default for Snssai {
    fn default() -> Self {
        Self { sst: 1, sd: None }
    }
}
impl Snssai {
    pub fn bytes(&self) -> Vec<u8> {
        let mut b = vec![self.sst];
        if let Some(sd) = self.sd {
            b.extend(sd);
        }
        b
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct SessionRequest {
    pub dnn: String,
    pub snssai: Snssai,
}
impl Default for SessionRequest {
    fn default() -> Self {
        Self {
            dnn: "lab".into(),
            snssai: Snssai::default(),
        }
    }
}
impl SessionRequest {
    pub fn validate(&self) -> Result<(), String> {
        // Lower-case ASCII DNS-label profile; NAS DNN maximum encoded size is 100.
        if self.dnn.is_empty()
            || self.dnn.len() > 99
            || self.dnn.split('.').any(|l| {
                l.is_empty()
                    || l.len() > 63
                    || l.starts_with('-')
                    || l.ends_with('-')
                    || !l
                        .bytes()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
            })
        {
            return Err("DNN must contain lower-case DNS labels (maximum 99 characters)".into());
        }
        if self.snssai.sst == 0 || self.snssai.sd == Some([255; 3]) {
            return Err("S-NSSAI requires nonzero SST and SD other than ffffff".into());
        }
        Ok(())
    }
    pub fn dnn_bytes(&self) -> Vec<u8> {
        self.dnn
            .split('.')
            .flat_map(|s| std::iter::once(s.len() as u8).chain(s.bytes()))
            .collect()
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionPolicy {
    pub request: SessionRequest,
    pub max_sessions: u16,
}
fn default_policies() -> Vec<SessionPolicy> {
    vec![SessionPolicy {
        request: SessionRequest::default(),
        max_sessions: 128,
    }]
}
