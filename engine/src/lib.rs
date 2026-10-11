pub mod model;
pub mod ngap;
pub mod protocol;
pub mod radio;
pub mod simulation;
pub mod transport;

#[cfg(target_arch = "wasm32")]
mod wasm {
    use crate::{model::Scenario, simulation::Engine};
    use wasm_bindgen::prelude::*;
    #[wasm_bindgen]
    pub struct BrowserEngine {
        inner: Engine,
    }
    #[wasm_bindgen]
    impl BrowserEngine {
        #[wasm_bindgen(constructor)]
        pub fn new(json: &str) -> Result<BrowserEngine, JsValue> {
            if json.len() > 2_000_000 {
                return Err(JsValue::from_str("Scenario exceeds 2 MB"));
            }
            let scenario: Scenario =
                serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
            Ok(Self {
                inner: Engine::new(scenario).map_err(|e| JsValue::from_str(&e))?,
            })
        }
        pub fn advance(&mut self, ms: u32) -> Result<(), JsValue> {
            self.inner
                .advance(ms as u64)
                .map_err(|e| JsValue::from_str(&e))
        }
        pub fn snapshot(&self) -> Result<String, JsValue> {
            serde_json::to_string(&self.inner.snapshot())
                .map_err(|e| JsValue::from_str(&e.to_string()))
        }
    }
}
