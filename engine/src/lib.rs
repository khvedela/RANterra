pub mod experiment;
pub mod research;
pub mod session;

pub mod auth;
pub mod model;
pub mod ngap;
pub mod protocol;
pub mod radio;
pub mod registration;
pub mod simulation;
pub mod transport;

#[cfg(target_arch = "wasm32")]
mod wasm {
    use crate::experiment::{Action, Experiment};
    use wasm_bindgen::prelude::*;
    #[wasm_bindgen]
    pub struct BrowserEngine {
        inner: Experiment,
    }
    #[wasm_bindgen]
    impl BrowserEngine {
        #[wasm_bindgen(constructor)]
        pub fn new(json: &str) -> Result<BrowserEngine, JsValue> {
            Ok(Self {
                inner: Experiment::from_json(json).map_err(|e| JsValue::from_str(&e))?,
            })
        }
        pub fn recording(&self) -> Result<String, JsValue> {
            serde_json::to_string(&self.inner.recording().map_err(|e| JsValue::from_str(&e))?)
                .map_err(|e| JsValue::from_str(&e.to_string()))
        }
        pub fn configuration(&self) -> Result<String, JsValue> {
            serde_json::to_string(self.inner.configuration())
                .map_err(|e| JsValue::from_str(&e.to_string()))
        }
        pub fn advance(&mut self, ms: u32) -> Result<(), JsValue> {
            self.inner
                .advance(ms as u64)
                .map_err(|e| JsValue::from_str(&e))
        }
        pub fn session(&mut self, id: u32, action: &str, loss: u8) -> Result<(), JsValue> {
            match action {
                "establish" => self.inner.execute(Action::Establish { ue_id: id }),
                "deregister" => self.inner.execute(Action::Deregister { ue_id: id }),
                "release" => self.inner.execute(Action::Release { ue_id: id }),
                "echo" => self.inner.execute(Action::Echo {
                    ue_id: id,
                    loss_percent: loss,
                }),
                _ => Err("Unknown session action".into()),
            }
            .map_err(|e| JsValue::from_str(&e))
        }
        pub fn snapshot(&self) -> Result<String, JsValue> {
            serde_json::to_string(&self.inner.snapshot())
                .map_err(|e| JsValue::from_str(&e.to_string()))
        }
    }
}
