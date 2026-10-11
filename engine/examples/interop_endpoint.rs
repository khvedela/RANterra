//! Bounded stdin/stdout adapter for an independent test peer; never a network service.
//! Calls the application's actual endpoint/security and session codecs; contains no crypto.
use ranterra_engine::{
    model::{LabFault, SessionPolicy, SessionRequest},
    ngap,
    registration::Registration,
    session,
    smf::{Establishment, Smf},
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    io::{self, BufRead, Write},
};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Command {
    op: String,
    id: u32,
    #[serde(default)]
    bytes: Vec<u8>,
    #[serde(default)]
    uplink: bool,
    #[serde(default)]
    now: u64,
    #[serde(default)]
    sqn: u64,
    #[serde(default)]
    policies: Option<Vec<SessionPolicy>>,
}
fn handle(
    c: Command,
    actors: &mut BTreeMap<u32, Registration>,
    smf: &mut Smf,
) -> Result<Value, String> {
    if c.id == 0 || c.bytes.len() > 65536 {
        return Err("Adapter input bounds".into());
    }
    if c.op == "configure" {
        if !actors.is_empty() {
            return Err("Configure before provisioning".into());
        }
        *smf = Smf::new(c.policies.ok_or("Missing policies")?)?;
        return Ok(json!({}));
    }
    if c.op == "advance" {
        let mut out = vec![];
        for event in smf.advance(c.now)? {
            let wire = event
                .n1
                .map(|n1| {
                    actors
                        .get_mut(&event.ue_id)
                        .ok_or("Unknown actor")?
                        .protect_session(n1, false)
                })
                .transpose()?;
            out.push(json!({"id":event.ue_id,"time_ms":event.time_ms,"bytes":wire,"reason":event.reason}));
        }
        return Ok(json!({"events":out}));
    }
    if c.op == "new" {
        if actors.len() >= 128 || actors.contains_key(&c.id) {
            return Err("Adapter actor limit/duplicate".into());
        }
        actors.insert(
            c.id,
            Registration::new(c.id, 42, LabFault::None)?
                .with_slices(smf.allowed_slices())
                .with_usim_sqn(c.sqn)?,
        );
        return Ok(json!({}));
    }
    let r = actors.get_mut(&c.id).ok_or("Unknown actor")?;
    match c.op.as_str() {
        "registration_timers" => {
            let events=r.advance(c.now)?.into_iter().map(|e| Ok(json!({"time_ms":e.time_ms,"name":e.name,"unconfirmed":e.unconfirmed,"bytes":e.wire.map(|b|ngap::downlink(b,c.id)).transpose()?}))).collect::<Result<Vec<_>,String>>()?;
            Ok(json!({"events":events}))
        }
        "setup_failed" => {
            let sm = smf.setup_failed(c.id, &c.bytes, c.now)?;
            Ok(
                json!({"bytes":r.protect_session(session::n1(sm,false,false,&SessionRequest::default())?,false)?,"failure":smf.failure(c.id)}),
            )
        }
        "amf" => {
            let (_, nas) = ngap::unwrap(&c.bytes, c.id)?;
            let received = r.at_amf_at(&nas, c.now)?;
            Ok(
                json!({"bytes":received.reply.map(|b|ngap::downlink(b,c.id)).transpose()?,"registered":received.registered}),
            )
        }
        "ue" => {
            let (_, nas) = ngap::unwrap(&c.bytes, c.id)?;
            let received = r.at_ue(&nas)?;
            Ok(json!({"bytes":received.reply.map(|b|ngap::uplink(b,c.id,1)).transpose()?}))
        }
        "receive" => Ok(json!({"bytes":r.receive_session(&c.bytes,c.uplink)?})),
        "protect" => Ok(json!({"bytes":r.protect_session(c.bytes,c.uplink)?})),
        "setup" => {
            let plain = r.receive_session(&c.bytes, true)?;
            match smf.establish(c.id, &plain, c.now)? {
                Establishment::Setup { session: s, n1 } => {
                    let protected = r.protect_session(n1, false)?;
                    let slice = ranterra_engine::model::Snssai {
                        sst: s.sst,
                        sd: s.sd,
                    };
                    Ok(
                        json!({"bytes":session::setup(c.id,s.uplink_teid,protected,&slice)?,"session":s}),
                    )
                }
                Establishment::Reject { n1 } => {
                    Ok(json!({"bytes":r.protect_session(n1,false)?,"failure":smf.failure(c.id)}))
                }
            }
        }
        "setup_response" => {
            smf.setup_response(c.id, &c.bytes)?;
            Ok(json!({"session":smf.session(c.id)}))
        }
        "release" => {
            let plain = r.receive_session(&c.bytes, true)?;
            let sm = session::unwrap_n1(&plain, true)?;
            let command = smf.release(c.id, &sm, c.now)?;
            let n1 = session::n1(command, false, false, &SessionRequest::default())?;
            Ok(json!({"bytes":r.protect_session(n1,false)?,"n2":session::release_n2(c.id)?[0]}))
        }
        "release_response" => {
            smf.release_response(c.id, &c.bytes)?;
            Ok(json!({}))
        }
        "release_complete" => {
            let n1 = r.receive_session(&c.bytes, true)?;
            smf.release_complete(c.id, &session::unwrap_n1(&n1, true)?)?;
            Ok(json!({}))
        }
        "state" => Ok(json!({"session":smf.session(c.id),"failure":smf.failure(c.id)})),
        "echo" => Ok(json!({"bytes":smf.echo(c.id,&c.bytes)?})),
        _ => Err("Unknown adapter operation".into()),
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut actors = BTreeMap::new();
    let mut smf = Smf::new(vec![SessionPolicy {
        request: SessionRequest::default(),
        max_sessions: 128,
    }])?;
    let mut input = io::stdin().lock();
    // Bound before allocation; unlike lines(), read at most one capped command.
    loop {
        let mut line = Vec::new();
        let n = std::io::Read::take(&mut input, 300_001).read_until(b'\n', &mut line)?;
        if n == 0 {
            break;
        }
        if line.len() > 300_000 || !line.ends_with(b"\n") {
            return Err("Adapter command exceeds bound".into());
        }
        let response = serde_json::from_slice::<Command>(&line)
            .map_err(|e| e.to_string())
            .and_then(|c| handle(c, &mut actors, &mut smf));
        println!(
            "{}",
            match response {
                Ok(v) => v,
                Err(e) => json!({"error":e}),
            }
        );
        io::stdout().flush()?;
    }
    Ok(())
}
