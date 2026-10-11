//! A bounded initial-registration profile over genuine NAS PDUs.
//! UE and AMF own separate security contexts; neither may read the other's keys.
use crate::{auth, model::LabFault, protocol};
use oxirush_nas::nas_5gs::ie::{CipheringAlgorithm, IntegrityAlgorithm};
use oxirush_nas::nas_5gs::{
    security::{Direction, NasSecurityContext},
    *,
};
use std::result::Result;
use subtle::ConstantTimeEq;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Initial,
    Authentication,
    Security,
    Accept,
    Registered,
    Failed,
}
pub struct Registration {
    ue_deadline: Option<(u64, bool)>,
    attempts: u8,
    now: u64,
    timer: Option<RetryTimer>,
    accept_cache: Option<Vec<u8>>,
    pub unconfirmed: bool,
    id: u32,
    seed: u32,
    fault: LabFault,
    ue_stage: Stage,
    amf_stage: Stage,
    vector: Option<auth::Vector>,
    ue_security: Option<NasSecurityContext>,
    amf_security: Option<NasSecurityContext>,
    sqn: u64,
    issued_sqn: u64,
    resyncs: u8,
    auth_cache: Option<(Vec<u8>, zeroize::Zeroizing<Vec<u8>>)>,
    request: Vec<u8>,
    allowed_slices: Vec<crate::model::Snssai>,
    pub failure: Option<String>,
}
struct RetryTimer {
    deadline: u64,
    retries: u8,
    plain: Vec<u8>,
    sht: u8,
}
#[derive(Debug)]
pub struct TimerEvent {
    pub time_ms: u64,
    pub wire: Option<Vec<u8>>,
    pub name: &'static str,
    pub unconfirmed: bool,
}
pub struct Received {
    pub plain: Vec<u8>,
    pub reply: Option<Vec<u8>>,
    pub registered: bool,
}
fn checked(bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    if bytes.len() > 65536 {
        return Err("NAS PDU exceeds limit".into());
    }
    let m = decode_nas_5gs_message(&bytes).map_err(|e| e.to_string())?;
    if !m.validate().is_empty() {
        return Err(format!("Invalid NAS: {:?}", m.validate()));
    }
    encode_nas_5gs_message(&m).map_err(|e| e.to_string())
}
fn context(kamf: &[u8; 32]) -> NasSecurityContext {
    NasSecurityContext::from_fresh_kamf(kamf, IntegrityAlgorithm::NIA2, CipheringAlgorithm::NEA2)
}
fn protect(
    ctx: &mut Option<NasSecurityContext>,
    bytes: Vec<u8>,
    sht: u8,
    direction: Direction,
) -> Result<Vec<u8>, String> {
    let bytes = checked(bytes)?;
    ctx.as_mut()
        .ok_or("Missing NAS security context")?
        .protect_bytes(
            bytes,
            match sht {
                3 => Nas5gsSecurityHeaderType::IntegrityProtectedWithNewContext,
                4 => Nas5gsSecurityHeaderType::IntegrityProtectedAndCipheredWithNewContext,
                _ => Nas5gsSecurityHeaderType::IntegrityProtectedAndCiphered,
            },
            direction,
        )
        .map_err(|e| e.to_string())
}
fn unprotect(
    ctx: &mut Option<NasSecurityContext>,
    bytes: &[u8],
    sht: u8,
    direction: Direction,
) -> Result<Vec<u8>, String> {
    if bytes.len() > 65536 {
        return Err("NAS PDU exceeds limit".into());
    }
    if bytes.get(1) != Some(&sht) {
        return Err("Unexpected NAS security header / downgrade attempt".into());
    }
    let (m, _) = ctx
        .as_mut()
        .ok_or("Missing NAS security context")?
        .unprotect(bytes, direction)
        .map_err(|e| e.to_string())?;
    encode_nas_5gs_message(&m).map_err(|e| e.to_string())
}
impl Registration {
    pub fn new(id: u32, seed: u32, fault: LabFault) -> Result<Self, String> {
        Ok(Self {
            ue_deadline: None,
            attempts: 0,
            now: 0,
            timer: None,
            accept_cache: None,
            unconfirmed: false,
            id,
            seed,
            fault,
            ue_stage: Stage::Initial,
            amf_stage: Stage::Initial,
            vector: None,
            ue_security: None,
            amf_security: None,
            sqn: if fault == LabFault::SqnOutOfSync {
                100
            } else {
                0
            },
            issued_sqn: 1,
            resyncs: 0,
            auth_cache: None,
            allowed_slices: vec![crate::model::Snssai::default()],
            request: protocol::registration_request(id)?,
            failure: None,
        })
    }
    /// Synthetic USIM provisioning for resynchronization experiments; never a key import.
    pub fn with_usim_sqn(mut self, sqn: u64) -> Result<Self, String> {
        if self.ue_stage != Stage::Initial
            || self.amf_stage != Stage::Initial
            || sqn > auth::MAX_SQN
        {
            return Err("Invalid 48-bit USIM SQN".into());
        }
        self.sqn = sqn;
        Ok(self)
    }
    fn authentication_request(&mut self) -> Result<Vec<u8>, String> {
        let mut v = auth::vector(self.seed, self.id, self.issued_sqn);
        if self.fault == LabFault::InvalidAutn {
            v.autn[15] ^= 1;
        }
        let mut msg = vec![0x7e, 0, 0x56, 0, 2, 0, 0, 0x21];
        msg.extend(v.rand);
        msg.extend([0x20, 16]);
        msg.extend(v.autn);
        let msg = checked(msg)?;
        self.vector = Some(v);
        self.amf_stage = Stage::Authentication;
        Ok(msg)
    }
    pub fn with_slices(mut self, slices: Vec<crate::model::Snssai>) -> Self {
        self.allowed_slices = slices;
        self
    }
    pub fn fail(&mut self, reason: String) {
        self.failure = Some(reason);
        self.ue_stage = Stage::Failed;
        self.amf_stage = Stage::Failed;
        self.ue_deadline = None;
        self.timer = None;
        self.accept_cache = None;
        self.vector = None;
        self.auth_cache = None;
        self.ue_security = None;
        self.amf_security = None;
    }
    pub fn clear(&mut self) {
        self.ue_deadline = None;
        self.timer = None;
        self.accept_cache = None;
        self.vector = None;
        self.auth_cache = None;
        self.ue_security = None;
        self.amf_security = None;
        self.ue_stage = Stage::Failed;
        self.amf_stage = Stage::Failed;
    }
    pub fn protect_session(&mut self, plain: Vec<u8>, uplink: bool) -> Result<Vec<u8>, String> {
        if (if uplink {
            self.ue_stage
        } else {
            self.amf_stage
        }) != Stage::Registered
        {
            return Err("UE is not registered".into());
        }
        protect(
            if uplink {
                &mut self.ue_security
            } else {
                &mut self.amf_security
            },
            plain,
            2,
            if uplink {
                Direction::Uplink
            } else {
                Direction::Downlink
            },
        )
    }
    pub fn receive_session(&mut self, wire: &[u8], uplink: bool) -> Result<Vec<u8>, String> {
        if (if uplink {
            self.amf_stage
        } else {
            self.ue_stage
        }) != Stage::Registered
        {
            return Err("UE is not registered".into());
        }
        unprotect(
            if uplink {
                &mut self.amf_security
            } else {
                &mut self.ue_security
            },
            wire,
            2,
            if uplink {
                Direction::Uplink
            } else {
                Direction::Downlink
            },
        )
    }
    pub fn at_amf_at(&mut self, wire: &[u8], now: u64) -> Result<Received, String> {
        if now < self.now || now > 3_600_000 {
            return Err("Invalid registration clock".into());
        }
        self.now = now;
        self.at_amf(wire)
    }
    pub fn next_deadline(&self) -> Option<u64> {
        self.timer
            .as_ref()
            .map(|t| t.deadline)
            .into_iter()
            .chain(self.ue_deadline.map(|(at, _)| at))
            .min()
    }
    pub fn start_ue(&mut self, now: u64) -> Result<Vec<u8>, String> {
        if now < self.now || now > 3_600_000 || self.ue_stage != Stage::Initial {
            return Err("Invalid UE registration start".into());
        }
        self.ue_deadline = Some((now + 15000, false));
        Ok(self.request.clone())
    }
    /// Normal initial registration: T3510 -> T3511, fifth failed attempt -> T3502.
    /// The local virtual N1 connection is released before retry. Persistent USIM
    /// SQN is retained, and a new network vector is issued on the next attempt.
    pub fn advance_ue(&mut self, now: u64) -> Result<Option<TimerEvent>, String> {
        if now < self.now || now > 3_600_000 {
            return Err("Invalid UE clock".into());
        }
        let Some((at, waiting)) = self.ue_deadline.filter(|(at, _)| *at <= now) else {
            return Ok(None);
        };
        if waiting {
            let name = if self.attempts == 5 { "T3502" } else { "T3511" };
            if self.attempts == 5 {
                self.attempts = 0;
            }
            self.ue_stage = Stage::Initial;
            self.amf_stage = Stage::Initial;
            self.failure = None;
            self.resyncs = 0;
            self.issued_sqn = self
                .issued_sqn
                .checked_add(1)
                .filter(|n| *n <= auth::MAX_SQN)
                .ok_or("SQN exhausted")?;
            self.ue_deadline = Some((at + 15000, false));
            Ok(Some(TimerEvent {
                time_ms: at,
                wire: Some(self.request.clone()),
                name,
                unconfirmed: false,
            }))
        } else {
            self.clear();
            self.attempts += 1;
            self.ue_deadline = Some((at + if self.attempts == 5 { 720000 } else { 10000 }, true));
            Ok(Some(TimerEvent {
                time_ms: at,
                wire: None,
                name: "T3510",
                unconfirmed: false,
            }))
        }
    }
    /// TS 24.501 18.7.0: normal WB-N1 T3560/T3550, 6 s, four retries.
    pub fn advance(&mut self, now: u64) -> Result<Vec<TimerEvent>, String> {
        if now < self.now || now > 3_600_000 {
            return Err("Invalid registration clock".into());
        }
        let mut events = vec![];
        while self.timer.as_ref().is_some_and(|t| t.deadline <= now) {
            let mut timer = self.timer.take().unwrap();
            let name = if timer.sht == 2 { "T3550" } else { "T3560" };
            let at = timer.deadline;
            if timer.retries == 4 {
                if timer.sht == 2 {
                    // §5.5.1.2.8(c): AMF enters REGISTERED, but Complete was not verified.
                    self.amf_stage = Stage::Registered;
                    self.unconfirmed = true;
                } else {
                    self.fail("T3560 exhausted; registration aborted".into());
                }
                events.push(TimerEvent {
                    time_ms: at,
                    wire: None,
                    name,
                    unconfirmed: self.unconfirmed,
                });
            } else {
                let wire = if timer.sht == 0 {
                    timer.plain.clone()
                } else {
                    protect(
                        &mut self.amf_security,
                        timer.plain.clone(),
                        timer.sht,
                        Direction::Downlink,
                    )?
                };
                timer.retries += 1;
                timer.deadline += 6000;
                self.timer = Some(timer);
                events.push(TimerEvent {
                    time_ms: at,
                    wire: Some(wire),
                    name,
                    unconfirmed: false,
                });
            }
        }
        self.now = now;
        Ok(events)
    }
    pub fn at_amf(&mut self, wire: &[u8]) -> Result<Received, String> {
        let plain = match self.amf_stage {
            Stage::Security => unprotect(&mut self.amf_security, wire, 4, Direction::Uplink)?,
            Stage::Accept => unprotect(&mut self.amf_security, wire, 2, Direction::Uplink)?,
            _ => checked(wire.to_vec())?,
        };
        let reply = match (self.amf_stage, plain.get(2).copied()) {
            (Stage::Initial, Some(0x41)) => {
                // Resolve the identity on the wire, not just the transport UE handle.
                let Nas5gsMessage::Gmm(_, Nas5gmmMessage::RegistrationRequest(ref req)) =
                    decode_nas_5gs_message(&plain).map_err(|e| e.to_string())?
                else {
                    return Err("Expected Registration Request".into());
                };
                let identity = &req.fgs_mobile_identity.value;
                let supi = oxirush_security::nas_5gs::suci_to_supi(identity, None)
                    .ok_or("Unsupported laboratory SUCI")?;
                if supi != format!("imsi-{}", auth::supi(self.id))
                    || self.fault == LabFault::UnknownSubscriber
                {
                    self.fail("Unknown laboratory subscriber (5GMM cause 3: illegal UE)".into());
                    Some(checked(vec![0x7e, 0, 0x44, 3])?)
                } else {
                    if plain != self.request {
                        return Err("Unsupported registration capabilities or identity".into());
                    }
                    Some(self.authentication_request()?)
                }
            }
            (Stage::Authentication, Some(0x57)) => {
                let Nas5gsMessage::Gmm(_, Nas5gmmMessage::AuthenticationResponse(ref res)) =
                    decode_nas_5gs_message(&plain).map_err(|e| e.to_string())?
                else {
                    return Err("Expected authentication response".into());
                };
                let res = &res
                    .authentication_response_parameter
                    .as_ref()
                    .ok_or("Missing RES*")?
                    .value;
                let v = self
                    .vector
                    .take()
                    .ok_or("Authentication vector already consumed")?;
                // SEAF HRES*/HXRES* check precedes AUSF RES*/XRES* verification.
                let hash_matches = if let Ok(res16) = <&[u8; 16]>::try_from(res.as_slice()) {
                    let hres = oxirush_security::nas_5gs::compute_hres_star(&v.rand, res16);
                    let hxres = oxirush_security::nas_5gs::compute_hres_star(&v.rand, &v.xres);
                    bool::from(hres.ct_eq(&hxres))
                } else {
                    false
                };
                if !hash_matches || !bool::from(v.xres.as_slice().ct_eq(res)) {
                    self.fail("AUSF RES*/XRES* mismatch".into());
                    Some(checked(vec![0x7e, 0, 0x58])?)
                } else {
                    self.amf_security = Some(context(&v.kamf));
                    self.amf_stage = Stage::Security;
                    let mut smc = protect(
                        &mut self.amf_security,
                        vec![0x7e, 0, 0x5d, 0x22, 0, 2, 0xa0, 0x20, 0x36, 1, 2],
                        3,
                        Direction::Downlink,
                    )?;
                    if self.fault == LabFault::InvalidMac {
                        smc[2] ^= 1;
                    }
                    if self.fault == LabFault::InvalidCounter {
                        smc[6] = 19;
                    }
                    Some(smc)
                }
            }
            (Stage::Authentication, Some(0x59)) => {
                // Cause 21, Authentication failure parameter IE 0x30, exactly 14 octets.
                if plain.get(3) == Some(&21)
                    && plain.len() == 20
                    && plain[4..6] == [0x30, 14]
                    && self.resyncs < 2
                {
                    let v = self.vector.take().ok_or("No outstanding RAND for AUTS")?;
                    match auth::resynchronize(self.id, &v.rand, &plain[6..], self.issued_sqn) {
                        Ok(next) => {
                            self.issued_sqn = next;
                            self.resyncs += 1;
                            Some(self.authentication_request()?)
                        }
                        Err(e) => {
                            self.fail(e);
                            Some(checked(vec![0x7e, 0, 0x58])?)
                        }
                    }
                } else {
                    self.fail(
                        if plain.get(3) == Some(&20) {
                            "UE rejected AUTN (5GMM cause 20)"
                        } else {
                            "Authentication failure / unsupported or repeated resynchronization"
                        }
                        .into(),
                    );
                    Some(checked(vec![0x7e, 0, 0x58])?)
                }
            }
            (Stage::Security, Some(0x5e)) => {
                let Nas5gsMessage::Gmm(_, Nas5gmmMessage::SecurityModeComplete(ref complete)) =
                    decode_nas_5gs_message(&plain).map_err(|e| e.to_string())?
                else {
                    return Err("Expected Security Mode Complete".into());
                };
                if complete
                    .nas_message_container
                    .as_ref()
                    .map(|v| v.value.as_slice())
                    != Some(self.request.as_slice())
                {
                    return Err("Initial NAS message replay mismatch".into());
                }
                self.amf_stage = Stage::Accept;
                let guti =
                    oxirush_security::nas_5gs::build_guti_bytes(&[0, 0xf1, 0x10], 1, 1, 0, self.id);
                let mut accept = vec![0x7e, 0, 0x42, 1, 1, 0x77, 0, 11];
                accept.extend(guti);
                // TAI type 0, one TAC, test PLMN 001/01, TAC 1; allowed NSSAI SST 1.
                accept.extend([0x54, 7, 0, 0, 0xf1, 0x10, 0, 0, 1]);
                let mut allowed = vec![];
                for slice in &self.allowed_slices {
                    allowed.push(slice.bytes().len() as u8);
                    allowed.extend(slice.bytes());
                }
                accept.extend([0x15, allowed.len() as u8]);
                accept.extend(allowed);
                self.accept_cache = Some(accept.clone());
                Some(protect(
                    &mut self.amf_security,
                    accept,
                    2,
                    Direction::Downlink,
                )?)
            }
            (Stage::Accept, Some(0x43)) => {
                self.amf_stage = Stage::Registered;
                None
            }
            _ => return Err("Unexpected AMF registration transition".into()),
        };
        self.timer = match self.amf_stage {
            Stage::Authentication => reply.as_ref().map(|b| RetryTimer {
                deadline: self.now + 6000,
                retries: 0,
                plain: b.clone(),
                sht: 0,
            }),
            Stage::Security => Some(RetryTimer {
                deadline: self.now + 6000,
                retries: 0,
                plain: vec![0x7e, 0, 0x5d, 0x22, 0, 2, 0xa0, 0x20, 0x36, 1, 2],
                sht: 3,
            }),
            Stage::Accept => Some(RetryTimer {
                deadline: self.now + 6000,
                retries: 0,
                plain: self.accept_cache.clone().ok_or("Missing Accept")?,
                sht: 2,
            }),
            _ => None,
        };
        Ok(Received {
            plain,
            reply,
            registered: self.amf_stage == Stage::Registered,
        })
    }
    pub fn at_ue(&mut self, wire: &[u8]) -> Result<Received, String> {
        // A retransmitted outstanding challenge reuses RES*, without re-running SQN
        // freshness or resetting NAS counters. The cache dies when SMC succeeds.
        if self.ue_stage == Stage::Authentication {
            if let Some((request, response)) = &self.auth_cache {
                if wire == request {
                    return Ok(Received {
                        plain: request.clone(),
                        reply: (self.fault != LabFault::Timeout).then(|| response.to_vec()),
                        registered: false,
                    });
                }
            }
        }
        let plain = match self.ue_stage {
            Stage::Authentication if wire != [0x7e, 0, 0x58] => {
                unprotect(&mut self.ue_security, wire, 3, Direction::Downlink)?
            }
            Stage::Security if wire.get(1) == Some(&3) => {
                unprotect(&mut self.ue_security, wire, 3, Direction::Downlink)?
            }
            Stage::Security | Stage::Registered => {
                unprotect(&mut self.ue_security, wire, 2, Direction::Downlink)?
            }
            _ => checked(wire.to_vec())?,
        };
        let reply = match (self.ue_stage, plain.get(2).copied()) {
            (_, Some(0x44 | 0x58))
                if wire.get(1) == Some(&0) && self.ue_stage != Stage::Registered =>
            {
                self.fail(
                    self.failure
                        .clone()
                        .unwrap_or("Network rejected registration".into()),
                );
                None
            }
            (Stage::Initial, Some(0x56)) => {
                let Nas5gsMessage::Gmm(_, Nas5gmmMessage::AuthenticationRequest(ref req)) =
                    decode_nas_5gs_message(&plain).map_err(|e| e.to_string())?
                else {
                    return Err("Expected authentication request".into());
                };
                if req.abba.value != auth::ABBA {
                    return Err("Unsupported ABBA".into());
                }
                let rand = req
                    .authentication_parameter_rand
                    .as_ref()
                    .ok_or("Missing RAND")?
                    .value
                    .as_slice()
                    .try_into()
                    .map_err(|_| "Invalid RAND")?;
                let autn = req
                    .authentication_parameter_autn
                    .as_ref()
                    .ok_or("Missing AUTN")?
                    .value
                    .as_slice()
                    .try_into()
                    .map_err(|_| "Invalid AUTN")?;
                let mut key = auth::lab_key(self.id);
                if self.fault == LabFault::InvalidCredentials {
                    key[0] ^= 1;
                }
                match auth::authenticate_or_sync(self.id, key, rand, autn, &mut self.sqn) {
                    Ok((mut res, kamf)) => {
                        self.ue_security = Some(context(&kamf));
                        self.ue_stage = Stage::Authentication;
                        {
                            if self.fault == LabFault::IncorrectResponse {
                                res[0] ^= 1;
                            }
                            let mut msg = vec![0x7e, 0, 0x57, 0x2d, 16];
                            msg.extend_from_slice(res.as_slice());
                            let response = checked(msg)?;
                            self.auth_cache =
                                Some((plain.clone(), zeroize::Zeroizing::new(response.clone())));
                            (self.fault != LabFault::Timeout).then_some(response)
                        }
                    }
                    Err(auth::AuthenticationFailure::Synchronization(auts)) => {
                        self.ue_security = None;
                        let mut msg = vec![0x7e, 0, 0x59, 21, 0x30, 14];
                        msg.extend(auts);
                        Some(checked(msg)?)
                    }
                    Err(reason) => {
                        self.failure = Some(reason.to_string());
                        self.ue_stage = Stage::Failed;
                        Some(checked(vec![0x7e, 0, 0x59, 20])?)
                    }
                }
            }
            (Stage::Authentication | Stage::Security, Some(0x5d)) => {
                if plain != [0x7e, 0, 0x5d, 0x22, 0, 2, 0xa0, 0x20, 0x36, 1, 2] {
                    return Err("Security algorithm/capability replay mismatch".into());
                }
                self.auth_cache = None;
                self.ue_stage = Stage::Security;
                let mut complete = vec![0x7e, 0, 0x5e, 0x71];
                complete.extend((self.request.len() as u16).to_be_bytes());
                complete.extend(&self.request);
                Some(protect(
                    &mut self.ue_security,
                    complete,
                    4,
                    Direction::Uplink,
                )?)
            }
            (Stage::Security | Stage::Registered, Some(0x42)) => {
                self.ue_deadline = None;
                self.attempts = 0;
                self.ue_stage = Stage::Registered;
                Some(protect(
                    &mut self.ue_security,
                    vec![0x7e, 0, 0x43],
                    2,
                    Direction::Uplink,
                )?)
            }
            _ => return Err("Unexpected UE registration transition".into()),
        };
        Ok(Received {
            plain,
            reply,
            registered: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nas_security_replay_tampering_and_count_rollover() {
        let mut tx = Some(context(&[42; 32]));
        let mut rx = Some(context(&[42; 32]));
        for _ in 0..260 {
            let bytes = protect(&mut tx, vec![0x7e, 0, 0x43], 2, Direction::Uplink).unwrap();
            let mut bad = bytes.clone();
            bad[2] ^= 1;
            assert!(unprotect(&mut rx, &bad, 2, Direction::Uplink).is_err());
            assert_eq!(
                unprotect(&mut rx, &bytes, 2, Direction::Uplink).unwrap(),
                [0x7e, 0, 0x43]
            );
            assert!(unprotect(&mut rx, &bytes, 2, Direction::Uplink).is_err());
        }
    }
    #[test]
    fn plaintext_security_mode_cannot_bypass_integrity_or_consume_context() {
        let mut r = Registration::new(101, 42, LabFault::None).unwrap();
        let challenge = r
            .at_amf(&protocol::registration_request(101).unwrap())
            .unwrap()
            .reply
            .unwrap();
        let response = r.at_ue(&challenge).unwrap().reply.unwrap();
        let command = r.at_amf(&response).unwrap().reply.unwrap();
        assert!(r
            .at_ue(&[0x7e, 0, 0x5d, 0x22, 0, 2, 0xa0, 0x20, 0x36, 1, 2])
            .is_err());
        let complete = r.at_ue(&command).unwrap().reply.unwrap();
        let accept = r.at_amf(&complete).unwrap().reply.unwrap();
        let complete = r.at_ue(&accept).unwrap().reply.unwrap();
        assert!(r.at_amf(&complete).unwrap().registered);
    }
    #[test]
    fn cannot_skip_authentication_or_security() {
        let mut r = Registration::new(2, 42, LabFault::None).unwrap();
        assert!(r.at_amf(&[0x7e, 0, 0x43]).is_err());
        assert!(r.at_ue(&[0x7e, 0, 0x42, 1, 1]).is_err());
    }
}
