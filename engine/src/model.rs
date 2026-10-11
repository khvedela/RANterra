use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Gnodeb {
    pub id: u32,
    pub name: String,
    pub lon: f64,
    pub lat: f64,
    pub frequency_mhz: f64,
    pub tx_power_dbm: f64,
    pub height_m: f64,
    pub gain_dbi: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ue {
    pub id: u32,
    pub name: String,
    pub lon: f64,
    pub lat: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Building {
    pub id: String,
    pub ring: Vec<[f64; 2]>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Scenario {
    pub version: u32,
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
        if self.version != 1 {
            return Err("Unsupported scenario version (expected 1)".into());
        }
        if !coordinate(self.center) || self.name.len() > 120 {
            return Err("Invalid scenario center or name".into());
        }
        if self.gnodebs.len() > 16 || self.ues.len() > 128 || self.buildings.len() > 600 {
            return Err("Scenario exceeds limits: 16 cells, 128 UEs, 600 buildings".into());
        }
        let mut ids = BTreeSet::new();
        for g in &self.gnodebs {
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
