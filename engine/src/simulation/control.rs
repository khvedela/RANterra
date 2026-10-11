//! Delayed, encoded session signaling for execution profile 3. Each scheduled
//! boundary consumes the wire produced by its peer, never a synthetic success.
use super::*;
use crate::{session, smf::Establishment};
#[derive(Debug)]
pub(super) enum Control {
    Request(Vec<u8>),
    Setup(Vec<u8>),
    SetupResponse(Vec<u8>, bool),
    Downlink(Vec<u8>),
    ReleaseRequest(Vec<u8>),
    ReleaseN2(Vec<u8>),
    ReleaseN2Response(Vec<u8>),
    Complete(Vec<u8>),
}
impl<T: Transport> Engine<T> {
    fn service(&self, id: u32) -> SessionRequest {
        self.scenario
            .ues
            .iter()
            .find(|u| u.id == id)
            .unwrap()
            .session
            .clone()
    }
    fn queue_control(&mut self, id: u32, event: Control) {
        let delay = self
            .scenario
            .ues
            .iter()
            .find(|u| u.id == id)
            .unwrap()
            .signaling
            .delay_ms;
        self.control_queue
            .insert((self.time_ms + delay, self.control_sequence), (id, event));
        self.control_sequence += 1;
    }
    pub(super) fn drop_signaling(&mut self, id: u32, loss: SignalingLoss, bytes: &[u8]) -> bool {
        if self.legacy || loss == SignalingLoss::None {
            return false;
        }
        let config = &self
            .scenario
            .ues
            .iter()
            .find(|u| u.id == id)
            .unwrap()
            .signaling;
        let used = self.loss_used.entry(id).or_default();
        if config.drop != loss || *used >= config.count {
            return false;
        }
        *used += 1;
        self.log(
            id,
            "Virtual transport".into(),
            "Discarded".into(),
            "Signaling loss".into(),
            format!("Deterministic laboratory loss: {loss:?}"),
            bytes,
            "STATE",
        );
        true
    }
    fn seal_n1(
        &mut self,
        id: u32,
        sm: Vec<u8>,
        uplink: bool,
        initial: bool,
    ) -> Result<Vec<u8>, String> {
        let n1 = session::n1(sm, uplink, initial, &self.service(id))?;
        self.registrations
            .get_mut(&id)
            .unwrap()
            .protect_session(n1, uplink)
    }
    fn receive_n1(&mut self, id: u32, wire: &[u8], uplink: bool) -> Result<Vec<u8>, String> {
        let cell = self.ues[&id].serving_cell.ok_or("No serving cell")?;
        let n2 = if uplink {
            crate::ngap::uplink(wire.to_vec(), id, cell)?
        } else {
            crate::ngap::downlink(wire.to_vec(), id)?
        };
        let (kind, bytes) = crate::ngap::unwrap(&n2, id)?;
        self.log(
            id,
            "gNodeB / AMF".into(),
            "AMF / gNodeB".into(),
            kind,
            "N1 SM transport".into(),
            &n2,
            "NGAP",
        );
        let plain = self
            .registrations
            .get_mut(&id)
            .unwrap()
            .receive_session(&bytes, uplink)?;
        let sm = session::unwrap_n1(&plain, uplink)?;
        self.log(
            id,
            if uplink {
                format!("UE-{id}")
            } else {
                "SMF via AMF".into()
            },
            if uplink {
                "SMF via AMF".into()
            } else {
                format!("UE-{id}")
            },
            if uplink {
                "UL NAS Transport".into()
            } else {
                "DL NAS Transport".into()
            },
            oxirush_nas::nas_5gs::decode_nas_5gs_message(&plain)
                .map_err(|e| e.to_string())?
                .to_string(),
            wire,
            "NAS",
        );
        self.log(
            id,
            "N1 sender".into(),
            "N1 receiver".into(),
            match sm.get(3) {
                Some(0xc1) => "PDU Session Establishment Request",
                Some(0xc2) => "PDU Session Establishment Accept",
                Some(0xc3) => "PDU Session Establishment Reject",
                Some(0xd1) => "PDU Session Release Request",
                Some(0xd3) => "PDU Session Release Command",
                Some(0xd4) => "PDU Session Release Complete",
                _ => "5GSM",
            }
            .into(),
            oxirush_nas::nas_5gs::decode_nas_5gs_message(&sm)
                .map_err(|e| e.to_string())?
                .to_string(),
            &sm,
            "5GSM",
        );
        Ok(plain)
    }
    pub(super) fn begin_setup(&mut self, id: u32) -> Result<(), String> {
        if self.ues.get(&id).map(|u| &u.state) != Some(&UeState::Registered) {
            return Err("Register the UE first".into());
        }
        if self.control_pending.contains(&id) {
            return Err("Session signaling is pending".into());
        }
        self.smf.check_establish(id, self.time_ms)?;
        let wire = self.seal_n1(id, session::request()?, true, true)?;
        self.control_pending.insert(id);
        self.queue_control(id, Control::Request(wire));
        Ok(())
    }
    pub(super) fn begin_release(&mut self, id: u32) -> Result<(), String> {
        if self.control_pending.contains(&id) || !self.smf.session(id).is_some_and(|s| s.active) {
            return Err("No active PDU session / signaling pending".into());
        }
        let wire = self.seal_n1(id, vec![0x2e, 1, 1, 0xd1], true, false)?;
        self.control_pending.insert(id);
        self.queue_control(id, Control::ReleaseRequest(wire));
        Ok(())
    }
    pub(super) fn cancel_control(&mut self, id: u32) {
        self.smf.cancel(id);
        self.control_pending.remove(&id);
        self.control_queue.retain(|_, (ue, _)| *ue != id);
    }
    fn finish_control(&mut self, id: u32) {
        if !self.smf.is_pending(id) {
            self.control_pending.remove(&id);
        }
    }
    fn send_release(&mut self, id: u32, sm: Vec<u8>) -> Result<(), String> {
        let wire = self.seal_n1(id, sm, false, false)?;
        if !self.drop_signaling(id, SignalingLoss::ReleaseCommand, &wire) {
            self.queue_control(id, Control::Downlink(wire));
        }
        Ok(())
    }
    fn consume_control(&mut self, id: u32, event: Control) -> Result<(), String> {
        match event {
            Control::Request(wire) => {
                let n1 = self.receive_n1(id, &wire, true)?;
                match self.smf.establish(id, &n1, self.time_ms)? {
                    Establishment::Reject { n1 } => {
                        let wire = self
                            .registrations
                            .get_mut(&id)
                            .unwrap()
                            .protect_session(n1, false)?;
                        self.queue_control(id, Control::Downlink(wire));
                    }
                    Establishment::Setup { session: s, n1 } => {
                        let wire = self
                            .registrations
                            .get_mut(&id)
                            .unwrap()
                            .protect_session(n1, false)?;
                        let n2 = session::setup(id, s.uplink_teid, wire, &self.service(id).snssai)?;
                        self.queue_control(id, Control::Setup(n2));
                    }
                }
            }
            Control::Setup(n2) => {
                let s = self.smf.session(id).ok_or("Missing allocation")?.clone();
                let wire = session::decode_setup(&n2, id, s.uplink_teid, &self.service(id).snssai)?;
                self.log(
                    id,
                    "AMF / SMF".into(),
                    "gNodeB".into(),
                    "PDUSessionResourceSetupRequest".into(),
                    "Session 1; pending resources".into(),
                    &n2,
                    "NGAP",
                );
                let failed = self
                    .scenario
                    .ues
                    .iter()
                    .find(|u| u.id == id)
                    .unwrap()
                    .signaling
                    .reject_n2;
                let response = if failed {
                    session::setup_failure(id)?
                } else {
                    // N1 Accept is delivered only if the gNodeB can establish resources.
                    let plain = self.receive_n1(id, &wire, false)?;
                    if session::unwrap_n1(&plain, false)?
                        != session::accept(s.ip, &self.service(id))?
                    {
                        return Err("UE accept mismatch".into());
                    }
                    session::setup_response(id, s.downlink_teid)?
                };
                if !self.drop_signaling(id, SignalingLoss::N2SetupResponse, &response) {
                    self.queue_control(id, Control::SetupResponse(response, failed));
                }
            }
            Control::SetupResponse(wire, failed) => {
                if failed {
                    let command = self.smf.setup_failed(id, &wire, self.time_ms)?;
                    self.send_release(id, command)?;
                } else {
                    self.smf.setup_response(id, &wire)?;
                    self.finish_control(id);
                }
                self.log(
                    id,
                    "gNodeB".into(),
                    "AMF / SMF".into(),
                    "PDUSessionResourceSetupResponse".into(),
                    if failed {
                        "Failed session 1: radio resources not available".into()
                    } else {
                        "Session 1 active".into()
                    },
                    &wire,
                    "NGAP",
                );
            }
            Control::Downlink(wire) => {
                let plain = self.receive_n1(id, &wire, false)?;
                let sm = session::unwrap_n1(&plain, false)?;
                if sm.get(3) == Some(&0xd3) {
                    let complete = self.seal_n1(id, vec![0x2e, 1, sm[2], 0xd4], true, false)?;
                    if !self.drop_signaling(id, SignalingLoss::ReleaseComplete, &complete) {
                        self.queue_control(id, Control::Complete(complete));
                    }
                } else {
                    self.finish_control(id);
                }
            }
            Control::ReleaseRequest(wire) => {
                let plain = self.receive_n1(id, &wire, true)?;
                let command =
                    self.smf
                        .release(id, &session::unwrap_n1(&plain, true)?, self.time_ms)?;
                let pending = self.inflight.keys().filter(|(ue, _)| *ue == id).count();
                self.smf.sessions.get_mut(&id).unwrap().dropped += pending as u64;
                self.inflight.retain(|(ue, _), _| *ue != id);
                self.send_release(id, command)?;
                self.queue_control(id, Control::ReleaseN2(session::release_n2(id)?[0].clone()));
            }
            Control::ReleaseN2(wire) => {
                if wire != session::release_n2(id)?[0] {
                    return Err("Invalid N2 release command".into());
                }
                self.log(
                    id,
                    "AMF".into(),
                    "gNodeB".into(),
                    "PDUSessionResourceReleaseCommand".into(),
                    "Session 1".into(),
                    &wire,
                    "NGAP",
                );
                let response = session::release_n2(id)?[1].clone();
                if !self.drop_signaling(id, SignalingLoss::N2ReleaseResponse, &response) {
                    self.queue_control(id, Control::ReleaseN2Response(response));
                }
            }
            Control::ReleaseN2Response(wire) => {
                self.smf.release_response(id, &wire)?;
                self.log(
                    id,
                    "gNodeB".into(),
                    "AMF".into(),
                    "PDUSessionResourceReleaseResponse".into(),
                    "Session 1".into(),
                    &wire,
                    "NGAP",
                );
                self.finish_control(id);
            }
            Control::Complete(wire) => {
                let n1 = self.receive_n1(id, &wire, true)?;
                self.smf
                    .release_complete(id, &session::unwrap_n1(&n1, true)?)?;
                self.finish_control(id);
            }
        }
        Ok(())
    }
    pub(super) fn advance_control(&mut self) -> Result<(), String> {
        while self
            .control_queue
            .first_key_value()
            .is_some_and(|(k, _)| k.0 <= self.time_ms)
        {
            let (_, (id, event)) = self.control_queue.pop_first().unwrap();
            if self.ues[&id].state != UeState::Registered {
                self.cancel_control(id);
                continue;
            }
            if let Err(reason) = self.consume_control(id, event) {
                self.cancel_control(id);
                self.smf.failures.insert(
                    id,
                    SessionFailure {
                        domain: "LOCAL".into(),
                        cause: 0,
                        reason: reason.clone(),
                        retry_at_ms: Some(self.time_ms),
                    },
                );
                self.log(
                    id,
                    "Session control".into(),
                    "Discarded".into(),
                    "Session signaling failed".into(),
                    reason,
                    &[],
                    "STATE",
                );
            }
        }
        for event in self.smf.advance(self.time_ms)? {
            self.log(
                event.ue_id,
                "SMF".into(),
                "Timer".into(),
                event.reason.clone(),
                "Virtual timer expiry".into(),
                &[],
                "STATE",
            );
            if let Some(n1) = event.n1 {
                let wire = self
                    .registrations
                    .get_mut(&event.ue_id)
                    .unwrap()
                    .protect_session(n1, false)?;
                if !self.drop_signaling(event.ue_id, SignalingLoss::ReleaseCommand, &wire) {
                    self.queue_control(event.ue_id, Control::Downlink(wire));
                }
            } else {
                self.cancel_control(event.ue_id);
            }
        }
        let ids: Vec<_> = self.registrations.keys().copied().collect();
        for id in ids {
            if let Some(event) = self
                .registrations
                .get_mut(&id)
                .unwrap()
                .advance_ue(self.time_ms)?
            {
                self.log(id,format!("UE-{id}"),"Timer".into(),format!("{} {}",event.name,if event.wire.is_some(){"retry"}else{"expired"}),"Normal initial registration timer; virtual N1 connection released before retry".into(),&[],"STATE");
                if let Some(wire) = event.wire {
                    self.ues.get_mut(&id).unwrap().state = UeState::RegistrationInitiated;
                    self.ues.get_mut(&id).unwrap().failure = None;
                    let cell = self.ues[&id].serving_cell.ok_or("No serving cell")?;
                    if !self.drop_signaling(id, SignalingLoss::RegistrationRequest, &wire) {
                        self.send(Frame {
                            from: Endpoint::Ue(id),
                            to: Endpoint::Gnodeb(cell),
                            ue_id: id,
                            cell_id: cell,
                            bytes: wire,
                            ngap: false,
                        });
                    }
                } else {
                    self.ues.get_mut(&id).unwrap().state = UeState::RegistrationRetry;
                    self.ues.get_mut(&id).unwrap().failure =
                        Some("T3510 expired; waiting for T3511/T3502 retry".into());
                    self.amf.remove(&id);
                }
            }
            for event in self
                .registrations
                .get_mut(&id)
                .unwrap()
                .advance(self.time_ms)?
            {
                self.log(
                    id,
                    "AMF".into(),
                    "Timer".into(),
                    format!(
                        "{} {}",
                        event.name,
                        if event.wire.is_some() {
                            "retransmission"
                        } else {
                            "exhausted"
                        }
                    ),
                    "Virtual timer expiry".into(),
                    &[],
                    "STATE",
                );
                if let Some(wire) = event.wire {
                    let loss = if event.name == "T3550" {
                        SignalingLoss::RegistrationAccept
                    } else if wire.get(1) == Some(&0) {
                        SignalingLoss::AuthenticationRequest
                    } else {
                        SignalingLoss::SecurityModeCommand
                    };
                    if !self.drop_signaling(id, loss, &wire) {
                        let cell = self.ues[&id].serving_cell.ok_or("No serving cell")?;
                        self.send(Frame {
                            from: Endpoint::Amf,
                            to: Endpoint::Gnodeb(cell),
                            ue_id: id,
                            cell_id: cell,
                            bytes: crate::ngap::downlink(wire, id)?,
                            ngap: true,
                        });
                    }
                } else if event.unconfirmed {
                    self.ues.get_mut(&id).unwrap().state = UeState::RegistrationUnconfirmed;
                    self.ues.get_mut(&id).unwrap().failure=Some("T3550 exhausted: AMF registered, UE completion unconfirmed; reset required".into());
                    self.amf.insert(id, AmfState::Registered);
                } else {
                    self.fail_ue(id, "T3560 exhausted; registration aborted".into());
                    for (i, wire) in crate::ngap::release_context(id)?.iter().enumerate() {
                        self.log(
                            id,
                            "AMF / gNodeB".into(),
                            "gNodeB / AMF".into(),
                            if i == 0 {
                                "UEContextReleaseCommand".into()
                            } else {
                                "UEContextReleaseComplete".into()
                            },
                            "Registration timer exhausted; local N1 release".into(),
                            wire,
                            "NGAP",
                        );
                    }
                }
            }
        }
        Ok(())
    }
}
