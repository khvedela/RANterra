//! Emit decrypted endpoint NAS and actual NGAP for an independent codec check.
use ranterra_engine::{
    model::{LabFault, SessionRequest, Snssai},
    ngap, protocol,
    registration::Registration,
    session,
};
use serde_json::{json, Value};
fn add(out: &mut Vec<Value>, protocol: &str, name: &str, b: &[u8]) {
    out.push(json!({"protocol":protocol,"name":name,"hex":b.iter().map(|b|format!("{b:02x}")).collect::<String>()}));
}
fn main() -> Result<(), String> {
    let request = SessionRequest {
        dnn: "research.example".into(),
        snssai: Snssai {
            sst: 2,
            sd: Some([1, 2, 3]),
        },
    };
    let mut out = vec![];
    let mut r =
        Registration::new(101, 42, LabFault::None)?.with_slices(vec![request.snssai.clone()]);
    let mut wire = protocol::registration_request(101)?;
    let mut uplink = true;
    for i in 0..10 {
        let n2 = if i == 0 {
            ngap::initial(wire.clone(), 101, 1)?
        } else if uplink {
            ngap::uplink(wire.clone(), 101, 1)?
        } else {
            ngap::downlink(wire.clone(), 101)?
        };
        add(&mut out, "NGAP", &format!("registration-{i}"), &n2);
        let received = if uplink {
            r.at_amf(&wire)?
        } else {
            r.at_ue(&wire)?
        };
        add(
            &mut out,
            "NAS",
            &format!("registration-{i}"),
            &received.plain,
        );
        match received.reply {
            Some(reply) => wire = reply,
            None => break,
        }
        uplink = !uplink;
    }
    let plain = session::n1(session::request()?, true, true, &request)?;
    let wire = r.protect_session(plain, true)?;
    add(
        &mut out,
        "NAS",
        "UL-service-selection",
        &r.receive_session(&wire, true)?,
    );
    add(
        &mut out,
        "NGAP",
        "UL-service-selection",
        &ngap::uplink(wire, 101, 1)?,
    );
    let sm = session::accept([10, 45, 0, 1], &request)?;
    add(&mut out, "NAS", "PDU-session-accept", &sm);
    let plain = session::n1(sm, false, false, &request)?;
    let wire = r.protect_session(plain, false)?;
    add(
        &mut out,
        "NAS",
        "DL-session-accept",
        &r.receive_session(&wire, false)?,
    );
    add(
        &mut out,
        "NGAP",
        "PDU-resource-setup",
        &session::setup(101, 2, wire, &request.snssai)?,
    );
    add(
        &mut out,
        "NGAP",
        "PDU-resource-response",
        &session::setup_response(101, 3)?,
    );
    for sm in [
        session::request()?,
        session::nas(vec![0x2e, 1, 1, 0xc3, 26, 0x37, 1, 0x65])?,
        session::nas(vec![0x2e, 1, 1, 0xd1])?,
        session::nas(vec![0x2e, 1, 1, 0xd3, 36])?,
        session::nas(vec![0x2e, 1, 1, 0xd4])?,
    ] {
        add(&mut out, "NAS", "5GSM", &sm);
    }
    let mut denied = session::n1(session::request()?, false, false, &request)?;
    denied.extend([0x58, 91]);
    add(
        &mut out,
        "NAS",
        "DL-unforwarded-cause91",
        &session::nas(denied)?,
    );
    for b in session::release_n2(101)? {
        add(&mut out, "NGAP", "PDU-resource-release", &b);
    }
    for b in ngap::release_context(101)? {
        add(&mut out, "NGAP", "UE-context-release", &b);
    }
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
    Ok(())
}
