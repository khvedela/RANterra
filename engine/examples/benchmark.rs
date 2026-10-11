use ranterra_engine::{
    model::{RadioKind, Scenario},
    simulation::Engine,
};
use std::time::Instant;
fn main() {
    let mut s: Scenario = serde_json::from_str(include_str!("../../examples/paris.json")).unwrap();
    s.radio.model = RadioKind::UMa;
    let base = s.gnodebs[0].clone();
    s.gnodebs = (1..=16)
        .map(|id| {
            let mut g = base.clone();
            g.id = id;
            g.height_m = 25.0;
            g.lon += (id % 4) as f64 * 0.001;
            g.lat += (id / 4) as f64 * 0.001;
            g
        })
        .collect();
    let base = s.ues[0].clone();
    s.ues = (101..229)
        .map(|id| {
            let mut u = base.clone();
            u.id = id;
            u.lon += (id % 16) as f64 * 0.0001;
            u.lat += ((id - 101) / 16) as f64 * 0.0001;
            u
        })
        .collect();
    let mut timings = Vec::new();
    let mut protocol = Vec::new();
    let mut snapshot_bytes = 0;
    for _ in 0..10 {
        let t = Instant::now();
        let mut e = Engine::new(s.clone()).unwrap();
        timings.push(t.elapsed().as_secs_f64() * 1000.0);
        let t = Instant::now();
        e.advance(1000).unwrap();
        protocol.push(t.elapsed().as_secs_f64() * 1000.0);
        snapshot_bytes = serde_json::to_vec(&e.snapshot()).unwrap().len();
    }
    timings.sort_by(f64::total_cmp);
    protocol.sort_by(f64::total_cmp);
    println!(
        "{}",
        serde_json::json!({"cells":16,"ues":128,"buildings":s.buildings.len(),"samples":1225,"runs":10,"rf_constructor_median_ms":timings[5],"rf_constructor_max_ms":timings[9],"registration_median_ms":protocol[5],"snapshot_json_bytes":snapshot_bytes})
    );
}
