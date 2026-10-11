use ranterra_engine::{
    experiment::{Action, Experiment},
    model::LabFault,
    model::{Scenario, SignalingLoss},
    protocol,
    registration::Registration,
    simulation::UeState,
};
fn scenario() -> Scenario {
    serde_json::from_str(include_str!("../../examples/minimal.json")).unwrap()
}
fn compare(a: &Experiment, b: &Experiment) {
    assert_eq!(
        serde_json::to_value(a.snapshot()).unwrap(),
        serde_json::to_value(b.snapshot()).unwrap()
    );
}
#[test]
fn every_registration_message_recovers_from_a_single_loss_and_replays() {
    for loss in [
        SignalingLoss::RegistrationRequest,
        SignalingLoss::AuthenticationRequest,
        SignalingLoss::AuthenticationResponse,
        SignalingLoss::SecurityModeCommand,
        SignalingLoss::SecurityModeComplete,
        SignalingLoss::RegistrationAccept,
        SignalingLoss::RegistrationComplete,
    ] {
        let mut s = scenario();
        s.ues[0].signaling.drop = loss;
        s.ues[0].signaling.count = 1;
        let mut a = Experiment::new(s.clone()).unwrap();
        let mut b = Experiment::new(s).unwrap();
        a.advance(28000).unwrap();
        for _ in 0..280 {
            b.advance(100).unwrap();
        }
        compare(&a, &b);
        assert_eq!(
            a.snapshot().ues[0].state,
            UeState::Registered,
            "{loss:?}: {:?}",
            a.snapshot().ues[0].failure
        );
        assert!(a.snapshot().ues[0].registration_latency_ms.unwrap() >= 6000);
        compare(&a, &Experiment::replay(a.recording().unwrap()).unwrap());
    }
}
#[test]
fn nas_timers_exhaust_without_fabricating_registration() {
    let mut s = scenario();
    s.ues[0].signaling.drop = SignalingLoss::RegistrationComplete;
    s.ues[0].signaling.count = 5;
    let mut a = Experiment::new(s).unwrap();
    a.advance(32000).unwrap();
    assert_eq!(a.snapshot().ues[0].state, UeState::RegistrationUnconfirmed);
    assert!(a.execute(Action::Establish { ue_id: 101 }).is_err());
    assert_eq!(a.snapshot().pending, 0);
    assert_eq!(
        a.snapshot()
            .events
            .iter()
            .filter(|e| e.kind == "T3550 retransmission")
            .count(),
        4
    );
    let mut r = Registration::new(101, 42, LabFault::None).unwrap();
    r.at_amf_at(&protocol::registration_request(101).unwrap(), 0)
        .unwrap();
    let retries = r.advance(30000).unwrap();
    assert_eq!(retries.len(), 5);
    assert!(retries[..4].iter().all(|r| r.wire.is_some()));
    assert!(retries[4].wire.is_none());
    assert!(r.failure.is_some());
    assert!(r.at_amf(&[0x7e, 0, 0x57, 0x2d, 16]).is_err());
}
#[test]
fn ue_t3510_t3511_and_t3502_preserve_backoff_and_recover() {
    let mut s = scenario();
    s.ues[0].signaling.drop = SignalingLoss::RegistrationRequest;
    s.ues[0].signaling.count = 5;
    let mut run = Experiment::new(s).unwrap();
    run.advance(15000).unwrap();
    assert_eq!(run.snapshot().ues[0].state, UeState::RegistrationRetry);
    run.advance(60000).unwrap();
    run.advance(40000).unwrap();
    assert_eq!(run.snapshot().time_ms, 115000);
    assert_eq!(run.snapshot().ues[0].state, UeState::RegistrationRetry);
    for _ in 0..12 {
        run.advance(60000).unwrap();
    }
    run.advance(1000).unwrap();
    assert_eq!(run.snapshot().ues[0].state, UeState::Registered);
    assert_eq!(
        run.snapshot()
            .events
            .iter()
            .filter(|e| e.kind == "T3510 expired")
            .count(),
        5
    );
    assert_eq!(
        run.snapshot()
            .events
            .iter()
            .filter(|e| e.kind == "T3502 retry")
            .count(),
        1
    );
    compare(&run, &Experiment::replay(run.recording().unwrap()).unwrap());
}
#[test]
fn delayed_session_checkpoints_no_early_forwarding_and_release_retry() {
    let mut s = scenario();
    s.ues[0].signaling.delay_ms = 250;
    s.ues[0].signaling.drop = SignalingLoss::ReleaseComplete;
    s.ues[0].signaling.count = 1;
    let mut run = Experiment::new(s).unwrap();
    run.advance(1000).unwrap();
    run.execute(Action::Establish { ue_id: 101 }).unwrap();
    assert!(run
        .execute(Action::Echo {
            ue_id: 101,
            loss_percent: 0
        })
        .is_err());
    for _ in 0..3 {
        run.advance(250).unwrap();
        compare(&run, &Experiment::replay(run.recording().unwrap()).unwrap());
    }
    assert!(run.snapshot().sessions[0].active);
    run.execute(Action::Echo {
        ue_id: 101,
        loss_percent: 0,
    })
    .unwrap();
    run.advance(100).unwrap();
    assert_eq!(run.snapshot().sessions[0].delivered, 1);
    run.execute(Action::Release { ue_id: 101 }).unwrap();
    run.advance(1000).unwrap();
    assert!(!run.snapshot().sessions[0].active);
    assert_eq!(run.snapshot().session_pending, vec![101]);
    assert!(run.execute(Action::Deregister { ue_id: 101 }).is_err());
    run.advance(16000).unwrap();
    assert!(run.snapshot().session_pending.is_empty());
    assert!(run
        .snapshot()
        .events
        .iter()
        .any(|e| e.kind == "T3592 retransmission"));
    run.execute(Action::Deregister { ue_id: 101 }).unwrap();
    compare(&run, &Experiment::replay(run.recording().unwrap()).unwrap());
}
#[test]
fn failed_and_missing_n2_never_activate_forwarding() {
    for drop in [false, true] {
        let mut s = scenario();
        s.ues[0].signaling.reject_n2 = !drop;
        if drop {
            s.ues[0].signaling.drop = SignalingLoss::N2SetupResponse;
            s.ues[0].signaling.count = 1;
        }
        let mut e = Experiment::new(s).unwrap();
        e.advance(1000).unwrap();
        e.execute(Action::Establish { ue_id: 101 }).unwrap();
        e.advance(11000).unwrap();
        assert!(!e.snapshot().sessions[0].active);
        assert!(e.snapshot().session_pending.is_empty());
        assert!(e
            .execute(Action::Echo {
                ue_id: 101,
                loss_percent: 0
            })
            .is_err());
        assert_eq!(
            e.snapshot().session_failures[&101].cause,
            if drop { 0 } else { 22 }
        );
        if !drop {
            assert!(e.snapshot().events.iter().any(|e| e.hex == "2e 01 00 d4"));
        }
        compare(&e, &Experiment::replay(e.recording().unwrap()).unwrap());
    }
}
#[test]
fn release_loss_exhaustion_and_missing_n2_cleanup_are_bounded() {
    for loss in [
        SignalingLoss::ReleaseCommand,
        SignalingLoss::ReleaseComplete,
        SignalingLoss::N2ReleaseResponse,
    ] {
        let mut s = scenario();
        s.ues[0].signaling.drop = loss;
        s.ues[0].signaling.count = 5;
        let mut e = Experiment::new(s).unwrap();
        e.advance(1000).unwrap();
        e.execute(Action::Establish { ue_id: 101 }).unwrap();
        e.advance(100).unwrap();
        e.execute(Action::Release { ue_id: 101 }).unwrap();
        e.advance(60000).unwrap();
        e.advance(21000).unwrap();
        assert!(!e.snapshot().sessions[0].active);
        assert!(e.snapshot().session_pending.is_empty());
        assert_eq!(e.snapshot().pending, 0);
        assert!(e.snapshot().session_failures.contains_key(&101));
        e.execute(Action::Establish { ue_id: 101 }).unwrap();
        e.advance(100).unwrap();
        assert!(e.snapshot().sessions[0].active);
    }
}

#[test]
fn pending_recording_detects_changed_future_signaling_configuration() {
    let mut e = Experiment::new(scenario()).unwrap();
    e.advance(1000).unwrap();
    e.execute(Action::Establish { ue_id: 101 }).unwrap();
    let mut recording = e.recording().unwrap();
    recording.scenario.ues[0].signaling.delay_ms = 1000;
    assert!(Experiment::replay(recording).is_err());
}
