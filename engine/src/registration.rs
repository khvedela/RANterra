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
    id: u32,
    seed: u32,
    fault: LabFault,
    ue_stage: Stage,
    amf_stage: Stage,
    vector: Option<auth::Vector>,
    ue_security: Option<NasSecurityContext>,
    amf_security: Option<NasSecurityContext>,
    sqn: u64,
    request: Vec<u8>,
    pub failure: Option<String>,
}
pub struct Received {
    pub plain: Vec<u8>,
    pub reply: Option<Vec<u8>>,
    pub registered: bool,
}
fn checked(bytes: Vec<u8>) -> Result<Vec<u8>, String> {
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
            id,
            seed,
            fault,
            ue_stage: Stage::Initial,
            amf_stage: Stage::Initial,
            vector: None,
            ue_security: None,
            amf_security: None,
            sqn: 0,
            request: protocol::registration_request(id)?,
            failure: None,
        })
    }
    pub fn fail(&mut self, reason: String) {
        self.failure = Some(reason);
        self.ue_stage = Stage::Failed;
        self.amf_stage = Stage::Failed;
        self.vector = None;
        self.ue_security = None;
        self.amf_security = None;
    }
    pub fn clear(&mut self) {
        self.vector = None;
        self.ue_security = None;
        self.amf_security = None;
        self.ue_stage = Stage::Failed;
        self.amf_stage = Stage::Failed;
    }
    pub fn protect_session(&mut self, plain: Vec<u8>, uplink: bool) -> Result<Vec<u8>, String> {
        if self.amf_stage != Stage::Registered || self.ue_stage != Stage::Registered {
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
        if self.amf_stage != Stage::Registered || self.ue_stage != Stage::Registered {
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
                    let mut v = auth::vector(self.seed, self.id, 1);
                    if self.fault == LabFault::InvalidAutn {
                        v.autn[15] ^= 1;
                    }
                    let mut msg = vec![0x7e, 0, 0x56, 0, 2, 0, 0, 0x21];
                    msg.extend(v.rand);
                    msg.extend([0x20, 16]);
                    msg.extend(v.autn);
                    self.vector = Some(v);
                    self.amf_stage = Stage::Authentication;
                    Some(checked(msg)?)
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
                self.fail("UE rejected AUTN (5GMM cause 20)".into());
                Some(checked(vec![0x7e, 0, 0x58])?)
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
                accept.extend([0x54, 7, 0, 0, 0xf1, 0x10, 0, 0, 1, 0x15, 2, 1, 1]);
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
        Ok(Received {
            plain,
            reply,
            registered: self.amf_stage == Stage::Registered && self.ue_stage == Stage::Registered,
        })
    }
    pub fn at_ue(&mut self, wire: &[u8]) -> Result<Received, String> {
        let plain = match self.ue_stage {
            Stage::Authentication if wire.get(1) == Some(&3) => {
                unprotect(&mut self.ue_security, wire, 3, Direction::Downlink)?
            }
            Stage::Security => unprotect(&mut self.ue_security, wire, 2, Direction::Downlink)?,
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
                match auth::authenticate(self.id, key, rand, autn, &mut self.sqn) {
                    Ok((mut res, kamf)) => {
                        self.ue_security = Some(context(&kamf));
                        self.ue_stage = Stage::Authentication;
                        if self.fault == LabFault::Timeout {
                            None
                        } else {
                            if self.fault == LabFault::IncorrectResponse {
                                res[0] ^= 1;
                            }
                            let mut msg = vec![0x7e, 0, 0x57, 0x2d, 16];
                            msg.extend_from_slice(res.as_slice());
                            Some(checked(msg)?)
                        }
                    }
                    Err(reason) => {
                        self.failure = Some(reason);
                        self.ue_stage = Stage::Failed;
                        Some(checked(vec![0x7e, 0, 0x59, 20])?)
                    }
                }
            }
            (Stage::Authentication, Some(0x5d)) => {
                if plain != [0x7e, 0, 0x5d, 0x22, 0, 2, 0xa0, 0x20, 0x36, 1, 2] {
                    return Err("Security algorithm/capability replay mismatch".into());
                }
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
            (Stage::Security, Some(0x42)) => {
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
    fn cannot_skip_authentication_or_security() {
        let mut r = Registration::new(2, 42, LabFault::None).unwrap();
        assert!(r.at_amf(&[0x7e, 0, 0x43]).is_err());
        assert!(r.at_ue(&[0x7e, 0, 0x42, 1, 1]).is_err());
    }
}
