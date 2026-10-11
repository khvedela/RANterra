//! TS 38.413 APER envelopes. NG setup and SCTP are intentionally outside this lab.
use oxirush_ngap::{build_ngap, extract_ngap_ies, helpers::*, macros::MissingIeError, ngap::*};
pub fn initial(nas: Vec<u8>, ue: u32, cell: u32) -> Result<Vec<u8>, String> {
    let network = plmn("001", "01");
    let location =
        UserLocationInformation::userLocationInformationNR(UserLocationInformationNR::new(
            nr_cgi(network.clone(), cell, 0),
            tai(network, &[0, 0, 1]),
            None,
            None,
        ));
    let pdu = build_ngap!(InitiatingMessage,InitialUEMessage,IGNORE,InitialUEMessage,
        REJECT RAN_UE_NGAP_ID(ue),REJECT NAS_PDU(nas),REJECT UserLocationInformation(location),
    );
    // OxiRush 0.4.2 maps RRCEstablishmentCause to 237 in its macro (should be 90).
    // Explicit Release-18 IE adapter; independently checked by free5GC.
    let NGAP_PDU::initiatingMessage(mut outer) = pdu else {
        unreachable!()
    };
    let mut message: InitialUEMessage =
        decode_open_type(&outer.value).map_err(|e| e.to_string())?;
    message
        .protocol_ies
        .0
        .push(AnonymousInitialUEMessageProtocolIEs::new(
            ProtocolIEID(90),
            Criticality::ignore,
            rasn::types::Any::new(
                rasn::aper::encode(&RRCEstablishmentCause::mo_Signalling)
                    .map_err(|e| e.to_string())?,
            ),
        ));
    outer.value = rasn::types::Any::new(rasn::aper::encode(&message).map_err(|e| e.to_string())?);
    NGAP_PDU::initiatingMessage(outer)
        .encode()
        .map_err(|e| e.to_string())
}
pub fn downlink(nas: Vec<u8>, ue: u32) -> Result<Vec<u8>, String> {
    let pdu = build_ngap!(InitiatingMessage,DownlinkNASTransport,IGNORE,DownlinkNASTransport,
        REJECT AMF_UE_NGAP_ID(ue as u64),REJECT RAN_UE_NGAP_ID(ue),REJECT NAS_PDU(nas),
    );
    pdu.encode().map_err(|e| e.to_string())
}
pub fn uplink(nas: Vec<u8>, ue: u32, cell: u32) -> Result<Vec<u8>, String> {
    let network = plmn("001", "01");
    let location =
        UserLocationInformation::userLocationInformationNR(UserLocationInformationNR::new(
            nr_cgi(network.clone(), cell, 0),
            tai(network, &[0, 0, 1]),
            None,
            None,
        ));
    let pdu = build_ngap!(InitiatingMessage,UplinkNASTransport,IGNORE,UplinkNASTransport,
        REJECT AMF_UE_NGAP_ID(ue as u64),REJECT RAN_UE_NGAP_ID(ue),REJECT NAS_PDU(nas),IGNORE UserLocationInformation(location),
    );
    pdu.encode().map_err(|e| e.to_string())
}
fn uplink_nas(msg: &UplinkNASTransport) -> Result<(u32, Vec<u8>), MissingIeError> {
    extract_ngap_ies!(msg,UplinkNASTransport,req ue:u32=RAN_UE_NGAP_ID(id),req amf:u64=AMF_UE_NGAP_ID(id),req nas:Vec<u8>=NAS_PDU(pdu)=>pdu.0.to_vec(),);
    if amf != ue as u64 {
        return Ok((0, nas));
    }
    Ok((ue, nas))
}
fn initial_nas(msg: &InitialUEMessage) -> Result<(u32, Vec<u8>), MissingIeError> {
    extract_ngap_ies!(msg,InitialUEMessage,req ue:u32=RAN_UE_NGAP_ID(id),req nas:Vec<u8>=NAS_PDU(pdu)=>pdu.0.to_vec(),);
    Ok((ue, nas))
}
fn downlink_nas(msg: &DownlinkNASTransport) -> Result<(u32, Vec<u8>), MissingIeError> {
    extract_ngap_ies!(msg,DownlinkNASTransport,req ue:u32=RAN_UE_NGAP_ID(id),req amf:u64=AMF_UE_NGAP_ID(id),req nas:Vec<u8>=NAS_PDU(pdu)=>pdu.0.to_vec(),);
    if amf != ue as u64 {
        return Ok((0, nas));
    }
    Ok((ue, nas))
}
pub fn unwrap(bytes: &[u8], expected_ue: u32) -> Result<(String, Vec<u8>), String> {
    if bytes.len() > 65536 {
        return Err("NGAP PDU exceeds limit".into());
    }
    let pdu = NGAP_PDU::decode(bytes).map_err(|e| e.to_string())?;
    let name = pdu.procedure_name().to_string();
    let (ue, nas) = match pdu {
        NGAP_PDU::initiatingMessage(m) if m.procedure_code.0 == 15 => {
            initial_nas(&decode_open_type::<InitialUEMessage>(&m.value).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?
        }
        NGAP_PDU::initiatingMessage(m) if m.procedure_code.0 == 4 => downlink_nas(
            &decode_open_type::<DownlinkNASTransport>(&m.value).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?,
        NGAP_PDU::initiatingMessage(m) if m.procedure_code.0 == 46 => uplink_nas(
            &decode_open_type::<UplinkNASTransport>(&m.value).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?,
        _ => return Err("Unsupported NGAP procedure".into()),
    };
    if ue != expected_ue {
        return Err("NGAP UE identifier mismatch".into());
    }
    Ok((name, nas))
}
/// UE-associated logical N2 context release after protected deregistration.
pub fn release_context(ue: u32) -> Result<Vec<Vec<u8>>, String> {
    let ids = UENGAPIDs::uE_NGAP_ID_pair(UENGAPIDPair::new(
        AMFUENGAPID(ue as u64),
        RANUENGAPID(ue),
        None,
    ));
    let cause = Cause::nas(CauseNas::normal_release);
    let command = build_ngap!(InitiatingMessage,UEContextRelease,REJECT,UEContextReleaseCommand,
        REJECT UE_NGAP_IDs(ids),IGNORE Cause(cause),
    )
    .encode()
    .map_err(|e| e.to_string())?;
    let complete = build_ngap!(SuccessfulOutcome,UEContextRelease,REJECT,UEContextReleaseComplete,
        IGNORE AMF_UE_NGAP_ID(ue as u64),IGNORE RAN_UE_NGAP_ID(ue),
    )
    .encode()
    .map_err(|e| e.to_string())?;
    for bytes in [&command, &complete] {
        NGAP_PDU::decode(bytes).map_err(|e| e.to_string())?;
    }
    Ok(vec![command, complete])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_corpus_does_not_panic() {
        for len in 0..128 {
            for byte in [0, 0xff, 0x7e, 0x30] {
                let bytes = vec![byte; len];
                assert!(std::panic::catch_unwind(|| unwrap(&bytes, 1)).is_ok());
            }
        }
    }
    #[test]
    fn initial_ue_uses_standard_rrc_establishment_ie_90() {
        let b = initial(vec![0x7e, 0, 0x41], 101, 1).unwrap();
        let NGAP_PDU::initiatingMessage(outer) = NGAP_PDU::decode(&b).unwrap() else {
            panic!()
        };
        let m: InitialUEMessage = decode_open_type(&outer.value).unwrap();
        let ie = m.protocol_ies.0.iter().find(|ie| ie.id.0 == 90).unwrap();
        assert_eq!(
            decode_open_type::<RRCEstablishmentCause>(&ie.value).unwrap(),
            RRCEstablishmentCause::mo_Signalling
        );
        assert!(!m.protocol_ies.0.iter().any(|ie| ie.id.0 == 237));
    }
    #[test]
    fn aper_round_trip() {
        let nas = crate::protocol::registration_request(9).unwrap();
        let encoded = initial(nas.clone(), 9, 1).unwrap();
        let (kind, decoded) = unwrap(&encoded, 9).unwrap();
        assert_eq!(kind, "InitialUEMessage");
        assert_eq!(decoded, nas);
        assert!(unwrap(&encoded, 10).is_err());
        let reject = crate::protocol::registration_reject().unwrap();
        assert_eq!(
            unwrap(&downlink(reject.clone(), 9).unwrap(), 9).unwrap().1,
            reject
        );
        assert!(unwrap(&[0xff], 9).is_err());
    }
}
