//! Stateful collocated SMF. Inputs are authenticated, decoded N1 and encoded N2.
//! No SBI or PFCP is claimed. The caller owns registration and NAS protection.
use crate::{
    model::SessionPolicy,
    session::{self, Session},
};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize)]
pub struct SessionFailure {
    pub domain: String,
    pub cause: u8,
    pub reason: String,
    pub retry_at_ms: Option<u64>,
}
#[derive(Clone, Debug)]
enum Pending {
    Setup {
        deadline: u64,
    },
    Release {
        deadline: u64,
        retries: u8,
        pti: u8,
        nas_done: bool,
        n2_done: bool,
    },
}
pub enum Establishment {
    Setup { session: Session, n1: Vec<u8> },
    Reject { n1: Vec<u8> },
}
#[derive(Debug, Serialize)]
pub struct TimerEvent {
    pub ue_id: u32,
    pub time_ms: u64,
    pub n1: Option<Vec<u8>>,
    pub reason: String,
}
pub struct Smf {
    pub(crate) sessions: BTreeMap<u32, Session>,
    pub(crate) failures: BTreeMap<u32, SessionFailure>,
    policies: Vec<SessionPolicy>,
    pending: BTreeMap<u32, Pending>,
    next_slot: u32,
    now: u64,
}
impl Smf {
    pub fn new(policies: Vec<SessionPolicy>) -> Result<Self, String> {
        if policies.is_empty() || policies.len() > 8 {
            return Err("SMF requires 1–8 policies".into());
        }
        for (i, p) in policies.iter().enumerate() {
            p.request.validate()?;
            if p.max_sessions > 128 || policies[..i].iter().any(|q| q.request == p.request) {
                return Err("Invalid SMF policy capacity/duplicate".into());
            }
        }
        Ok(Self {
            sessions: BTreeMap::new(),
            failures: BTreeMap::new(),
            policies,
            pending: BTreeMap::new(),
            next_slot: 1,
            now: 0,
        })
    }
    pub fn allowed_slices(&self) -> Vec<crate::model::Snssai> {
        let mut slices = vec![];
        for p in &self.policies {
            if !slices.contains(&p.request.snssai) {
                slices.push(p.request.snssai.clone());
            }
        }
        slices
    }
    pub fn session(&self, id: u32) -> Option<&Session> {
        self.sessions.get(&id)
    }
    pub fn failure(&self, id: u32) -> Option<&SessionFailure> {
        self.failures.get(&id)
    }
    pub fn check_establish(&self, id: u32, now: u64) -> Result<(), String> {
        if id == 0 || now < self.now || now > 3_600_000 {
            return Err("Invalid SMF identity/time".into());
        }
        if self.pending.contains_key(&id) || self.sessions.get(&id).is_some_and(|s| s.active) {
            return Err("Session already active or pending".into());
        }
        if !self.sessions.contains_key(&id) && self.sessions.len() >= 128 {
            return Err("SMF UE context limit".into());
        }
        if let Some(f) = self.failures.get(&id) {
            if f.retry_at_ms.is_none_or(|at| now < at) {
                return Err(format!("Session retry blocked: {}", f.reason));
            }
        }
        Ok(())
    }
    pub fn establish(&mut self, id: u32, n1: &[u8], now: u64) -> Result<Establishment, String> {
        let request = session::selection(n1)?;
        let sm = session::unwrap_n1(n1, true)?;
        if sm != session::request()? {
            return Err("Unsupported SMF request".into());
        }
        // A duplicate pending request must not consume another address or restart
        // the resource deadline. Only the identical supported service may retry.
        if matches!(self.pending.get(&id), Some(Pending::Setup { .. })) {
            if now < self.now || now > 3_600_000 {
                return Err("Invalid SMF time".into());
            }
            let s = &self.sessions[&id];
            if s.dnn != request.dnn || s.sst != request.snssai.sst || s.sd != request.snssai.sd {
                return Err("Pending request service mismatch".into());
            }
            return Ok(Establishment::Setup {
                session: s.clone(),
                n1: session::n1(session::accept(s.ip, &request)?, false, false, &request)?,
            });
        }
        self.check_establish(id, now)?;
        let policy = self.policies.iter().find(|p| p.request == request);
        let count = self
            .sessions
            .values()
            .filter(|s| {
                (s.active || self.pending.contains_key(&s.ue_id))
                    && s.dnn == request.dnn
                    && s.sst == request.snssai.sst
                    && s.sd == request.snssai.sd
            })
            .count();
        let failure = if policy.is_none() {
            Some(SessionFailure {
                domain: "5GMM".into(),
                cause: 91,
                reason: "DNN not supported or not subscribed in the slice; reset to change service"
                    .into(),
                retry_at_ms: None,
            })
        } else if policy.is_some_and(|p| count >= p.max_sessions as usize) || self.next_slot > 254 {
            Some(SessionFailure {
                domain: "5GSM".into(),
                cause: 26,
                reason: "Insufficient resources; T3396 10 seconds".into(),
                retry_at_ms: Some(now + 10000),
            })
        } else {
            None
        };
        if let Some(f) = failure {
            if !self.failures.contains_key(&id) && self.failures.len() >= 128 {
                return Err("SMF failure context limit".into());
            }
            let n1 = if f.domain == "5GMM" {
                let mut b = session::n1(sm, false, false, &request)?;
                b.extend([0x58, 91]);
                session::nas(b)?
            } else {
                session::n1(
                    session::nas(vec![0x2e, 1, 1, 0xc3, 26, 0x37, 1, 0x65])?,
                    false,
                    false,
                    &request,
                )?
            };
            self.failures.insert(id, f);
            return Ok(Establishment::Reject { n1 });
        }
        let session = Session::new(id, self.next_slot, &request);
        let n1 = session::n1(
            session::accept(session.ip, &request)?,
            false,
            false,
            &request,
        )?;
        self.sessions.insert(id, session.clone());
        self.pending.insert(
            id,
            Pending::Setup {
                deadline: now + 10000,
            },
        );
        self.next_slot += 1; // Never recycle a tunnel while old packets might still exist.
        self.failures.remove(&id);
        Ok(Establishment::Setup { session, n1 })
    }
    pub fn setup_response(&mut self, id: u32, wire: &[u8]) -> Result<(), String> {
        if !matches!(self.pending.get(&id), Some(Pending::Setup { .. })) {
            return Err("No pending N2 setup".into());
        }
        let teid = session::response_teid(wire, id)?;
        if self.sessions.values().any(|s| {
            s.ue_id != id
                && (s.active || self.pending.contains_key(&s.ue_id))
                && s.downlink_teid == teid
        }) {
            return Err("Downlink TEID already assigned".into());
        }
        let s = self.sessions.get_mut(&id).ok_or("Unknown session")?;
        s.downlink_teid = teid;
        s.active = true;
        self.pending.remove(&id);
        Ok(())
    }
    pub fn next_deadline(&self) -> Option<u64> {
        self.pending
            .values()
            .map(|p| match p {
                Pending::Setup { deadline } | Pending::Release { deadline, .. } => *deadline,
            })
            .min()
    }
    pub fn is_pending(&self, id: u32) -> bool {
        self.pending.contains_key(&id)
    }
    pub fn cancel(&mut self, id: u32) {
        self.pending.remove(&id);
        if let Some(s) = self.sessions.get_mut(&id) {
            s.active = false;
        }
    }
    /// Failed N2 setup never activates a tunnel. Explicit network-requested release
    /// cleans the N1 context; PTI=0 denotes a network-initiated transaction.
    pub fn setup_failed(&mut self, id: u32, wire: &[u8], now: u64) -> Result<Vec<u8>, String> {
        if now < self.now
            || now > 3_600_000
            || !matches!(self.pending.get(&id), Some(Pending::Setup { .. }))
        {
            return Err("No pending N2 setup / invalid time".into());
        }
        session::validate_setup_failure(wire, id)?;
        self.sessions.get_mut(&id).unwrap().active = false;
        self.failures.insert(
            id,
            SessionFailure {
                domain: "NGAP radioNetwork".into(),
                cause: 22,
                reason: "Radio resources not available; releasing N1 context".into(),
                retry_at_ms: Some(now),
            },
        );
        self.pending.insert(
            id,
            Pending::Release {
                deadline: now + 16000,
                retries: 0,
                pti: 0,
                nas_done: false,
                n2_done: true,
            },
        );
        session::nas(vec![0x2e, 1, 0, 0xd3, 26])
    }
    pub fn release(&mut self, id: u32, sm: &[u8], now: u64) -> Result<Vec<u8>, String> {
        if now < self.now || now > 3_600_000 {
            return Err("Invalid SMF time".into());
        }
        if session::nas(sm.to_vec())? != [0x2e, 1, 1, 0xd1] {
            return Err("Expected release request".into());
        }
        if matches!(
            self.pending.get(&id),
            Some(Pending::Release {
                nas_done: false,
                ..
            })
        ) {
            return session::nas(vec![0x2e, 1, 1, 0xd3, 36]);
        }
        if self.pending.contains_key(&id) {
            return Err("Session procedure already pending".into());
        }
        let s = self
            .sessions
            .get_mut(&id)
            .filter(|s| s.active)
            .ok_or("No active PDU session")?;
        s.active = false; // Stop forwarding immediately; retain reservation until both endpoints release.
        self.pending.insert(
            id,
            Pending::Release {
                deadline: now + 16000,
                retries: 0,
                pti: 1,
                nas_done: false,
                n2_done: false,
            },
        );
        session::nas(vec![0x2e, 1, 1, 0xd3, 36])
    }
    pub fn release_response(&mut self, id: u32, wire: &[u8]) -> Result<(), String> {
        session::validate_release_response(wire, id)?;
        let Some(Pending::Release { n2_done, .. }) = self.pending.get_mut(&id) else {
            return Err("No pending release".into());
        };
        if *n2_done {
            return Err("Duplicate N2 release response".into());
        }
        *n2_done = true;
        self.finish_release(id);
        Ok(())
    }
    pub fn release_complete(&mut self, id: u32, sm: &[u8]) -> Result<(), String> {
        let pti = match self.pending.get(&id) {
            Some(Pending::Release { pti, .. }) => *pti,
            _ => return Err("No pending release".into()),
        };
        if session::nas(sm.to_vec())? != [0x2e, 1, pti, 0xd4] {
            return Err("Expected release complete".into());
        }
        let Some(Pending::Release { nas_done, .. }) = self.pending.get_mut(&id) else {
            return Err("No pending release".into());
        };
        if *nas_done {
            return Err("Duplicate release complete".into());
        }
        *nas_done = true;
        self.finish_release(id);
        Ok(())
    }
    fn finish_release(&mut self, id: u32) {
        if matches!(
            self.pending.get(&id),
            Some(Pending::Release {
                nas_done: true,
                n2_done: true,
                ..
            })
        ) {
            self.pending.remove(&id);
        }
    }
    pub fn echo(&self, id: u32, wire: &[u8]) -> Result<Vec<u8>, String> {
        let s = self
            .sessions
            .get(&id)
            .filter(|s| s.active)
            .ok_or("No active PDU session")?;
        let ip = session::decode_gtpu(wire, s.uplink_teid)?;
        session::gtpu(s.downlink_teid, &session::echo_reply(ip, s.ip)?)
    }
    /// T3592: four retransmissions at 16 s, abort on fifth expiry. N2 setup/cleanup
    /// use explicit laboratory deadlines; no NGAP standardized timer is claimed.
    pub fn advance(&mut self, now: u64) -> Result<Vec<TimerEvent>, String> {
        if now < self.now || now > 3_600_000 {
            return Err("Invalid SMF time".into());
        }
        let mut events = vec![];
        loop {
            let next = self
                .pending
                .iter()
                .map(|(&id, p)| {
                    (
                        match p {
                            Pending::Setup { deadline } | Pending::Release { deadline, .. } => {
                                *deadline
                            }
                        },
                        id,
                    )
                })
                .filter(|(t, _)| *t <= now)
                .min();
            let Some((at, id)) = next else { break };
            let p = self.pending.get_mut(&id).unwrap();
            if let Pending::Release {
                deadline,
                retries,
                pti,
                nas_done: false,
                ..
            } = p
            {
                if *retries < 4 {
                    *retries += 1;
                    *deadline += 16000;
                    let s = &self.sessions[&id];
                    let request = crate::model::SessionRequest {
                        dnn: s.dnn.clone(),
                        snssai: crate::model::Snssai {
                            sst: s.sst,
                            sd: s.sd,
                        },
                    };
                    events.push(TimerEvent {
                        ue_id: id,
                        time_ms: at,
                        n1: Some(session::n1(
                            session::nas(vec![
                                0x2e,
                                1,
                                *pti,
                                0xd3,
                                if *pti == 0 { 26 } else { 36 },
                            ])?,
                            false,
                            false,
                            &request,
                        )?),
                        reason: "T3592 retransmission".into(),
                    });
                    continue;
                }
            }
            let reason = match p {
                Pending::Setup { .. } => "N2 setup laboratory timeout",
                Pending::Release { nas_done: true, .. } => "N2 release laboratory timeout",
                _ => "T3592 exhausted",
            };
            self.sessions.get_mut(&id).unwrap().active = false;
            self.failures.insert(
                id,
                SessionFailure {
                    domain: "LOCAL".into(),
                    cause: 0,
                    reason: reason.into(),
                    retry_at_ms: Some(at),
                },
            );
            events.push(TimerEvent {
                ue_id: id,
                time_ms: at,
                n1: None,
                reason: reason.into(),
            });
            self.pending.remove(&id);
        }
        self.now = now;
        Ok(events)
    }
}
