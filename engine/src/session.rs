//! Collocated SMF/UPF laboratory profile. N4 is a typed local boundary, not PFCP.
use oxirush_nas::Validate;

use oxirush_nas::nas_5gs::{decode_nas_5gs_message, encode_nas_5gs_message};
use oxirush_ngap::{build_ngap, extract_ngap_ies, helpers::*, macros::MissingIeError, ngap::*};
use rasn::types::{Any, FixedOctetString};
use serde::Serialize;
#[derive(Clone, Debug, Serialize)]
pub struct Session {
    pub ue_id: u32,
    pub id: u8,
    pub ip: [u8; 4],
    pub uplink_teid: u32,
    pub downlink_teid: u32,
    pub dnn: String,
    pub sst: u8,
    pub sent: u64,
    pub delivered: u64,
    pub dropped: u64,
    pub last_rtt_ms: Option<u64>,
    pub active: bool,
}
impl Session {
    pub fn new(ue_id: u32, slot: u32) -> Self {
        Self {
            ue_id,
            id: 1,
            ip: [10, 45, 0, slot as u8],
            uplink_teid: slot * 2,
            downlink_teid: slot * 2 + 1,
            dnn: "lab".into(),
            sst: 1,
            sent: 0,
            delivered: 0,
            dropped: 0,
            last_rtt_ms: None,
            active: false,
        }
    }
}
pub fn nas(bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    let m = decode_nas_5gs_message(&bytes).map_err(|e| e.to_string())?;
    if !m.validate().is_empty() {
        return Err(format!("Invalid session NAS: {:?}", m.validate()));
    }
    encode_nas_5gs_message(&m).map_err(|e| e.to_string())
}
pub fn request() -> Result<Vec<u8>, String> {
    nas(vec![0x2e, 1, 1, 0xc1, 0xff, 0xff, 0x91, 0xa1])
}
pub fn accept(ip: [u8; 4]) -> Result<Vec<u8>, String> {
    // SSC1/IPv4; one default QoS rule, match-all packet filter, QFI1;
    // session AMBR 1 Mbit/s in both directions; 5QI9 non-GBR.
    let mut b = vec![
        0x2e, 1, 1, 0xc2, 0x11, 0, 9, 1, 0, 6, 0x31, 0x31, 1, 1, 0xff, 1, 6, 6, 0, 1, 6, 0, 1,
        0x29, 5, 1,
    ];
    b.extend(ip);
    b.extend([
        0x22, 1, 1, 0x79, 0, 6, 1, 0x20, 0x41, 1, 1, 9, 0x25, 4, 3, b'l', b'a', b'b',
    ]);
    nas(b)
}
pub fn n1(sm: Vec<u8>, uplink: bool, initial: bool) -> Result<Vec<u8>, String> {
    let mut b = vec![0x7e, 0, if uplink { 0x67 } else { 0x68 }, 1];
    b.extend((sm.len() as u16).to_be_bytes());
    b.extend(sm);
    b.extend([0x12, 1]);
    if initial {
        b.extend([0x81, 0x22, 1, 1, 0x25, 4, 3, b'l', b'a', b'b']);
    }
    nas(b)
}
pub fn unwrap_n1(bytes: &[u8], uplink: bool) -> Result<Vec<u8>, String> {
    let m = decode_nas_5gs_message(bytes).map_err(|e| e.to_string())?;
    use oxirush_nas::nas_5gs::{Nas5gmmMessage, Nas5gsMessage};
    let (p, id) = match m {
        Nas5gsMessage::Gmm(_, Nas5gmmMessage::UlNasTransport(m)) if uplink => {
            (m.payload_container.value, m.pdu_session_id.map(|i| i.value))
        }
        Nas5gsMessage::Gmm(_, Nas5gmmMessage::DlNasTransport(m)) if !uplink => {
            (m.payload_container.value, m.pdu_session_id.map(|i| i.value))
        }
        _ => return Err("Unexpected N1 SM transport".into()),
    };
    if id != Some(1) || p.get(1) != Some(&1) || p.get(2) != Some(&1) {
        return Err("PDU session ID / PTI mismatch".into());
    }
    nas(p)
}
fn encode<T: rasn::Encode>(v: &T) -> Result<Vec<u8>, String> {
    rasn::aper::encode(v).map_err(|e| e.to_string())
}
fn tunnel(teid: u32) -> UPTransportLayerInformation {
    UPTransportLayerInformation::gTPTunnel(GTPTunnel::new(
        TransportLayerAddress(bytes_to_bitvec(&[10, 0, 0, 1])),
        GTPTEID(FixedOctetString::new(teid.to_be_bytes())),
        None,
    ))
}
pub fn setup(ue: u32, teid: u32, nas: Vec<u8>) -> Result<Vec<u8>, String> {
    let qos = QosFlowSetupRequestList(vec![QosFlowSetupRequestItem::new(
        QosFlowIdentifier(1.into()),
        QosFlowLevelQosParameters::new(
            QosCharacteristics::nonDynamic5QI(NonDynamic5QIDescriptor::new(
                FiveQI(9.into()),
                None,
                None,
                None,
                None,
            )),
            AllocationAndRetentionPriority::new(
                PriorityLevelARP(8),
                PreEmptionCapability::shall_not_trigger_pre_emption,
                PreEmptionVulnerability::not_pre_emptable,
                None,
            ),
            None,
            None,
            None,
            None,
        ),
        None,
        None,
    )]);
    let ies = vec![
        (139, encode(&tunnel(teid))?),
        (134, encode(&PDUSessionType::ipv4)?),
        (136, encode(&qos)?),
    ];
    let transfer = PDUSessionResourceSetupRequestTransfer::new(
        PDUSessionResourceSetupRequestTransferProtocolIEs(
            ies.into_iter()
                .map(|(id, b)| {
                    AnonymousPDUSessionResourceSetupRequestTransferProtocolIEs::new(
                        ProtocolIEID(id),
                        Criticality::reject,
                        Any::new(b),
                    )
                })
                .collect(),
        ),
    );
    let list = PDUSessionResourceSetupListSUReq(vec![PDUSessionResourceSetupItemSUReq::new(
        PDUSessionID(1),
        Some(NASPDU(nas.into())),
        SNSSAI::new(SST(FixedOctetString::new([1])), None, None),
        encode(&transfer)?.into(),
        None,
    )]);
    build_ngap!(InitiatingMessage,PDUSessionResourceSetup,REJECT,PDUSessionResourceSetupRequest,
        REJECT AMF_UE_NGAP_ID(ue as u64),REJECT RAN_UE_NGAP_ID(ue),REJECT PDUSessionResourceSetupListSUReq(list),
    ).encode().map_err(|e|e.to_string())
}
fn setup_ies(
    m: &PDUSessionResourceSetupRequest,
) -> Result<(u32, u64, PDUSessionResourceSetupListSUReq), MissingIeError> {
    extract_ngap_ies!(m,PDUSessionResourceSetupRequest,req ue:u32=RAN_UE_NGAP_ID(id),req amf:u64=AMF_UE_NGAP_ID(id),req list:PDUSessionResourceSetupListSUReq=PDUSessionResourceSetupListSUReq(v)=>v.clone(),);
    Ok((ue, amf, list))
}
pub fn decode_setup(bytes: &[u8], ue: u32, teid: u32) -> Result<Vec<u8>, String> {
    let NGAP_PDU::initiatingMessage(p) = NGAP_PDU::decode(bytes).map_err(|e| e.to_string())? else {
        return Err("Expected N2 setup request".into());
    };
    if p.procedure_code.0 != 29 {
        return Err("Expected N2 setup procedure".into());
    }
    let msg =
        decode_open_type::<PDUSessionResourceSetupRequest>(&p.value).map_err(|e| e.to_string())?;
    let (ran, amf, list) = setup_ies(&msg).map_err(|e| e.to_string())?;
    if ran != ue || amf != ue as u64 || list.0.len() != 1 || list.0[0].p_dusession_id.0 != 1 {
        return Err("N2 session context mismatch".into());
    }
    let item = &list.0[0];
    let transfer: PDUSessionResourceSetupRequestTransfer =
        rasn::aper::decode(&item.p_dusession_resource_setup_request_transfer)
            .map_err(|e| e.to_string())?;
    let ie = transfer
        .protocol_ies
        .0
        .iter()
        .find(|i| i.id.0 == 139)
        .ok_or("Missing uplink tunnel")?;
    let transport: UPTransportLayerInformation =
        decode_open_type(&ie.value).map_err(|e| e.to_string())?;
    if transport != tunnel(teid) {
        return Err("N2 TEID mismatch".into());
    }
    Ok(item
        .p_dusession_nas_pdu
        .as_ref()
        .ok_or("Missing N1 accept")?
        .0
        .to_vec())
}
pub fn setup_response(ue: u32, teid: u32) -> Result<Vec<u8>, String> {
    let transfer = PDUSessionResourceSetupResponseTransfer::new(
        QosFlowPerTNLInformation::new(
            tunnel(teid),
            AssociatedQosFlowList(vec![AssociatedQosFlowItem::new(
                QosFlowIdentifier(1.into()),
                None,
                None,
            )]),
            None,
        ),
        None,
        None,
        None,
        None,
    );
    let list = PDUSessionResourceSetupListSURes(vec![PDUSessionResourceSetupItemSURes::new(
        PDUSessionID(1),
        encode(&transfer)?.into(),
        None,
    )]);
    build_ngap!(SuccessfulOutcome,PDUSessionResourceSetup,REJECT,PDUSessionResourceSetupResponse,
        IGNORE AMF_UE_NGAP_ID(ue as u64),IGNORE RAN_UE_NGAP_ID(ue),IGNORE PDUSessionResourceSetupListSURes(list),
    ).encode().map_err(|e|e.to_string())
}
pub fn validate_response(bytes: &[u8], ue: u32, teid: u32) -> Result<(), String> {
    let decoded = NGAP_PDU::decode(bytes).map_err(|e| e.to_string())?;
    let expected = NGAP_PDU::decode(&setup_response(ue, teid)?).map_err(|e| e.to_string())?;
    if decoded != expected {
        return Err("N2 response context mismatch".into());
    }
    Ok(())
}
/// TS 29.281 G-PDU, no optional fields or extension headers in this profile.
pub fn gtpu(teid: u32, payload: &[u8]) -> Result<Vec<u8>, String> {
    if teid == 0 || payload.len() > 1500 {
        return Err("Invalid TEID or MTU".into());
    }
    let mut b = vec![0x30, 0xff];
    b.extend((payload.len() as u16).to_be_bytes());
    b.extend(teid.to_be_bytes());
    b.extend(payload);
    Ok(b)
}
pub fn decode_gtpu(bytes: &[u8], teid: u32) -> Result<&[u8], String> {
    if bytes.len() < 8
        || bytes.len() > 1508
        || bytes[..2] != [0x30, 0xff]
        || u16::from_be_bytes([bytes[2], bytes[3]]) as usize != bytes.len() - 8
        || bytes[4..8] != teid.to_be_bytes()
        || teid == 0
    {
        return Err("Malformed GTP-U / unknown TEID".into());
    }
    Ok(&bytes[8..])
}
pub fn release_n2(ue: u32) -> Result<Vec<Vec<u8>>, String> {
    let transfer =
        PDUSessionResourceReleaseCommandTransfer::new(Cause::nas(CauseNas::normal_release), None);
    let list =
        PDUSessionResourceToReleaseListRelCmd(vec![PDUSessionResourceToReleaseItemRelCmd::new(
            PDUSessionID(1),
            encode(&transfer)?.into(),
            None,
        )]);
    let command=build_ngap!(InitiatingMessage,PDUSessionResourceRelease,REJECT,PDUSessionResourceReleaseCommand,
        REJECT AMF_UE_NGAP_ID(ue as u64),REJECT RAN_UE_NGAP_ID(ue),REJECT PDUSessionResourceToReleaseListRelCmd(list),
    ).encode().map_err(|e|e.to_string())?;
    let transfer = PDUSessionResourceReleaseResponseTransfer::new(None);
    let list =
        PDUSessionResourceReleasedListRelRes(vec![PDUSessionResourceReleasedItemRelRes::new(
            PDUSessionID(1),
            encode(&transfer)?.into(),
            None,
        )]);
    let response=build_ngap!(SuccessfulOutcome,PDUSessionResourceRelease,REJECT,PDUSessionResourceReleaseResponse,
        IGNORE AMF_UE_NGAP_ID(ue as u64),IGNORE RAN_UE_NGAP_ID(ue),IGNORE PDUSessionResourceReleasedListRelRes(list),
    ).encode().map_err(|e|e.to_string())?;
    for b in [&command, &response] {
        NGAP_PDU::decode(b).map_err(|e| e.to_string())?;
    }
    Ok(vec![command, response])
}
fn checksum(bytes: &[u8]) -> u16 {
    let mut sum: u32 = bytes
        .chunks(2)
        .map(|b| u16::from_be_bytes([b[0], *b.get(1).unwrap_or(&0)]) as u32)
        .sum();
    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}
pub fn echo_packet(ip: [u8; 4], seq: u16) -> Vec<u8> {
    let mut b = vec![0x45, 0, 0, 36, 0, 0, 0, 0, 64, 1, 0, 0];
    b.extend(ip);
    b.extend([10, 45, 255, 1]);
    let c = checksum(&b);
    b[10..12].copy_from_slice(&c.to_be_bytes());
    b.extend([8, 0, 0, 0, 0, 1]);
    b.extend(seq.to_be_bytes());
    b.extend(b"RANterra");
    let c = checksum(&b[20..]);
    b[22..24].copy_from_slice(&c.to_be_bytes());
    b
}
pub fn echo_reply(bytes: &[u8], ip: [u8; 4]) -> Result<Vec<u8>, String> {
    if bytes.len() != 36
        || bytes[0] != 0x45
        || bytes[2..4] != [0, 36]
        || bytes[9] != 1
        || bytes[12..16] != ip
        || bytes[16..20] != [10, 45, 255, 1]
        || bytes[20..22] != [8, 0]
        || checksum(&bytes[..20]) != 0
        || checksum(&bytes[20..]) != 0
    {
        return Err("Invalid IPv4/ICMP echo route or checksum".into());
    }
    let mut b = bytes.to_vec();
    b[12..16].copy_from_slice(&[10, 45, 255, 1]);
    b[16..20].copy_from_slice(&ip);
    b[20] = 0;
    b[22] = 0;
    b[23] = 0;
    let c = checksum(&b[20..]);
    b[22..24].copy_from_slice(&c.to_be_bytes());
    Ok(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn n1_sm_roundtrips() {
        let r = request().unwrap();
        assert_eq!(
            unwrap_n1(&n1(r.clone(), true, true).unwrap(), true).unwrap(),
            r
        );
        accept([10, 45, 0, 1]).unwrap();
    }
    #[test]
    fn n2_setup_roundtrip() {
        let b = setup(101, 2, vec![1, 2, 3]).unwrap();
        assert_eq!(decode_setup(&b, 101, 2).unwrap(), [1, 2, 3]);
        assert!(decode_setup(&b, 102, 2).is_err());
        let b = setup_response(101, 3).unwrap();
        validate_response(&b, 101, 3).unwrap();
    }
    #[test]
    fn gtpu_wire_and_malformed() {
        let b = gtpu(42, &[1, 2, 3]).unwrap();
        assert_eq!(b, [0x30, 0xff, 0, 3, 0, 0, 0, 42, 1, 2, 3]);
        assert_eq!(decode_gtpu(&b, 42).unwrap(), [1, 2, 3]);
        for n in 0..b.len() {
            assert!(decode_gtpu(&b[..n], 42).is_err());
        }
        assert!(decode_gtpu(&b, 43).is_err());
    }
}
