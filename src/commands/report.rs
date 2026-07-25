use crate::error::Result;
use crate::fileio;

pub fn run(json: bool, files: &[String]) -> Result<()> {
    for file in files {
        let (frames, rate) = fileio::read_ppk2(file)?;
        let count = frames.len() as u64;
        let sum: f64 = frames.iter().map(|(ua, _)| *ua as f64).sum();
        let avg = if count > 0 { sum / count as f64 } else { 0.0 };
        let duration = count as f64 / rate as f64;
        let charge = avg * duration / 3600.0;

        if json {
            println!(
                r#"{{"file":"{}","duration_s":{},"samples":{},"avg_ua":{},"charge_uah":{}}}"#,
                file, duration, count, avg, charge
            );
        } else {
            println!(
                "{}: {:.1}s  {} samples  avg {:.1}uA  charge {:.3}uAh",
                file, duration, count, avg, charge
            );
        }
    }
    Ok(())
}
