//! Illustrative close-in log-distance model, NOT a calibrated 3GPP channel model.
use crate::model::{Building, Gnodeb};
use serde::Serialize;
pub trait RadioModel {
    fn rsrp(&self, cell: &Gnodeb, point: [f64; 2], buildings: &[Building]) -> f64;
}
pub struct LogDistance;
fn cross(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}
fn inside(p: [f64; 2], ring: &[[f64; 2]]) -> bool {
    let mut yes = false;
    for e in ring.windows(2) {
        let (a, b) = (e[0], e[1]);
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            yes = !yes;
        }
    }
    yes
}
pub fn obstructs(a: [f64; 2], b: [f64; 2], ring: &[[f64; 2]]) -> bool {
    let minx = ring.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
    let maxx = ring.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max);
    let miny = ring.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min);
    let maxy = ring.iter().map(|p| p[1]).fold(f64::NEG_INFINITY, f64::max);
    if a[0].max(b[0]) < minx
        || a[0].min(b[0]) > maxx
        || a[1].max(b[1]) < miny
        || a[1].min(b[1]) > maxy
    {
        return false;
    }
    inside(a, ring)
        || inside(b, ring)
        || ring.windows(2).any(|e| {
            cross(a, b, e[0]) * cross(a, b, e[1]) < 0.0
                && cross(e[0], e[1], a) * cross(e[0], e[1], b) < 0.0
        })
}
impl RadioModel for LogDistance {
    fn rsrp(&self, g: &Gnodeb, p: [f64; 2], buildings: &[Building]) -> f64 {
        let dx = (p[0] - g.lon) * 111_320.0 * ((p[1] + g.lat) * 0.5).to_radians().cos();
        let dy = (p[1] - g.lat) * 111_320.0;
        let d = (dx * dx + dy * dy + (g.height_m - 1.5).powi(2))
            .sqrt()
            .max(1.0);
        let loss = 32.4 + 20.0 * (g.frequency_mhz / 1000.0).log10() + 30.0 * d.log10();
        let count = buildings
            .iter()
            .filter(|b| obstructs([g.lon, g.lat], p, &b.ring))
            .take(3)
            .count();
        g.tx_power_dbm + g.gain_dbi - 10.0 * (12.0_f64 * 106.0).log10() - loss - 12.0 * count as f64
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Sample {
    pub position: [f64; 2],
    pub rsrp: f64,
    pub cell_id: u32,
}
pub fn strongest(
    model: &dyn RadioModel,
    cells: &[Gnodeb],
    p: [f64; 2],
    buildings: &[Building],
) -> Option<(u32, f64)> {
    cells
        .iter()
        .map(|g| (g.id, model.rsrp(g, p, buildings)))
        .max_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn distance_and_obstruction_reduce_signal() {
        let g = Gnodeb {
            antenna: Default::default(),
            id: 1,
            name: "a".into(),
            lon: 0.0,
            lat: 0.0,
            frequency_mhz: 3500.0,
            tx_power_dbm: 46.0,
            height_m: 25.0,
            gain_dbi: 16.0,
        };
        let near = LogDistance.rsrp(&g, [0.001, 0.0], &[]);
        let far = LogDistance.rsrp(&g, [0.01, 0.0], &[]);
        assert!(near > far + 20.0);
        let b = Building {
            id: "a".into(),
            ring: vec![
                [0.0004, -0.001],
                [0.0006, -0.001],
                [0.0006, 0.001],
                [0.0004, 0.001],
                [0.0004, -0.001],
            ],
        };
        assert!((near - LogDistance.rsrp(&g, [0.001, 0.0], &[b]) - 12.0).abs() < 1e-9);
    }
}

/// TR 38.901 v18.0.0 Table 7.4.1-1, fixed UT 1.5 m, hE=1 m.
/// None outside the supported domain; never silently extrapolate.
pub fn path_loss(kind: crate::model::RadioKind, los: bool, d2: f64, fc: f64) -> Option<f64> {
    use crate::model::RadioKind::*;
    if !(10.0..=5000.0).contains(&d2) || !(0.5..=100.0).contains(&fc) || kind == Legacy {
        return None;
    }
    let h = if kind == UMi { 10.0_f64 } else { 25.0_f64 };
    let d3 = (d2 * d2 + (h - 1.5).powi(2)).sqrt();
    let bp = 4.0 * (h - 1.0) * 0.5 * fc * 1e9 / 3e8;
    let pl_los = if kind == UMi {
        if d2 <= bp {
            32.4 + 21.0 * d3.log10() + 20.0 * fc.log10()
        } else {
            32.4 + 40.0 * d3.log10() + 20.0 * fc.log10()
                - 9.5 * (bp * bp + (h - 1.5).powi(2)).log10()
        }
    } else if d2 <= bp {
        28.0 + 22.0 * d3.log10() + 20.0 * fc.log10()
    } else {
        28.0 + 40.0 * d3.log10() + 20.0 * fc.log10() - 9.0 * (bp * bp + (h - 1.5).powi(2)).log10()
    };
    let nlos = if kind == UMi {
        35.3 * d3.log10() + 22.4 + 21.3 * fc.log10()
    } else {
        13.54 + 39.08 * d3.log10() + 20.0 * fc.log10()
    };
    Some(if los { pl_los } else { pl_los.max(nlos) })
}
pub fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    let dx = (b[0] - a[0]) * 111320.0 * ((a[1] + b[1]) * 0.5).to_radians().cos();
    let dy = (b[1] - a[1]) * 111320.0;
    dx.hypot(dy)
}
type IndexedFootprint = (f64, f64, f64, f64, Vec<[f64; 2]>);
pub struct Tr38901 {
    kind: crate::model::RadioKind,
    seed: u32,
    shadow: bool,
    // Sorted longitude interval index. Bounds are built once, not per RF sample.
    geometry: Vec<IndexedFootprint>,
}
impl Tr38901 {
    pub fn new(s: &crate::model::Scenario) -> Self {
        let mut geometry: Vec<_> = s
            .buildings
            .iter()
            .map(|b| {
                (
                    b.ring.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min),
                    b.ring
                        .iter()
                        .map(|p| p[0])
                        .fold(f64::NEG_INFINITY, f64::max),
                    b.ring.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min),
                    b.ring
                        .iter()
                        .map(|p| p[1])
                        .fold(f64::NEG_INFINITY, f64::max),
                    b.ring.clone(),
                )
            })
            .collect();
        geometry.sort_by(|a, b| a.0.total_cmp(&b.0));
        Self {
            kind: s.radio.model,
            seed: s.seed,
            shadow: s.radio.shadow_fading,
            geometry,
        }
    }
    fn los(&self, a: [f64; 2], b: [f64; 2]) -> bool {
        let end = self.geometry.partition_point(|g| g.0 <= a[0].max(b[0]));
        !self.geometry[..end].iter().any(|g| {
            g.1 >= a[0].min(b[0])
                && g.2 <= a[1].max(b[1])
                && g.3 >= a[1].min(b[1])
                && obstructs(a, b, &g.4)
        })
    }
}
pub fn antenna_loss(g: &Gnodeb, p: [f64; 2]) -> f64 {
    if !g.antenna.sector {
        return 0.0;
    }
    let dx = (p[0] - g.lon) * ((g.lat + p[1]) * 0.5).to_radians().cos();
    let dy = p[1] - g.lat;
    let bearing = dx.atan2(dy).to_degrees();
    let angle = (bearing - g.antenna.azimuth_deg + 180.0).rem_euclid(360.0) - 180.0;
    let elevation = ((g.height_m - 1.5) / distance([g.lon, g.lat], p).max(1.0))
        .atan()
        .to_degrees();
    (12.0 * (angle / 65.0).powi(2) + 12.0 * ((elevation - g.antenna.downtilt_deg) / 65.0).powi(2))
        .min(30.0)
}
impl RadioModel for Tr38901 {
    fn rsrp(&self, g: &Gnodeb, p: [f64; 2], _: &[Building]) -> f64 {
        let d = distance([g.lon, g.lat], p);
        let los = self.los([g.lon, g.lat], p);
        let Some(loss) = path_loss(self.kind, los, d, g.frequency_mhz / 1000.0) else {
            return -300.0;
        };
        let shadow = if self.shadow {
            // Independent 0.00025-degree grid values; NOT the TR spatial-correlation process.
            let mut seed = self.seed
                ^ g.id.wrapping_mul(2654435761)
                ^ ((p[0] * 4000.0).floor() as i64 as u32).rotate_left(13)
                ^ (p[1] * 4000.0).floor() as i64 as u32;
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let u = (seed as f64 + 1.0) / 4294967297.0;
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let v = (seed as f64 + 1.0) / 4294967297.0;
            let sigma = if los {
                4.0
            } else if self.kind == crate::model::RadioKind::UMi {
                7.82
            } else {
                6.0
            };
            sigma * (-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * v).cos()
        } else {
            0.0
        };
        g.tx_power_dbm + g.gain_dbi
            - antenna_loss(g, p)
            - 10.0 * (12.0 * g.antenna.resource_blocks as f64).log10()
            - loss
            - shadow
    }
}
/// Full-load, co-channel interference per subcarrier; receiver NF=7 dB.
pub fn sinr(
    model: &dyn RadioModel,
    cells: &[Gnodeb],
    point: [f64; 2],
    buildings: &[Building],
    serving: u32,
) -> Option<f64> {
    let cell = cells.iter().find(|g| g.id == serving)?;
    let signal = model.rsrp(cell, point, buildings);
    if signal <= -300.0 {
        return None;
    }
    let scs = 15000.0 * 2.0_f64.powi(cell.antenna.numerology as i32);
    let noise = -174.0 + 10.0 * scs.log10() + 7.0;
    let interference: f64 = cells
        .iter()
        .filter(|g| g.id != serving && (g.frequency_mhz - cell.frequency_mhz).abs() < 0.001)
        .map(|g| 10.0_f64.powf(model.rsrp(g, point, buildings) / 10.0))
        .sum();
    Some(signal - 10.0 * (10.0_f64.powf(noise / 10.0) + interference).log10())
}
#[cfg(test)]
mod research_tests {
    use super::*;
    use crate::model::RadioKind::*;
    #[test]
    fn tr38901_reference_values() {
        // Independent calculator fixtures, hUT=1.5 m; d2D=100 m; fc=3.5 GHz.
        assert!((path_loss(UMi, true, 100.0, 3.5).unwrap() - 85.314188).abs() < 1e-5);
        assert!((path_loss(UMa, true, 100.0, 3.5).unwrap() - 83.1381567).abs() < 1e-5);
        for m in [UMi, UMa] {
            assert!(path_loss(m, false, 100.0, 3.5) > path_loss(m, true, 100.0, 3.5));
            assert!(path_loss(m, true, 9.9, 3.5).is_none());
            assert!(path_loss(m, true, 5001.0, 3.5).is_none());
        }
    }
    #[test]
    fn deterministic_shadow_antenna_interference() {
        let mut s: crate::model::Scenario =
            serde_json::from_str(include_str!("../../examples/minimal.json")).unwrap();
        s.radio.model = UMa;
        s.radio.shadow_fading = true;
        let m = Tr38901::new(&s);
        let g = &s.gnodebs[0];
        let p = [g.lon + 0.002, g.lat];
        assert_eq!(m.rsrp(g, p, &[]), m.rsrp(g, p, &[]));
        let one = sinr(&m, &s.gnodebs, p, &[], g.id).unwrap();
        let mut second = g.clone();
        second.id = 999;
        s.gnodebs.push(second);
        assert!(sinr(&m, &s.gnodebs, p, &[], s.gnodebs[0].id).unwrap() < one);
        let mut g = s.gnodebs[0].clone();
        g.antenna.sector = true;
        g.antenna.azimuth_deg = 90.0;
        let front = m.rsrp(&g, p, &[]);
        g.antenna.azimuth_deg = 270.0;
        assert!(m.rsrp(&g, p, &[]) < front - 20.0);
    }
}
