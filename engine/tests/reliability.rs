//! Reproducible bounded mutation/soak checks, not a substitute for coverage-guided fuzzing.
use ranterra_engine::{
    experiment::{Action, Experiment},
    model::{Scenario, SessionRequest, Snssai},
    ngap, protocol, session,
};
fn scenario() -> Scenario {
    serde_json::from_str(include_str!("../../examples/minimal.json")).unwrap()
}
fn next(seed: &mut u32) -> u32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    *seed
}
#[test]
fn deterministic_packet_mutation_campaign() {
    let req = protocol::registration_request(101).unwrap();
    let service = SessionRequest::default();
    let corpus = [
        req.clone(),
        ngap::initial(req, 101, 1).unwrap(),
        session::n1(session::request().unwrap(), true, true, &service).unwrap(),
        session::setup(101, 2, vec![0x7e, 0, 0x43], &Snssai::default()).unwrap(),
        session::setup_response(101, 3).unwrap(),
        session::gtpu(2, &session::echo_packet([10, 45, 0, 1], 0)).unwrap(),
    ];
    let mut seed = 0x5eed501;
    for index in 0..12_000 {
        let mut b = corpus[index % corpus.len()].clone();
        match index % 4 {
            0 => {
                let n = (next(&mut seed) as usize) % (b.len() + 1);
                b.truncate(n)
            }
            1 => {
                for _ in 0..3 {
                    let i = next(&mut seed) as usize % b.len();
                    b[i] ^= next(&mut seed) as u8;
                }
            }
            2 => b.extend(vec![next(&mut seed) as u8; next(&mut seed) as usize % 32]),
            _ => {
                b = (0..next(&mut seed) % 256)
                    .map(|_| next(&mut seed) as u8)
                    .collect()
            }
        }
        // Catch here identifies exact deterministic failing input. Production must never rely on unwind.
        let result = std::panic::catch_unwind(|| {
            let _ = protocol::inspect(&b);
            let _ = session::nas(b.clone());
            let _ = session::unwrap_n1(&b, true);
            let _ = session::selection(&b);
            let _ = session::response_teid(&b, 101);
            let _ = session::validate_release_response(&b, 101);
            let _ = ranterra_engine::auth::resynchronize(101, &[0; 16], &b, 1);
            let _ = ngap::unwrap(&b, 101);
            let _ = session::decode_setup(&b, 101, 2, &Snssai::default());
            let _ = session::validate_response(&b, 101, 3);
            let _ = session::decode_gtpu(&b, 2);
            let _ = session::echo_reply(&b, [10, 45, 0, 1]);
        });
        assert!(result.is_ok(), "mutation {index}: {b:02x?}");
    }
}
#[test]
fn repeated_session_lifecycle_exhaustion_cleanup_and_replay() {
    let mut run = Experiment::new(scenario()).unwrap();
    run.advance(1000).unwrap();
    for cycle in 0..254 {
        run.execute(Action::Establish { ue_id: 101 }).unwrap();
        run.advance(100).unwrap();
        run.execute(Action::Echo {
            ue_id: 101,
            loss_percent: 0,
        })
        .unwrap();
        if cycle % 2 == 0 {
            run.advance(100).unwrap();
        }
        run.execute(Action::Release { ue_id: 101 }).unwrap();
        run.advance(100).unwrap();
        run.advance(100).unwrap();
        let snap = run.snapshot();
        let s = &snap.sessions[0];
        assert!(!s.active);
        assert_eq!(s.sent, s.delivered + s.dropped);
        assert_eq!(snap.pending, 0);
        assert!(snap.events.len() <= 16384);
    }
    run.execute(Action::Establish { ue_id: 101 }).unwrap();
    run.advance(100).unwrap();
    assert_eq!(run.snapshot().session_failures[&101].cause, 26);
    run.execute(Action::Deregister { ue_id: 101 }).unwrap();
    assert!(run.snapshot().session_failures.is_empty());
    assert_eq!(run.snapshot().pending, 0);
    let restored = Experiment::replay(run.recording().unwrap()).unwrap();
    assert_eq!(
        run.recording().unwrap().state_digest,
        restored.recording().unwrap().state_digest
    );
}
#[test]
fn queue_saturation_is_transactional_and_drains() {
    let mut run = Experiment::new(scenario()).unwrap();
    run.advance(1000).unwrap();
    run.execute(Action::Establish { ue_id: 101 }).unwrap();
    run.advance(100).unwrap();
    for _ in 0..128 {
        run.execute(Action::Echo {
            ue_id: 101,
            loss_percent: 0,
        })
        .unwrap();
    }
    let before = run.recording().unwrap().state_digest;
    assert!(run
        .execute(Action::Echo {
            ue_id: 101,
            loss_percent: 0
        })
        .is_err());
    assert_eq!(before, run.recording().unwrap().state_digest);
    run.advance(1000).unwrap();
    let s = &run.snapshot().sessions[0];
    assert_eq!(s.delivered, 128);
    assert_eq!(s.dropped, 0);
    assert_eq!(run.snapshot().pending, 0);
}

#[derive(Clone, Default)]
struct Injectable(std::rc::Rc<std::cell::RefCell<ranterra_engine::transport::VirtualTransport>>);
impl ranterra_engine::transport::Transport for Injectable {
    fn send(&mut self, at: u64, frame: ranterra_engine::transport::Frame) {
        self.0.borrow_mut().send(at, frame)
    }
    fn receive(&mut self, until: u64) -> Option<(u64, ranterra_engine::transport::Frame)> {
        self.0.borrow_mut().receive(until)
    }
    fn next_time(&self) -> Option<u64> {
        self.0.borrow().next_time()
    }
    fn pending(&self) -> usize {
        self.0.borrow().pending()
    }
}
#[test]
fn malformed_transport_isolated_and_lost_echo_times_out() {
    use ranterra_engine::{
        radio::LogDistance,
        simulation::{Engine, UeState},
        transport::{Endpoint, Frame, Transport},
    };
    let mut s = scenario();
    let mut other = s.ues[0].clone();
    other.id = 102;
    s.ues.push(other);
    let mut transport = Injectable::default();
    let mut engine = Engine::with_adapters(s, transport.clone(), Box::new(LogDistance)).unwrap();
    // Bad contexts and malformed APER must not panic or abort the valid registration run.
    for id in [999, 101] {
        transport.send(
            1,
            Frame {
                from: Endpoint::Gnodeb(1),
                to: Endpoint::Amf,
                ue_id: id,
                cell_id: 1,
                bytes: vec![255; 4],
                ngap: true,
            },
        );
    }
    engine.advance(1000).unwrap();
    assert!(engine
        .snapshot()
        .ues
        .iter()
        .all(|u| u.state == UeState::Registered));
    assert_eq!(
        engine
            .snapshot()
            .events
            .iter()
            .filter(|e| e.kind == "Dropped malformed frame")
            .count(),
        2
    );
    transport.send(
        1001,
        Frame {
            from: Endpoint::Gnodeb(1),
            to: Endpoint::Amf,
            ue_id: 101,
            cell_id: 1,
            bytes: ngap::uplink(vec![0x7e, 0, 0x43], 101, 1).unwrap(),
            ngap: true,
        },
    );
    engine.advance(1).unwrap();
    assert_eq!(engine.snapshot().ues[0].state, UeState::Registered);
    engine.establish_session(101).unwrap();
    engine.advance(100).unwrap();
    engine.establish_session(102).unwrap();
    engine.advance(100).unwrap();
    engine.echo(101, 0).unwrap();
    // Consume its real uplink and re-inject a corrupt packet with the correct tunnel.
    let (at, mut packet) = transport.receive(2000).unwrap();
    packet.bytes[8 + 35] ^= 1;
    transport.send(at, packet);
    engine.echo(102, 0).unwrap();
    engine.advance(1000).unwrap();
    assert_eq!(engine.snapshot().sessions[1].delivered, 1);
    assert!(engine.snapshot().pending > 0);
    engine.advance(1000).unwrap();
    let snap = engine.snapshot();
    assert_eq!(snap.sessions[0].dropped, 1);
    assert_eq!(snap.sessions[0].delivered, 0);
    assert_eq!(snap.pending, 0);
    assert!(snap
        .events
        .iter()
        .any(|e| e.kind == "Echo timeout" && e.time_ms == 3201));
    engine.echo(101, 0).unwrap();
    engine.advance(1000).unwrap();
    assert_eq!(engine.snapshot().sessions[0].delivered, 1);
}
