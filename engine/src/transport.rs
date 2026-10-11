//! Virtual transport boundary. A native adapter can implement this without browser APIs.
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Endpoint {
    Ue(u32),
    Gnodeb(u32),
    Amf,
    Upf,
    DataGnodeb(u32),
}
#[derive(Clone, Debug)]
pub struct Frame {
    pub from: Endpoint,
    pub to: Endpoint,
    pub ue_id: u32,
    pub cell_id: u32,
    pub bytes: Vec<u8>,
    pub ngap: bool,
}
pub trait Transport {
    fn send(&mut self, at: u64, frame: Frame);
    fn receive(&mut self, until: u64) -> Option<(u64, Frame)>;
    fn pending(&self) -> usize;
    fn next_time(&self) -> Option<u64>;
}
#[derive(Default)]
pub struct VirtualTransport {
    sequence: u64,
    queue: std::collections::BTreeMap<(u64, u64), Frame>,
}
impl Transport for VirtualTransport {
    fn send(&mut self, at: u64, frame: Frame) {
        self.queue.insert((at, self.sequence), frame);
        self.sequence += 1;
    }
    fn receive(&mut self, until: u64) -> Option<(u64, Frame)> {
        let key = *self.queue.first_key_value()?.0;
        if key.0 > until {
            return None;
        }
        self.queue.remove(&key).map(|frame| (key.0, frame))
    }
    fn next_time(&self) -> Option<u64> {
        self.queue.first_key_value().map(|(k, _)| k.0)
    }
    fn pending(&self) -> usize {
        self.queue.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn schedule_orders_by_time_then_insertion() {
        let mut t = VirtualTransport::default();
        for (at, id) in [(20, 1), (10, 2), (10, 3)] {
            t.send(
                at,
                Frame {
                    from: Endpoint::Ue(id),
                    to: Endpoint::Amf,
                    ue_id: id,
                    cell_id: 1,
                    bytes: vec![],
                    ngap: false,
                },
            );
        }
        assert!(t.receive(9).is_none());
        assert_eq!(t.receive(10).unwrap().1.ue_id, 2);
        assert_eq!(t.receive(10).unwrap().1.ue_id, 3);
        assert_eq!(t.pending(), 1);
        assert_eq!(t.receive(20).unwrap().1.ue_id, 1);
    }
}
