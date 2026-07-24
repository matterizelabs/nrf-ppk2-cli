use crate::error::Result;
use crate::fileio;

pub fn run(json: bool, file: &str) -> Result<()> {
    let frames = fileio::read_ppk2(file)?;

    let count = frames.len() as u64;
    let sum: f64 = frames.iter().map(|(ua, _)| *ua as f64).sum();
    let avg = if count > 0 { sum / count as f64 } else { 0.0 };
    let min = frames
        .iter()
        .map(|(ua, _)| *ua)
        .fold(f32::MAX, |a, b| a.min(b));
    let max = frames
        .iter()
        .map(|(ua, _)| *ua)
        .fold(f32::MIN, |a, b| a.max(b));
    let duration = count as f64 / 100_000.0;
    let charge = avg * duration / 3600.0;

    if json {
        println!(
            r#"{{"duration_s":{},"samples":{},"avg_ua":{:.3},"charge_uah":{:.6},"power_uw":null,"min_ua":{:.3},"max_ua":{:.3}}}"#,
            duration, count, avg, charge, min, max,
        );
    } else {
        println!(
            "duration {:.1}s  samples {}  avg {:.1}uA  charge {:.3}uAh",
            duration, count, avg, charge,
        );
    }

    Ok(())
}
