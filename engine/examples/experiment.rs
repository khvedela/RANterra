//! Load a scenario or restore a portable recording. Optionally advance then save.
use ranterra_engine::experiment::Experiment;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let path = args
        .get(1)
        .ok_or("Usage: experiment INPUT [ADVANCE_MS] [OUTPUT_RECORDING]")?;
    let mut run = Experiment::from_json(&std::fs::read_to_string(path)?)?;
    let mut remaining: u64 = args.get(2).map(|s| s.parse()).transpose()?.unwrap_or(0);
    while remaining > 0 {
        let step = remaining.min(60000);
        run.advance(step)?;
        remaining -= step;
    }
    if let Some(path) = args.get(3) {
        std::fs::write(path, serde_json::to_string_pretty(&run.recording()?)?)?;
    }
    println!("{}", serde_json::to_string(&run.snapshot())?);
    Ok(())
}
