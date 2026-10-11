//! Generate a current-profile service-policy experiment.
//! The checked-in profile-2 recording remains a legacy compatibility fixture.
use ranterra_engine::{
    experiment::{Action, Experiment},
    model::{Scenario, SessionRequest, Snssai},
};
fn main() -> Result<(), String> {
    let mut s: Scenario =
        serde_json::from_str(include_str!("../../examples/minimal.json")).unwrap();
    s.name = "Service policy, backoff and replay".into();
    let service = SessionRequest {
        dnn: "research.example".into(),
        snssai: Snssai {
            sst: 2,
            sd: Some([1, 2, 3]),
        },
    };
    s.session_policies[0].request = service.clone();
    s.session_policies[0].max_sessions = 1;
    s.ues[0].session = service;
    let mut second = s.ues[0].clone();
    second.id = 102;
    second.name = "Second lab UE".into();
    s.ues.push(second);
    let mut run = Experiment::new(s)?;
    run.advance(1000)?;
    run.execute(Action::Establish { ue_id: 101 })?;
    run.execute(Action::Establish { ue_id: 102 })?;
    run.advance(100)?;
    run.execute(Action::Echo {
        ue_id: 101,
        loss_percent: 0,
    })?;
    run.advance(100)?;
    run.execute(Action::Release { ue_id: 101 })?;
    run.advance(9900)?;
    run.execute(Action::Establish { ue_id: 102 })?;
    run.advance(100)?;
    run.execute(Action::Echo {
        ue_id: 102,
        loss_percent: 100,
    })?;
    run.execute(Action::Echo {
        ue_id: 102,
        loss_percent: 0,
    })?;
    run.advance(100)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&run.recording()?).unwrap()
    );
    Ok(())
}
