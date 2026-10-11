//! Real TS 24.501 NAS codecs. Identities are synthetic lab-only SUCIs (null scheme).
use oxirush_nas::nas_5gs::{ie::*, messages::*, *};

pub fn registration_request(id: u32) -> std::result::Result<Vec<u8>, String> {
    // Ten decimal MSIN digits encoded as TBCD, unique within the scenario.
    let digits = format!("{:010}", id);
    let msin = digits
        .as_bytes()
        .chunks(2)
        .map(|p| (p[0] - b'0') | ((p[1] - b'0') << 4))
        .collect();
    let identity = NasFGsMobileIdentity::from_suci(&Suci::Imsi(ImsiSuci {
        plmn_id: PlmnId {
            mcc: [0, 0, 1],
            mnc: [0, 1, 0xf],
        },
        routing_indicator: vec![0xf0, 0xff],
        protection_scheme: ProtectionScheme::Null,
        home_nw_public_key_id: 0,
        scheme_output: msin,
    }));
    let request = NasRegistrationRequest::new(
        NasFGsRegistrationType::from_registration_type(RegistrationType::InitialRegistration)
            .with_ngksi(7),
        identity,
    )
    .set_ue_security_capability(NasUeSecurityCapability::from_capabilities(0xe0, 0xe0));
    let message = Nas5gsMessage::new_5gmm(Nas5gmmMessage::RegistrationRequest(request));
    if !message.validate().is_empty() {
        return Err(format!("NAS validation: {:?}", message.validate()));
    }
    encode_nas_5gs_message(&message).map_err(|e| e.to_string())
}

pub fn registration_reject() -> std::result::Result<Vec<u8>, String> {
    encode_nas_5gs_message(&Nas5gsMessage::new_5gmm(
        Nas5gmmMessage::RegistrationReject(NasRegistrationReject::new(NasFGmmCause::from_cause(
            GmmCause::FiveGSServicesNotAllowed,
        ))),
    ))
    .map_err(|e| e.to_string())
}

pub fn inspect(bytes: &[u8]) -> std::result::Result<(String, String), String> {
    let message = decode_nas_5gs_message(bytes).map_err(|e| e.to_string())?;
    let kind = match &message {
        Nas5gsMessage::Gmm(_, Nas5gmmMessage::RegistrationRequest(_)) => "Registration Request",
        Nas5gsMessage::Gmm(_, Nas5gmmMessage::RegistrationReject(_)) => "Registration Reject",
        _ => return Err("Unsupported NAS message in this procedure".into()),
    };
    Ok((kind.into(), message.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn request_round_trip_and_wire_header() {
        let bytes = registration_request(1).unwrap();
        assert_eq!(&bytes[..4], &[0x7e, 0, 0x41, 0x71]);
        let decoded = decode_nas_5gs_message(&bytes).unwrap();
        assert!(decoded.validate().is_empty());
        assert_eq!(encode_nas_5gs_message(&decoded).unwrap(), bytes);
        assert!(inspect(&bytes).unwrap().1.contains("SUCI"));
        assert_ne!(bytes, registration_request(2).unwrap());
    }
    #[test]
    fn reject_known_wire_vector() {
        assert_eq!(registration_reject().unwrap(), vec![0x7e, 0, 0x44, 7]);
        assert_eq!(
            inspect(&[0x7e, 0, 0x44, 7]).unwrap().0,
            "Registration Reject"
        );
    }
    #[test]
    fn malformed_nas_rejected() {
        for bytes in [&[][..], &[0x7e], &[0x7e, 0, 0x41], &[0xff, 0, 0x41]] {
            assert!(inspect(bytes).is_err());
        }
    }
}
