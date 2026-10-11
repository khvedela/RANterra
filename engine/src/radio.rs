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
