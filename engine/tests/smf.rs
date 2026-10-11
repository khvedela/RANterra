use ranterra_engine::{
    model::{SessionPolicy, SessionRequest},
    session,
    smf::{Establishment, Smf},
};
fn smf() -> Smf {
    Smf::new(vec![SessionPolicy {
        request: SessionRequest::default(),
        max_sessions: 2,
    }])
    .unwrap()
}
fn establish(s: &mut Smf, id: u32, now: u64) -> session::Session {
    let n1 = session::n1(
        session::request().unwrap(),
        true,
        true,
        &SessionRequest::default(),
    )
    .unwrap();
    let Establishment::Setup { session, .. } = s.establish(id, &n1, now).unwrap() else {
        panic!("rejected")
    };
    session
}
#[test]
fn pending_requests_reserve_resources_and_duplicate_does_not_reset_deadline() {
    let mut s = smf();
    let first = establish(&mut s, 1, 0);
    let retry = establish(&mut s, 1, 9000);
    assert_eq!(first.ip, retry.ip);
    assert_eq!(first.uplink_teid, retry.uplink_teid);
    assert!(s
        .echo(
            1,
            &session::gtpu(first.uplink_teid, &session::echo_packet(first.ip, 1)).unwrap()
        )
        .is_err());
    assert!(s.advance(9999).unwrap().is_empty());
    assert_eq!(s.advance(10000).unwrap().len(), 1);
    assert!(s
        .setup_response(1, &session::setup_response(1, 3).unwrap())
        .is_err());
    assert_ne!(establish(&mut s, 1, 10000).ip, first.ip);
}
#[test]
fn rejects_tunnel_collisions_and_bad_responses_without_poisoning_valid_state() {
    let mut s = smf();
    establish(&mut s, 1, 0);
    establish(&mut s, 2, 0);
    s.setup_response(1, &session::setup_response(1, 700).unwrap())
        .unwrap();
    for b in [
        session::setup_response(2, 700).unwrap(),
        session::setup_response(1, 701).unwrap(),
        session::setup_response(2, 0).unwrap(),
    ] {
        assert!(s.setup_response(2, &b).is_err());
    }
    let mut trailing = session::setup_response(2, 701).unwrap();
    trailing.push(0);
    assert!(s.setup_response(2, &trailing).is_err());
    assert!(!s.session(2).unwrap().active);
    s.setup_response(2, &session::setup_response(2, 701).unwrap())
        .unwrap();
    assert!(s.session(2).unwrap().active);
}
#[test]
fn release_timer_is_step_invariant_and_stops_on_complete() {
    fn pending() -> Smf {
        let mut s = smf();
        establish(&mut s, 1, 0);
        s.setup_response(1, &session::setup_response(1, 3).unwrap())
            .unwrap();
        s.release(1, &[0x2e, 1, 1, 0xd1], 0).unwrap();
        s
    }
    let mut a = pending();
    let mut b = pending();
    // UE retry resends command but does not extend T3592.
    a.release(1, &[0x2e, 1, 1, 0xd1], 15999).unwrap();
    let full = a.advance(80000).unwrap();
    let mut stepped = vec![];
    for t in (0..=80000).step_by(1000) {
        stepped.extend(b.advance(t).unwrap());
    }
    assert_eq!(
        serde_json::to_value(&full).unwrap(),
        serde_json::to_value(&stepped).unwrap()
    );
    assert_eq!(full.len(), 5);
    assert_eq!(full[0].time_ms, 16000);
    assert_eq!(full[4].time_ms, 80000);
    assert!(full[..4].iter().all(|e| e.n1.is_some()));
    assert!(full[4].n1.is_none());
    assert!(!a.session(1).unwrap().active);
    assert!(a.release_complete(1, &[0x2e, 1, 1, 0xd4]).is_err());
    let mut c = pending();
    assert_eq!(c.advance(16000).unwrap().len(), 1);
    c.release_complete(1, &[0x2e, 1, 1, 0xd4]).unwrap();
    c.release_response(1, &session::release_n2(1).unwrap()[1])
        .unwrap();
    assert!(c.advance(80000).unwrap().is_empty());
    establish(&mut c, 1, 80000);
    // N1 alone stops NAS retransmission, but missing N2 must still time out locally.
    let mut d = pending();
    d.release_complete(1, &[0x2e, 1, 1, 0xd4]).unwrap();
    let events = d.advance(80000).unwrap();
    assert_eq!(events.len(), 1);
    assert!(events[0].n1.is_none());
    assert_eq!(events[0].reason, "N2 release laboratory timeout");
}
#[test]
fn malformed_selection_never_allocates_and_time_is_monotonic() {
    let mut s = smf();
    let good = session::n1(
        session::request().unwrap(),
        true,
        true,
        &SessionRequest::default(),
    )
    .unwrap();
    for i in 0..good.len() {
        assert!(s.establish(1, &good[..i], 0).is_err());
        assert!(s.session(1).is_none());
    }
    let mut wrong = good.clone();
    let pos = wrong.iter().position(|&b| b == 0x81).unwrap();
    wrong[pos] = 0x82;
    assert!(s.establish(1, &wrong, 0).is_err());
    assert_eq!(establish(&mut s, 1, 0).ip, [10, 45, 0, 1]);
    s.advance(10).unwrap();
    assert!(s.advance(9).is_err());
    assert!(s.establish(2, &good, 9).is_err());
}

#[test]
fn zero_capacity_policy_is_a_valid_deny_all_experiment() {
    let mut s = Smf::new(vec![SessionPolicy {
        request: SessionRequest::default(),
        max_sessions: 0,
    }])
    .unwrap();
    let n1 = session::n1(
        session::request().unwrap(),
        true,
        true,
        &SessionRequest::default(),
    )
    .unwrap();
    let Establishment::Reject { n1 } = s.establish(1, &n1, 0).unwrap() else {
        panic!("zero capacity accepted")
    };
    assert_eq!(
        session::unwrap_n1(&n1, false).unwrap(),
        [0x2e, 1, 1, 0xc3, 26, 0x37, 1, 0x65]
    );
    assert!(s.session(1).is_none());
    assert_eq!(s.failure(1).unwrap().retry_at_ms, Some(10000));
}
