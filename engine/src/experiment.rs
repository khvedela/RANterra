//! Portable command replay, never serialized security contexts.
use crate::{
    model::Scenario,
    simulation::{Engine, Snapshot},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
pub const PROFILE: &str = "ranterra-lab/3";
pub const MAX_COMMANDS: usize = 4096;
pub const MAX_TIME_MS: u64 = 3_600_000;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Establish { ue_id: u32 },
    Release { ue_id: u32 },
    Echo { ue_id: u32, loss_percent: u8 },
    Deregister { ue_id: u32 },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub time_ms: u64,
    pub action: Action,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recording {
    pub format: String,
    pub version: u32,
    pub engine_profile: String,
    pub scenario: Scenario,
    pub commands: Vec<Command>,
    pub time_ms: u64,
    pub started: bool,
    pub state_digest: String,
}
/// The application and portable native runner use this boundary for all mutations.
pub struct Experiment {
    engine: Engine,
    profile: String,
    commands: Vec<Command>,
}
impl Experiment {
    pub fn new(scenario: Scenario) -> Result<Self, String> {
        Ok(Self {
            engine: Engine::new(scenario)?,
            profile: PROFILE.into(),
            commands: vec![],
        })
    }
    pub fn configuration(&self) -> &Scenario {
        &self.engine.scenario
    }
    pub fn snapshot(&self) -> Snapshot {
        self.engine.snapshot()
    }
    pub fn advance(&mut self, ms: u64) -> Result<(), String> {
        if self
            .engine
            .time_ms
            .checked_add(ms)
            .is_none_or(|t| t > MAX_TIME_MS)
        {
            return Err("Experiment limit is 3600000 virtual ms".into());
        }
        self.engine.advance(ms)
    }
    pub fn execute(&mut self, action: Action) -> Result<(), String> {
        if self.commands.len() >= MAX_COMMANDS {
            return Err("Command journal limit (4096); export and start a new experiment".into());
        }
        let before = self.recording()?;
        let result = match action {
            Action::Establish { ue_id } => self.engine.establish_session(ue_id),
            Action::Release { ue_id } => self.engine.release_session(ue_id),
            Action::Echo {
                ue_id,
                loss_percent,
            } => self.engine.echo(ue_id, loss_percent),
            Action::Deregister { ue_id } => self.engine.deregister(ue_id),
        };
        if let Err(error) = result {
            // Unexpected codec errors must not leave consumed NAS counters or partial sessions.
            *self = Self::replay(before)?;
            return Err(error);
        }
        self.commands.push(Command {
            time_ms: self.engine.time_ms,
            action,
        });
        Ok(())
    }
    pub fn recording(&self) -> Result<Recording, String> {
        Ok(Recording {
            format: "ranterra-experiment".into(),
            version: 1,
            engine_profile: self.profile.clone(),
            scenario: self.engine.scenario.clone(),
            commands: self.commands.clone(),
            time_ms: self.engine.time_ms,
            started: self.engine.snapshot().started,
            state_digest: self.digest()?,
        })
    }
    fn digest(&self) -> Result<String, String> {
        let s = self.snapshot();
        // Exclude floating-point radio observations, whose last bits may differ by runtime.
        // Protocol events, virtual time, identity states and session counters are exact.
        let states: Vec<_> = s
            .ues
            .iter()
            .map(|u| {
                (
                    &u.id,
                    &u.state,
                    &u.serving_cell,
                    &u.failure,
                    &u.registration_latency_ms,
                )
            })
            .collect();
        let bytes = serde_json::to_vec(&(
            s.time_ms,
            s.started,
            s.pending,
            states,
            s.gnodebs,
            s.amf,
            s.events,
            s.sessions,
            s.session_failures,
        ))
        .map_err(|e| e.to_string())?;
        let mut hash = Sha256::new();
        hash.update(bytes);
        if !self.engine.legacy {
            // Profile 3 authenticates compatibility of the complete input history,
            // including future delayed actions; this is still not a digital signature.
            hash.update(
                serde_json::to_vec(&(&self.profile, &self.engine.scenario, &self.commands))
                    .map_err(|e| e.to_string())?,
            );
        }
        Ok(format!("{:x}", hash.finalize()))
    }
    pub fn replay(recording: Recording) -> Result<Self, String> {
        if recording.format != "ranterra-experiment"
            || recording.version != 1
            || ![PROFILE, "ranterra-lab/2"].contains(&recording.engine_profile.as_str())
        {
            return Err("Unsupported experiment format/version/engine profile".into());
        }
        if recording.time_ms > MAX_TIME_MS
            || recording.commands.len() > MAX_COMMANDS
            || recording.state_digest.len() != 64
            || !recording
                .state_digest
                .bytes()
                .all(|b| b.is_ascii_hexdigit())
        {
            return Err("Invalid experiment bounds or digest".into());
        }
        if !recording.started && (recording.time_ms != 0 || !recording.commands.is_empty()) {
            return Err("Unstarted experiment cannot contain commands or elapsed time".into());
        }
        let mut previous = 0;
        for c in &recording.commands {
            if c.time_ms < previous || c.time_ms > recording.time_ms {
                return Err("Command times must be ordered and within recorded duration".into());
            }
            previous = c.time_ms;
        }
        let mut out = Self::new(recording.scenario)?;
        out.engine.legacy = recording.engine_profile == "ranterra-lab/2";
        if out.engine.legacy
            && out
                .engine
                .scenario
                .ues
                .iter()
                .any(|u| u.signaling != Default::default())
        {
            return Err("Profile 2 does not support signaling configuration".into());
        }
        out.profile = recording.engine_profile;
        for c in recording.commands {
            out.advance_to(c.time_ms)?;
            // Replay only into the candidate. No recursive transactional execute/rollback.
            match c.action {
                Action::Establish { ue_id } => out.engine.establish_session(ue_id),
                Action::Release { ue_id } => out.engine.release_session(ue_id),
                Action::Echo {
                    ue_id,
                    loss_percent,
                } => out.engine.echo(ue_id, loss_percent),
                Action::Deregister { ue_id } => out.engine.deregister(ue_id),
            }
            .map_err(|e| {
                format!(
                    "Replay command {} at {} ms: {e}",
                    out.commands.len(),
                    c.time_ms
                )
            })?;
            out.commands.push(c);
        }
        if recording.started {
            out.advance_to(recording.time_ms)?;
        }
        if out.digest()? != recording.state_digest {
            return Err(
                "Replay state digest mismatch; recording altered or incompatible engine behavior"
                    .into(),
            );
        }
        Ok(out)
    }
    fn advance_to(&mut self, time: u64) -> Result<(), String> {
        while self.engine.time_ms < time {
            self.advance((time - self.engine.time_ms).min(60_000))?;
        }
        if !self.engine.snapshot().started {
            self.advance(0)?;
        }
        Ok(())
    }
    pub fn from_json(json: &str) -> Result<Self, String> {
        if json.len() > 4_000_000 {
            return Err("Experiment input exceeds 4 MB".into());
        }
        let value: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
        if value.get("format").is_some() {
            Self::replay(serde_json::from_value(value).map_err(|e| e.to_string())?)
        } else {
            if json.len() > 2_000_000 {
                return Err("Scenario exceeds 2 MB".into());
            }
            Self::new(serde_json::from_value(value).map_err(|e| e.to_string())?)
        }
    }
}
