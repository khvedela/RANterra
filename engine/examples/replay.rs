use ranterra_engine::{model::Scenario, simulation::Engine};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "examples/minimal.json".into());
    let scenario: Scenario = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    let mut engine = Engine::new(scenario)?;
    engine.advance(1000)?;
    println!("{}", serde_json::to_string_pretty(&engine.snapshot())?);
    Ok(())
}
