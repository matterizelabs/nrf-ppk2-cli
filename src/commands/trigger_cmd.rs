use crate::autosave::Autosave;
use crate::config::Config;
use crate::device::Ppk2Device;
use crate::error::{Error, Result};
use crate::transport::resolve_port;
use crate::trigger::{TriggerConfig, TriggerEdge, TriggerEngine, TriggerState};
use crate::types::MeasurementStats;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

#[allow(clippy::too_many_arguments)]
pub fn run(
    json: bool,
    threshold_ua: f64,
    edge: &str,
    pre_trigger_ms: u64,
    post_trigger_ms: u64,
    save: Option<&str>,
    port: Option<&str>,
    serial: Option<&str>,
) -> Result<()> {
    let edge = match edge {
        "rising" => TriggerEdge::Rising,
        "falling" => TriggerEdge::Falling,
        "both" => TriggerEdge::Both,
        _ => return Err(Error::InvalidArg(format!("unknown edge: {}", edge))),
    };

    let config = Config::load()?;
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

    let trigger_config = TriggerConfig {
        threshold_ua,
        edge,
        pre_trigger_ms,
        post_trigger_ms,
    };
    let mut engine = TriggerEngine::new(trigger_config);

    let mut autosave = if save.is_some() || config.autosave.enabled {
        Some(Autosave::new(&serial_str, &config.autosave)?)
    } else {
        None
    };

    let mut parser = crate::parser::SampleParser::new();
    let start = Instant::now();
    let mut count: u64 = 0;
    let mut sum: f64 = 0.0;
    let mut min: f64 = f64::MAX;
    let mut max: f64 = f64::MIN;
    let mut triggered = false;
    let mut saved_path: Option<String> = None;

    loop {
        if !running.load(Ordering::SeqCst) {
            break;
        }
        if engine.state() == TriggerState::Done {
            triggered = true;
            break;
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
                    count += 1;
                    sum += ua;
                    if ua < min {
                        min = ua;
                    }
                    if ua > max {
                        max = ua;
                    }
                    engine.feed(ua, sample.logic);

                    if engine.state() == TriggerState::Done
                        || engine.state() == TriggerState::Collecting
                    {
                        if let Some(ref mut asv) = autosave {
                            asv.push((ua as f32, sample.logic))?;
                        }
                    }
                }
            }
            Ok(None) => continue,
            Err(Error::Disconnected(_)) => {
                let elapsed = start.elapsed().as_secs_f64();
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
    }

    device.stop_measurement()?;

    if auto_power == "session" {
        device.set_power(false)?;
    }

    if let Some(asv) = autosave.take() {
        saved_path = Some(asv.finalize(save)?);
    }

    let elapsed = start.elapsed().as_secs_f64();
    if !triggered && !running.load(Ordering::SeqCst) {
        eprintln!(
            "trigger: interrupted before trigger fired ({} samples)",
            count
        );
        return Ok(());
    }

    if !triggered {
        eprintln!("trigger: never fired ({} samples, {:.1}s)", count, elapsed);
        return Ok(());
    }

    let captured = engine.captured();
    let avg = if count > 0 { sum / count as f64 } else { 0.0 };
    let charge = avg * elapsed / 3600.0;

    let power = match device.current_mode() {
        crate::types::MeasurementMode::Source => Some(device.vdd_mv() as f64 * avg / 1000.0),
        _ => None,
    };

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
            "trigger fired at {} samples  captured {} samples  duration {:.1}s  avg {:.1}uA  power {}",
            engine.fired_at().unwrap_or(0),
            captured.len(),
            elapsed,
            avg,
            power_str,
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
