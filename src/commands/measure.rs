use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crate::autosave::Autosave;
use crate::config::Config;
use crate::device::Ppk2Device;
use crate::error::{Error, Result};
use crate::transport::resolve_port;
use crate::types::MeasurementStats;

pub fn run(
    json: bool,
    duration: Option<f64>,
    save: Option<&str>,
    rate: Option<u32>,
    port: Option<&str>,
    serial: Option<&str>,
) -> Result<()> {
    let config = Config::load()?;

    if let Some(d) = duration {
        if d <= 0.0 {
            return Err(Error::InvalidArg("duration must be positive".into()));
        }
    }

    let (port_path, serial_str) = resolve_port(port, serial)?;
    let mut device = Ppk2Device::open(&port_path)?;

    let auto_power = config.behavior.auto_power.as_str();
    match auto_power {
        "never" => {
            if !device.is_power_on() {
                return Err(Error::PowerNotOn);
            }
        }
        _ => {
            if !device.is_power_on() {
                device.set_power(true)?;
            }
        }
    }

    device.start_measurement()?;

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })?;

    let start = Instant::now();
    let end_time = duration.map(|d| start + std::time::Duration::from_secs_f64(d));

    let mut downsampler = rate.map(crate::downsample::Downsampler::new);
    let sample_rate = downsampler
        .as_ref()
        .map(|d| d.actual_rate())
        .unwrap_or(100_000);

    let mut autosave = if save.is_some() || config.autosave.enabled {
        Some(Autosave::new(&serial_str, &config.autosave, sample_rate)?)
    } else {
        None
    };

    let mut count: u64 = 0;
    let mut sum: f64 = 0.0;
    let mut min: f64 = f64::MAX;
    let mut max: f64 = f64::MIN;
    let mut saved_path: Option<String> = None;

    let mut parser = crate::parser::SampleParser::new();
    let mut last_report = Instant::now();
    let report_interval = std::time::Duration::from_millis(500);

    loop {
        if !running.load(Ordering::SeqCst) {
            break;
        }
        if let Some(et) = end_time {
            if Instant::now() >= et {
                break;
            }
        }

        match device.read_sample_raw() {
            Ok(Some(raw)) => {
                let samples = parser.feed(&[raw]);
                for sample in samples {
                    let ua = match device.convert_sample(&sample) {
                        Ok(v) => v,
                        Err(e) => {
                            eprintln!("conversion error: {}", e);
                            continue;
                        }
                    };
                    if let Some(ref mut ds) = downsampler {
                        if let Some((avg_ua, bits)) = ds.feed(ua, sample.logic) {
                            count += 1;
                            sum += avg_ua;
                            if avg_ua < min {
                                min = avg_ua;
                            }
                            if avg_ua > max {
                                max = avg_ua;
                            }
                            if let Some(ref mut asv) = autosave {
                                asv.push((avg_ua as f32, bits))?;
                            }
                        }
                    } else {
                        count += 1;
                        sum += ua;
                        if ua < min {
                            min = ua;
                        }
                        if ua > max {
                            max = ua;
                        }
                        if let Some(ref mut asv) = autosave {
                            asv.push((ua as f32, sample.logic))?;
                        }
                    }
                }
            }
            Ok(None) => continue,
            Err(Error::Disconnected(e)) => {
                let elapsed = if e == 0.0 {
                    start.elapsed().as_secs_f64()
                } else {
                    e
                };
                if let Some(asv) = autosave.take() {
                    let _ = asv.finalize(save)?;
                }
                return Err(Error::PartialCapture {
                    samples: count,
                    duration: elapsed,
                });
            }
            Err(e) => return Err(e),
        }

        if !json && last_report.elapsed() >= report_interval {
            let elapsed = start.elapsed().as_secs_f64();
            let avg = if count > 0 { sum / count as f64 } else { 0.0 };
            let (v, unit) = format_current(avg);
            let mut line = format!("{:.1}s  avg {:.1}{}", elapsed, v, unit);
            if let crate::types::MeasurementMode::Source = device.current_mode() {
                let pw = device.vdd_mv() as f64 * avg / 1000.0;
                let (pv, punit) = format_power(pw);
                line.push_str(&format!("  {:.1}{}", pv, punit));
            }
            eprint!("\x1b[2K\r{}", line);
            let _ = std::io::stderr().flush();
            last_report = Instant::now();
        }
    }

    if let Some(ref ds) = downsampler {
        if let Some((avg_ua, bits)) = ds.flush() {
            count += 1;
            sum += avg_ua;
            if avg_ua < min {
                min = avg_ua;
            }
            if avg_ua > max {
                max = avg_ua;
            }
            if let Some(ref mut asv) = autosave {
                asv.push((avg_ua as f32, bits))?;
            }
        }
    }

    device.stop_measurement()?;

    if auto_power == "session" {
        device.set_power(false)?;
    }

    let elapsed = start.elapsed().as_secs_f64();
    let avg = if count > 0 { sum / count as f64 } else { 0.0 };
    let charge = avg * elapsed / 3600.0;

    let power = match device.current_mode() {
        crate::types::MeasurementMode::Source => Some(device.vdd_mv() as f64 * avg / 1000.0),
        _ => None,
    };

    if device.current_mode() == crate::types::MeasurementMode::Source && avg > 580_000.0_f64 {
        eprintln!(
            "warning: current {:.0}uA approaching source mode limit (600mA), consider switching to ampere mode with external supply",
            avg
        );
    }
    if device.current_mode() == crate::types::MeasurementMode::Source && avg > 400_000.0_f64 {
        eprintln!(
            "warning: high current detected, connect both USB ports for reliable operation above 400mA"
        );
    }

    if let Some(asv) = autosave.take() {
        saved_path = Some(asv.finalize(save)?);
    }

    if !json {
        eprint!("\x1b[2K\r");
    }

    if json {
        let stats = MeasurementStats {
            duration_s: elapsed,
            samples: count,
            avg_ua: avg,
            charge_uah: charge,
            power_uw: power,
            min_ua: if min == f64::MAX { 0.0 } else { min },
            max_ua: if max == f64::MIN { 0.0 } else { max },
        };
        println!("{}", stats.to_json());
        if let Some(ref p) = saved_path {
            eprintln!("saved {}", p);
        }
    } else {
        let power_str = power
            .map(|p| format!("{:.0}uW", p))
            .unwrap_or_else(|| "—".to_string());
        println!(
            "duration {:.1}s  samples {}  avg {:.1}uA  charge {:.3}uAh  power {}",
            elapsed, count, avg, charge, power_str,
        );
        if let Some(ref p) = saved_path {
            println!("saved {}", p);
        }
    }

    if parser.lost_samples() > 0 {
        eprintln!(
            "warning: data loss: {} samples dropped",
            parser.lost_samples()
        );
    }

    Ok(())
}

fn format_current(ua: f64) -> (f64, &'static str) {
    if ua >= 1_000_000.0 {
        (ua / 1_000_000.0, "A")
    } else if ua >= 1000.0 {
        (ua / 1000.0, "mA")
    } else {
        (ua, "uA")
    }
}

fn format_power(uw: f64) -> (f64, &'static str) {
    if uw >= 1_000_000.0 {
        (uw / 1_000_000.0, "W")
    } else if uw >= 1000.0 {
        (uw / 1000.0, "mW")
    } else {
        (uw, "uW")
    }
}
