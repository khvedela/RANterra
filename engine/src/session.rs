//! Collocated SMF/UPF laboratory profile. N4 is a typed local boundary, not PFCP.
use oxirush_nas::Validate;

use crate::model::{SessionRequest, Snssai};
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
    pub sd: Option<[u8; 3]>,
    pub sent: u64,
    pub delivered: u64,
    pub dropped: u64,
    pub last_rtt_ms: Option<u64>,
    pub active: bool,
}
impl Session {
    pub fn new(ue_id: u32, slot: u32, request: &SessionRequest) -> Self {
        Self {
            ue_id,
            id: 1,
            ip: [10, 45, 0, slot as u8],
            uplink_teid: slot * 2,
            downlink_teid: slot * 2 + 1,
            dnn: request.dnn.clone(),
            sst: request.snssai.sst,
            sd: request.snssai.sd,
            sent: 0,
            delivered: 0,
            dropped: 0,
            last_rtt_ms: None,
            active: false,
        }
    }
}
pub fn nas(bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    if bytes.len() > 65536 {
        return Err("NAS PDU exceeds limit".into());
    }
    let m = decode_nas_5gs_message(&bytes).map_err(|e| e.to_string())?;
    if !m.validate().is_empty() {
        return Err(format!("Invalid session NAS: {:?}", m.validate()));
    }
    encode_nas_5gs_message(&m).map_err(|e| e.to_string())
}
pub fn request() -> Result<Vec<u8>, String> {
    nas(vec![0x2e, 1, 1, 0xc1, 0xff, 0xff, 0x91, 0xa1])
}
pub fn accept(ip: [u8; 4], policy: &SessionRequest) -> Result<Vec<u8>, String> {
    // SSC1/IPv4; one default QoS rule, match-all packet filter, QFI1;
    // session AMBR 1 Mbit/s in both directions; 5QI9 non-GBR.
    let mut b = vec![
        0x2e, 1, 1, 0xc2, 0x11, 0, 9, 1, 0, 6, 0x31, 0x31, 1, 1, 0xff, 1, 6, 6, 0, 1, 6, 0, 1,
        0x29, 5, 1,
    ];
    b.extend(ip);
    b.extend([0x22, policy.snssai.bytes().len() as u8]);
    b.extend(policy.snssai.bytes());
    b.extend([0x79, 0, 6, 1, 0x20, 0x41, 1, 1, 9]);
    b.extend([0x25, policy.dnn_bytes().len() as u8]);
    b.extend(policy.dnn_bytes());
    nas(b)
}
pub fn n1(
    sm: Vec<u8>,
    uplink: bool,
    initial: bool,
    policy: &SessionRequest,
) -> Result<Vec<u8>, String> {
    if sm.len() > 65535 {
        return Err("N1 payload exceeds length field".into());
    }
    let mut b = vec![0x7e, 0, if uplink { 0x67 } else { 0x68 }, 1];
    b.extend((sm.len() as u16).to_be_bytes());
    b.extend(sm);
    b.extend([0x12, 1]);
    if initial {
        b.extend([0x81, 0x22, policy.snssai.bytes().len() as u8]);
        b.extend(policy.snssai.bytes());
        b.extend([0x25, policy.dnn_bytes().len() as u8]);
        b.extend(policy.dnn_bytes());
    }
    nas(b)
}
pub fn unwrap_n1(bytes: &[u8], uplink: bool) -> Result<Vec<u8>, String> {
    if bytes.len() > 65536 {
        return Err("NAS PDU exceeds limit".into());
    }
    let m = decode_nas_5gs_message(bytes).map_err(|e| e.to_string())?;
    use oxirush_nas::nas_5gs::{Nas5gmmMessage, Nas5gsMessage};
    let (p, id, kind) = match m {
        Nas5gsMessage::Gmm(_, Nas5gmmMessage::UlNasTransport(m)) if uplink => (
            m.payload_container.value,
            m.pdu_session_id.map(|i| i.value),
            m.payload_container_type.value & 15,
        ),
        Nas5gsMessage::Gmm(_, Nas5gmmMessage::DlNasTransport(m)) if !uplink => (
            m.payload_container.value,
            m.pdu_session_id.map(|i| i.value),
            m.payload_container_type.value & 15,
        ),
        _ => return Err("Unexpected N1 SM transport".into()),
    };
    if kind != 1
        || id != Some(1)
        || p.get(1) != Some(&1)
        || !(p.get(2) == Some(&1)
            || (p.get(2) == Some(&0) && matches!(p.get(3), Some(0xd3 | 0xd4))))
    {
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
pub fn setup(ue: u32, teid: u32, nas: Vec<u8>, slice: &Snssai) -> Result<Vec<u8>, String> {
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
        (
            130,
            encode(&PDUSessionAggregateMaximumBitRate::new(
                BitRate(1_000_000.into()),
                BitRate(1_000_000.into()),
                None,
            ))?,
        ),
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
        SNSSAI::new(
            SST(FixedOctetString::new([slice.sst])),
            slice.sd.map(|sd| SD(FixedOctetString::new(sd))),
            None,
        ),
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
pub fn decode_setup(bytes: &[u8], ue: u32, teid: u32, slice: &Snssai) -> Result<Vec<u8>, String> {
    if bytes.len() > 65536 {
        return Err("NGAP PDU exceeds limit".into());
    }
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
    if item.s_nssai.s_st.0.as_ref() != [slice.sst]
        || item.s_nssai.s_d.as_ref().map(|sd| sd.0.as_ref())
            != slice.sd.as_ref().map(|sd| sd.as_slice())
    {
        return Err("N2 S-NSSAI mismatch".into());
    }
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
/// Supported failure outcome: NG-RAN cannot allocate radio resources (TS 38.413).
pub fn setup_failure(ue: u32) -> Result<Vec<u8>, String> {
    let transfer = PDUSessionResourceSetupUnsuccessfulTransfer::new(
        Cause::radioNetwork(CauseRadioNetwork::radio_resources_not_available),
        None,
        None,
    );
    let list = PDUSessionResourceFailedToSetupListSURes(vec![
        PDUSessionResourceFailedToSetupItemSURes::new(
            PDUSessionID(1),
            encode(&transfer)?.into(),
            None,
        ),
    ]);
    build_ngap!(SuccessfulOutcome,PDUSessionResourceSetup,REJECT,PDUSessionResourceSetupResponse,
        IGNORE AMF_UE_NGAP_ID(ue as u64),IGNORE RAN_UE_NGAP_ID(ue),IGNORE PDUSessionResourceFailedToSetupListSURes(list),
    ).encode().map_err(|e|e.to_string())
}
pub fn validate_setup_failure(bytes: &[u8], ue: u32) -> Result<(), String> {
    if bytes.len() > 65536 {
        return Err("NGAP PDU exceeds limit".into());
    }
    let pdu = NGAP_PDU::decode(bytes).map_err(|e| e.to_string())?;
    if pdu.encode().map_err(|e| e.to_string())? != bytes
        || pdu != NGAP_PDU::decode(&setup_failure(ue)?).map_err(|e| e.to_string())?
    {
        return Err("Unsupported N2 failure or mismatched context".into());
    }
    Ok(())
}
pub fn validate_response(bytes: &[u8], ue: u32, teid: u32) -> Result<(), String> {
    if bytes.len() > 65536 {
        return Err("NGAP PDU exceeds limit".into());
    }
    let pdu = NGAP_PDU::decode(bytes).map_err(|e| e.to_string())?;
    if pdu.encode().map_err(|e| e.to_string())? != bytes {
        return Err("Noncanonical or trailing N2 bytes".into());
    }
    let NGAP_PDU::successfulOutcome(decoded) = pdu else {
        return Err("Expected N2 setup response".into());
    };
    let NGAP_PDU::successfulOutcome(expected) =
        NGAP_PDU::decode(&setup_response(ue, teid)?).map_err(|e| e.to_string())?
    else {
        unreachable!()
    };
    if decoded.procedure_code != expected.procedure_code
        || decoded.criticality != expected.criticality
    {
        return Err("N2 response procedure mismatch".into());
    }
    let decoded: PDUSessionResourceSetupResponse =
        decode_open_type(&decoded.value).map_err(|e| e.to_string())?;
    let expected: PDUSessionResourceSetupResponse =
        decode_open_type(&expected.value).map_err(|e| e.to_string())?;
    // TS 38.413 9.4.1 requires object-set ordering and exact mandatory IE multiplicity.
    if decoded != expected {
        return Err("N2 response context mismatch".into());
    }
    Ok(())
}
/// The gNodeB allocates its downlink TEID; the SMF must not prescribe it.
pub fn response_teid(bytes: &[u8], ue: u32) -> Result<u32, String> {
    if bytes.len() > 65536 {
        return Err("NGAP PDU exceeds limit".into());
    }
    let NGAP_PDU::successfulOutcome(p) = NGAP_PDU::decode(bytes).map_err(|e| e.to_string())? else {
        return Err("Expected setup response".into());
    };
    if p.procedure_code.0 != 29 {
        return Err("Wrong procedure".into());
    }
    let m: PDUSessionResourceSetupResponse =
        decode_open_type(&p.value).map_err(|e| e.to_string())?;
    let ie = m
        .protocol_ies
        .0
        .iter()
        .find(|i| i.id.0 == 75)
        .ok_or("Missing setup success list")?;
    let list: PDUSessionResourceSetupListSURes =
        decode_open_type(&ie.value).map_err(|e| e.to_string())?;
    if list.0.len() != 1 {
        return Err("Expected one session".into());
    }
    let tr: PDUSessionResourceSetupResponseTransfer =
        rasn::aper::decode(&list.0[0].p_dusession_resource_setup_response_transfer)
            .map_err(|e| e.to_string())?;
    let UPTransportLayerInformation::gTPTunnel(t) = tr
        .d_lqos_flow_per_tnlinformation
        .u_ptransport_layer_information
    else {
        return Err("Expected GTP tunnel".into());
    };
    let teid = u32::from_be_bytes(
        t.g_tp_teid
            .0
            .as_ref()
            .try_into()
            .map_err(|_| "Invalid TEID")?,
    );
    if teid == 0 {
        return Err("Zero TEID".into());
    }
    validate_response(bytes, ue, teid)?;
    Ok(teid)
}
pub fn validate_release_response(bytes: &[u8], ue: u32) -> Result<(), String> {
    if bytes.len() > 65536 {
        return Err("NGAP PDU exceeds limit".into());
    }
    let expected = release_n2(ue)?.pop().unwrap();
    let decoded = NGAP_PDU::decode(bytes).map_err(|e| e.to_string())?;
    if decoded.encode().map_err(|e| e.to_string())? != expected || bytes != expected {
        return Err("N2 release response context mismatch".into());
    }
    Ok(())
}
/// Decode the selected service from the actual N1 transport, not UI configuration.
pub fn selection(bytes: &[u8]) -> Result<SessionRequest, String> {
    unwrap_n1(bytes, true)?;
    use oxirush_nas::nas_5gs::{Nas5gmmMessage, Nas5gsMessage};
    let Nas5gsMessage::Gmm(_, Nas5gmmMessage::UlNasTransport(m)) =
        decode_nas_5gs_message(bytes).map_err(|e| e.to_string())?
    else {
        return Err("Expected UL transport".into());
    };
    if m.request_type.as_ref().map(|t| t.value & 7) != Some(1) {
        return Err("Expected initial request type".into());
    }
    let d = m.dnn.ok_or("Missing DNN")?.value;
    let mut rest = d.as_slice();
    let mut labels = vec![];
    while !rest.is_empty() {
        let n = rest[0] as usize;
        if n == 0 || rest.len() < n + 1 {
            return Err("Invalid DNN labels".into());
        }
        labels.push(std::str::from_utf8(&rest[1..n + 1]).map_err(|_| "Invalid DNN ASCII")?);
        rest = &rest[n + 1..];
    }
    let s = m.s_nssai.ok_or("Missing slice")?.value;
    if s.len() != 1 && s.len() != 4 {
        return Err("Unsupported S-NSSAI length".into());
    }
    let r = SessionRequest {
        dnn: labels.join("."),
        snssai: Snssai {
            sst: s[0],
            sd: if s.len() == 4 {
                Some(s[1..].try_into().unwrap())
            } else {
                None
            },
        },
    };
    r.validate()?;
    Ok(r)
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
pub fn validate_echo_packet(bytes: &[u8], ip: [u8; 4], reply: bool) -> Result<(), String> {
    let (source, destination, kind) = if reply {
        ([10, 45, 255, 1], ip, 0)
    } else {
        (ip, [10, 45, 255, 1], 8)
    };
    if bytes.len() != 36
        || bytes[0] != 0x45
        || bytes[2..4] != [0, 36]
        || bytes[9] != 1
        || bytes[8] == 0
        || u16::from_be_bytes([bytes[6], bytes[7]]) & 0xbfff != 0
        || bytes[12..16] != source
        || bytes[16..20] != destination
        || bytes[20..22] != [kind, 0]
        || checksum(&bytes[..20]) != 0
        || checksum(&bytes[20..]) != 0
    {
        return Err("Invalid IPv4/ICMP echo route or checksum".into());
    }
    Ok(())
}
pub fn echo_reply(bytes: &[u8], ip: [u8; 4]) -> Result<Vec<u8>, String> {
    validate_echo_packet(bytes, ip, false)?;
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

/// AMF validates the service selected on the actual decoded UL transport.
pub fn validate_selection(bytes: &[u8], expected: &SessionRequest) -> Result<(), String> {
    use oxirush_nas::nas_5gs::{Nas5gmmMessage, Nas5gsMessage};
    if bytes.len() > 65536 {
        return Err("NAS PDU exceeds limit".into());
    }
    let Nas5gsMessage::Gmm(_, Nas5gmmMessage::UlNasTransport(m)) =
        decode_nas_5gs_message(bytes).map_err(|e| e.to_string())?
    else {
        return Err("Expected UL NAS transport".into());
    };
    if m.dnn.as_ref().map(|d| &d.value) != Some(&expected.dnn_bytes())
        || m.s_nssai.as_ref().map(|s| &s.value) != Some(&expected.snssai.bytes())
    {
        return Err("N1 service selection mismatch".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn n1_sm_roundtrips() {
        let r = request().unwrap();
        assert_eq!(
            unwrap_n1(
                &n1(r.clone(), true, true, &SessionRequest::default()).unwrap(),
                true
            )
            .unwrap(),
            r
        );
        accept([10, 45, 0, 1], &SessionRequest::default()).unwrap();
    }
    #[test]
    fn n2_setup_roundtrip() {
        let b = setup(101, 2, vec![1, 2, 3], &Snssai::default()).unwrap();
        assert_eq!(
            decode_setup(&b, 101, 2, &Snssai::default()).unwrap(),
            [1, 2, 3]
        );
        assert!(decode_setup(&b, 102, 2, &Snssai::default()).is_err());
        let b = setup_response(101, 3).unwrap();
        validate_response(&b, 101, 3).unwrap();
    }
    #[test]
    fn rejects_wrong_n1_payload_kind_and_invalid_echo_headers() {
        let mut b = n1(request().unwrap(), true, true, &SessionRequest::default()).unwrap();
        b[3] = 2; // SMS payload type must not be dispatched as 5GSM.
        assert!(unwrap_n1(&b, true).is_err());
        let ip = [10, 45, 0, 1];
        for (offset, value) in [(6, 0x20), (6, 0x80), (7, 1), (8, 0)] {
            let mut b = echo_packet(ip, 1);
            b[offset] = value;
            b[10] = 0;
            b[11] = 0;
            let c = checksum(&b[..20]);
            b[10..12].copy_from_slice(&c.to_be_bytes());
            assert!(echo_reply(&b, ip).is_err());
        }
        let mut reply = echo_reply(&echo_packet(ip, 1), ip).unwrap();
        validate_echo_packet(&reply, ip, true).unwrap();
        reply[35] ^= 1;
        assert!(validate_echo_packet(&reply, ip, true).is_err());
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
