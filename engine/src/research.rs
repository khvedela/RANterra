//! Versioned research boundaries: deterministic observations and permitted typed actions.
use crate::{model::Ue, radio::distance, simulation::Snapshot};
use serde::{Deserialize, Serialize};
pub trait MobilityModel {
    fn position(&self, ue: &Ue, time_ms: u64) -> [f64; 2];
}
pub struct Waypoints;
impl MobilityModel for Waypoints {
    fn position(&self, ue: &Ue, time_ms: u64) -> [f64; 2] {
        let mut a = [ue.lon, ue.lat];
        let mut remaining = ue.mobility.speed_mps * time_ms as f64 / 1000.0;
        for b in &ue.mobility.waypoints {
            let length = distance(a, *b);
            if length > 0.0 && remaining < length {
                let t = remaining / length;
                return [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
            }
            remaining -= length;
            a = *b;
        }
        a
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ControllerAction {
    Hold,
    SuggestCell { ue_id: u32, cell_id: u32 },
}
pub trait NetworkController {
    fn actions(&mut self, observation: &Snapshot) -> Vec<ControllerAction>;
}
/// Reproducible no-op baseline. No training, AI claims or hidden network mutations.
pub struct HoldPolicy;
impl NetworkController for HoldPolicy {
    fn actions(&mut self, _: &Snapshot) -> Vec<ControllerAction> {
        vec![ControllerAction::Hold]
    }
}
pub trait MetricsCollector {
    fn observe(&mut self, snapshot: &Snapshot);
}
pub trait ScenarioExecutor {
    fn step(&mut self, virtual_ms: u64) -> Result<(), String>;
    fn observation(&self) -> Snapshot;
}
impl<T: crate::transport::Transport> ScenarioExecutor for crate::simulation::Engine<T> {
    fn step(&mut self, virtual_ms: u64) -> Result<(), String> {
        self.advance(virtual_ms)
    }
    fn observation(&self) -> Snapshot {
        self.snapshot()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deterministic_velocity_and_end() {
        let u:Ue=serde_json::from_str(r#"{"id":1,"name":"walk","lon":0,"lat":0,"mobility":{"speed_mps":1,"waypoints":[[0,0.001]]}}"#).unwrap();
        assert_eq!(Waypoints.position(&u, 0), [0.0, 0.0]);
        assert!((distance([0.0, 0.0], Waypoints.position(&u, 10000)) - 10.0).abs() < 1e-6);
        assert_eq!(Waypoints.position(&u, 200000), [0.0, 0.001]);
    }
}
