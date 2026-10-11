use ranterra_engine::{
    experiment::{Action, Experiment},
    model::{Scenario, SessionRequest, Snssai},
    simulation::UeState,
};
fn scenario() -> Scenario {
    serde_json::from_str(include_str!("../../examples/minimal.json")).unwrap()
}
fn json(e: &Experiment) -> String {
    serde_json::to_string(&e.snapshot()).unwrap()
}
#[test]
fn checkpoint_restores_inflight_packets_counts_and_continuation() {
    let mut e = Experiment::new(scenario()).unwrap();
    let id = e.configuration().ues[0].id;
    e.advance(1000).unwrap();
    e.execute(Action::Establish { ue_id: id }).unwrap();
    e.execute(Action::Echo {
        ue_id: id,
        loss_percent: 0,
    })
    .unwrap();
    let recording = serde_json::to_string(&e.recording().unwrap()).unwrap();
    for forbidden in ["kamf", "xres", "opc", "ue_security", "amf_security"] {
        assert!(!recording.contains(forbidden));
    }
    let mut restored = Experiment::from_json(&recording).unwrap();
    assert_eq!(json(&e), json(&restored));
    for run in [&mut e, &mut restored] {
        run.advance(100).unwrap();
        run.execute(Action::Echo {
            ue_id: id,
            loss_percent: 100,
        })
        .unwrap();
        run.execute(Action::Release { ue_id: id }).unwrap();
        run.execute(Action::Establish { ue_id: id }).unwrap();
        run.execute(Action::Echo {
            ue_id: id,
            loss_percent: 0,
        })
        .unwrap();
        run.advance(100).unwrap();
        run.execute(Action::Deregister { ue_id: id }).unwrap();
    }
    assert_eq!(json(&e), json(&restored));
    assert_eq!(
        json(&e),
        json(&Experiment::replay(e.recording().unwrap()).unwrap())
    );
    assert_eq!(e.snapshot().ues[0].state, UeState::Deregistered);
}
#[test]
fn migration_bounds_order_digest_and_failed_command_are_transactional() {
    let mut e = Experiment::new(scenario()).unwrap();
    assert_eq!(e.configuration().version, 2);
    assert_eq!(e.configuration().ues[0].session, SessionRequest::default());
    assert_eq!(
        json(&e),
        json(&Experiment::replay(e.recording().unwrap()).unwrap())
    );
    let before = json(&e);
    assert!(e.execute(Action::Establish { ue_id: 101 }).is_err());
    assert_eq!(json(&e), before);
    e.advance(1000).unwrap();
    e.execute(Action::Establish { ue_id: 101 }).unwrap();
    e.advance(20).unwrap();
    e.execute(Action::Echo {
        ue_id: 101,
        loss_percent: 0,
    })
    .unwrap();
    let r = e.recording().unwrap();
    let mut bad = r.clone();
    bad.commands.swap(0, 1);
    assert!(Experiment::replay(bad).is_err());
    let mut bad = r.clone();
    bad.state_digest = "0".repeat(64);
    assert!(Experiment::replay(bad).is_err());
    let mut bad = r.clone();
    bad.time_ms = 3_600_001;
    assert!(Experiment::replay(bad).is_err());
    let mut bad = r.clone();
    bad.engine_profile = "future".into();
    assert!(Experiment::replay(bad).is_err());
    let before = json(&e);
    assert!(e
        .execute(Action::Echo {
            ue_id: 101,
            loss_percent: 101
        })
        .is_err());
    assert_eq!(before, json(&e));
    assert!(Experiment::from_json(&" ".repeat(4_000_001)).is_err());
}
#[test]
fn policy_selection_capacity_reject_backoff_and_retry() {
    let mut s = scenario();
    let request = SessionRequest {
        dnn: "research.example".into(),
        snssai: Snssai {
            sst: 2,
            sd: Some([1, 2, 3]),
        },
    };
    s.session_policies[0].request = request.clone();
    s.session_policies[0].max_sessions = 1;
    s.ues[0].session = request;
    let mut other = s.ues[0].clone();
    other.id = 102;
    s.ues.push(other);
    let mut e = Experiment::new(s).unwrap();
    e.advance(1000).unwrap();
    e.execute(Action::Establish { ue_id: 101 }).unwrap();
    assert_eq!(e.snapshot().sessions[0].dnn, "research.example");
    assert_eq!(e.snapshot().sessions[0].sd, Some([1, 2, 3]));
    e.execute(Action::Establish { ue_id: 102 }).unwrap();
    assert_eq!(e.snapshot().session_failures[&102].cause, 26);
    assert!(e
        .snapshot()
        .events
        .iter()
        .any(|e| e.hex == "2e 01 01 c3 1a 37 01 65"));
    assert!(e.execute(Action::Establish { ue_id: 102 }).is_err());
    e.execute(Action::Release { ue_id: 101 }).unwrap();
    e.advance(10000).unwrap();
    e.execute(Action::Establish { ue_id: 102 }).unwrap();
    assert_eq!(e.snapshot().sessions[1].ip, [10, 45, 0, 2]);
    assert!(e.snapshot().session_failures.is_empty());
    assert_eq!(
        json(&e),
        json(&Experiment::replay(e.recording().unwrap()).unwrap())
    );
}
#[test]
fn unsupported_service_returns_protected_unforwarded_n1_without_allocation() {
    let mut s = scenario();
    s.ues[0].session.dnn = "unknown".into();
    let mut e = Experiment::new(s).unwrap();
    e.advance(1000).unwrap();
    e.execute(Action::Establish { ue_id: 101 }).unwrap();
    let snap = e.snapshot();
    assert!(snap.sessions.is_empty());
    assert_eq!(snap.session_failures[&101].cause, 91);
    assert_eq!(snap.ues[0].state, UeState::Registered);
    assert!(!snap
        .events
        .iter()
        .any(|e| e.kind == "PDUSessionResourceSetupRequest"));
    assert_eq!(
        json(&e),
        json(&Experiment::replay(e.recording().unwrap()).unwrap())
    );
}
#[test]
fn timeout_with_mobility_is_independent_of_step_boundaries() {
    let mut s = scenario();
    s.ues[0].fault = ranterra_engine::model::LabFault::Timeout;
    s.ues[0].mobility.speed_mps = 1.0;
    s.ues[0].mobility.waypoints = vec![[s.ues[0].lon + 0.001, s.ues[0].lat]];
    let mut a = Experiment::new(s.clone()).unwrap();
    let mut b = Experiment::new(s).unwrap();
    a.advance(10000).unwrap();
    for _ in 0..1000 {
        b.advance(10).unwrap();
    }
    assert_eq!(json(&a), json(&b));
    assert_eq!(a.snapshot().ues[0].state, UeState::RegistrationRejected);
    assert!(a
        .snapshot()
        .events
        .iter()
        .any(|e| e.time_ms == 6000 && e.detail.contains("timeout")));
}
#[test]
fn policy_schema_rejects_duplicates_bad_dns_and_reserved_sd() {
    for dnn in ["A", "a..b", "-bad", "bad-", "a_b"] {
        let mut s = scenario();
        s.session_policies[0].request.dnn = dnn.into();
        assert!(s.validate().is_err());
    }
    let mut s = scenario();
    s.session_policies.push(s.session_policies[0].clone());
    assert!(s.validate().is_err());
    let mut s = scenario();
    s.ues[0].session.snssai.sd = Some([255; 3]);
    assert!(s.validate().is_err());
}

#[test]
fn registration_checkpoints_before_and_after_every_protocol_stage() {
    for t in [0, 1, 15, 40, 100, 180, 240, 350, 1000] {
        let mut run = Experiment::new(scenario()).unwrap();
        run.advance(t).unwrap();
        let mut restored = Experiment::replay(run.recording().unwrap()).unwrap();
        assert_eq!(json(&run), json(&restored));
        run.advance(1000).unwrap();
        restored.advance(1000).unwrap();
        assert_eq!(json(&run), json(&restored));
    }
}
#[test]
fn mobile_reassociation_and_loss_recording_restores_identically() {
    let mut s = scenario();
    s.ues[0].mobility.speed_mps = 50.0;
    s.ues[0].mobility.waypoints = vec![[s.ues[0].lon + 0.1, s.ues[0].lat]];
    s.radio.coverage_dbm = -65.0;
    let mut run = Experiment::new(s).unwrap();
    run.advance(1000).unwrap();
    run.advance(9000).unwrap();
    assert_eq!(
        json(&run),
        json(&Experiment::replay(run.recording().unwrap()).unwrap())
    );
}
#[test]
fn checked_in_experiment_is_replayable() {
    let e = Experiment::from_json(include_str!(
        "../../examples/session-policy-experiment.json"
    ))
    .unwrap();
    assert_eq!(e.snapshot().time_ms, 11100);
    let session = e
        .snapshot()
        .sessions
        .into_iter()
        .find(|s| s.ue_id == 102)
        .unwrap();
    assert!(session.active);
    assert_eq!(session.delivered, 1);
    assert_eq!(session.dropped, 1);
}
