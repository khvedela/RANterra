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
        REJECT RAN_UE_NGAP_ID(ue),REJECT NAS_PDU(nas),REJECT UserLocationInformation(location),IGNORE RRCEstablishmentCause(RRCEstablishmentCause::mo_Signalling),
    );
    pdu.encode().map_err(|e| e.to_string())
}
pub fn downlink(nas: Vec<u8>, ue: u32) -> Result<Vec<u8>, String> {
    let pdu = build_ngap!(InitiatingMessage,DownlinkNASTransport,IGNORE,DownlinkNASTransport,
        REJECT AMF_UE_NGAP_ID(ue as u64),REJECT RAN_UE_NGAP_ID(ue),REJECT NAS_PDU(nas),
    );
    pdu.encode().map_err(|e| e.to_string())
}
fn initial_nas(msg: &InitialUEMessage) -> Result<(u32, Vec<u8>), MissingIeError> {
    extract_ngap_ies!(msg,InitialUEMessage,req ue:u32=RAN_UE_NGAP_ID(id),req nas:Vec<u8>=NAS_PDU(pdu)=>pdu.0.to_vec(),);
    Ok((ue, nas))
}
fn downlink_nas(msg: &DownlinkNASTransport) -> Result<(u32, Vec<u8>), MissingIeError> {
    extract_ngap_ies!(msg,DownlinkNASTransport,req ue:u32=RAN_UE_NGAP_ID(id),req nas:Vec<u8>=NAS_PDU(pdu)=>pdu.0.to_vec(),);
    Ok((ue, nas))
}
pub fn unwrap(bytes: &[u8], expected_ue: u32) -> Result<(String, Vec<u8>), String> {
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
        _ => return Err("Unsupported NGAP procedure".into()),
    };
    if ue != expected_ue {
        return Err("NGAP UE identifier mismatch".into());
    }
    Ok((name, nas))
}
#[cfg(test)]
mod tests {
    use super::*;
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
