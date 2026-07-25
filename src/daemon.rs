#[cfg(unix)]
mod unix {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use crate::autosave::Autosave;
    use crate::config::Config;
    use crate::device::Ppk2Device;
    use crate::error::Result;
    use serde::Deserialize;

    struct SharedState {
        running: AtomicBool,
        stats: Mutex<DaemonStats>,
        save_path: Mutex<Option<String>>,
    }

    struct DaemonStats {
        count: u64,
        sum: f64,
        min: f64,
        max: f64,
        start: Instant,
    }

    #[derive(Deserialize)]
    struct DaemonCommand {
        cmd: String,
        #[serde(default)]
        save: Option<String>,
    }

    pub fn socket_path(serial: &str) -> PathBuf {
        Config::state_dir().join(serial).join("daemon.sock")
    }

    pub fn run_daemon(port_path: &str, serial: &str, rate: Option<u32>) -> Result<()> {
        let sock_path = socket_path(serial);
        if let Some(parent) = sock_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let _ = std::fs::remove_file(&sock_path);
        let listener = UnixListener::bind(&sock_path)?;
        println!("{}", sock_path.display());
        println!("{}", std::process::id());

        let state = Arc::new(SharedState {
            running: AtomicBool::new(true),
            stats: Mutex::new(DaemonStats {
                count: 0,
                sum: 0.0,
                min: f64::MAX,
                max: f64::MIN,
                start: Instant::now(),
            }),
            save_path: Mutex::new(None),
        });

        let state_m = Arc::clone(&state);
        let port = port_path.to_string();
        let sn = serial.to_string();
        let handle = std::thread::spawn(move || {
            if let Err(e) = measure_loop(&port, &sn, &state_m, rate) {
                eprintln!("daemon measure error: {}", e);
            }
            state_m.running.store(false, Ordering::SeqCst);
        });

        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let mut reader = BufReader::new(&mut stream);
                    let mut line = String::new();
                    if reader.read_line(&mut line).is_err() {
                        continue;
                    }
                    let line = line.trim().to_string();

                    let response = handle_command(&line, &state);
                    let _ = stream.write_all(response.as_bytes());
                    let _ = stream.write_all(b"\n");
                    let _ = stream.flush();

                    if line.contains("\"stop\"") {
                        break;
                    }
                }
                Err(_) => break,
            }
        }

        // Graceful shutdown: wait up to 5s for measure thread
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if handle.is_finished() {
                break;
            }
            if Instant::now() >= deadline {
                eprintln!("warning: measurement thread did not stop within 5s, forcing exit");
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = handle.join();
        let _ = std::fs::remove_file(&sock_path);
        Ok(())
    }

    fn handle_command(line: &str, state: &SharedState) -> String {
        let cmd: DaemonCommand = match serde_json::from_str(line) {
            Ok(c) => c,
            Err(_) => return r#"{"error":"invalid command"}"#.to_string(),
        };

        match cmd.cmd.as_str() {
            "status" => {
                let stats = state.stats.lock().unwrap();
                let elapsed = stats.start.elapsed().as_secs_f64();
                let avg = if stats.count > 0 {
                    stats.sum / stats.count as f64
                } else {
                    0.0
                };
                format!(
                    r#"{{"elapsed_s":{:.1},"samples":{},"avg_ua":{:.1},"min_ua":{:.1},"max_ua":{:.1}}}"#,
                    elapsed,
                    stats.count,
                    avg,
                    if stats.min == f64::MAX {
                        0.0
                    } else {
                        stats.min
                    },
                    if stats.max == f64::MIN {
                        0.0
                    } else {
                        stats.max
                    },
                )
            }
            "stop" => {
                if let Some(s) = &cmd.save {
                    *state.save_path.lock().unwrap() = Some(s.clone());
                }
                state.running.store(false, Ordering::SeqCst);
                r#"{"status":"stopping"}"#.to_string()
            }
            _ => r#"{"error":"unknown command"}"#.to_string(),
        }
    }

    fn measure_loop(
        port_path: &str,
        serial: &str,
        state: &SharedState,
        rate: Option<u32>,
    ) -> Result<()> {
        let config = Config::load()?;
        let mut device = Ppk2Device::open(port_path)?;

        let auto_power = config.behavior.auto_power.clone();
        match auto_power.as_str() {
            "never" => {
                if !device.is_power_on() {
                    device.set_power(true)?;
                }
            }
            _ => {
                if !device.is_power_on() {
                    device.set_power(true)?;
                }
            }
        }

        device.start_measurement()?;

        let mut downsampler = rate.map(crate::downsample::Downsampler::new);
        let sample_rate = downsampler
            .as_ref()
            .map(|d| d.actual_rate())
            .unwrap_or(100_000);

        let mut autosave = if config.autosave.enabled {
            Some(Autosave::new(serial, &config.autosave, sample_rate)?)
        } else {
            None
        };

        let mut parser = crate::parser::SampleParser::new();

        loop {
            if !state.running.load(Ordering::SeqCst) {
                break;
            }

            match device.read_sample_raw() {
                Ok(Some(raw)) => {
                    let samples = parser.feed(&[raw]);
                    let mut converted: Vec<(f64, u8)> = Vec::new();
                    for sample in samples {
                        match device.convert_sample(&sample) {
                            Ok(ua) => {
                                if let Some(ref mut ds) = downsampler {
                                    if let Some((avg_ua, bits)) = ds.feed(ua, sample.logic) {
                                        converted.push((avg_ua, bits));
                                    }
                                } else {
                                    converted.push((ua, sample.logic));
                                }
                            }
                            Err(e) => eprintln!("conversion error: {}", e),
                        }
                    }
                    if converted.is_empty() {
                        continue;
                    }
                    {
                        let mut stats = state.stats.lock().unwrap();
                        for &(ua, _) in &converted {
                            stats.count += 1;
                            stats.sum += ua;
                            if ua < stats.min {
                                stats.min = ua;
                            }
                            if ua > stats.max {
                                stats.max = ua;
                            }
                        }
                    }
                    if let Some(ref mut asv) = autosave {
                        for (ua, logic) in converted {
                            asv.push((ua as f32, logic))?;
                        }
                    }
                }
                Ok(None) => continue,
                Err(_) => break,
            }
        }

        if let Some(ref ds) = downsampler {
            if let Some((avg_ua, bits)) = ds.flush() {
                {
                    let mut stats = state.stats.lock().unwrap();
                    stats.count += 1;
                    stats.sum += avg_ua;
                    if avg_ua < stats.min {
                        stats.min = avg_ua;
                    }
                    if avg_ua > stats.max {
                        stats.max = avg_ua;
                    }
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

        let stats = state.stats.lock().unwrap();
        let elapsed = stats.start.elapsed().as_secs_f64();
        let avg = if stats.count > 0 {
            stats.sum / stats.count as f64
        } else {
            0.0
        };
        let charge = avg * elapsed / 3600.0;

        if let Some(asv) = autosave.take() {
            let save = state.save_path.lock().unwrap().clone();
            let path = asv.finalize(save.as_deref())?;
            eprintln!(
                "duration {:.1}s  samples {}  avg {:.1}uA  charge {:.3}uAh",
                elapsed, stats.count, avg, charge,
            );
            eprintln!("saved {}", path);
        } else {
            eprintln!(
                "duration {:.1}s  samples {}  avg {:.1}uA  charge {:.3}uAh",
                elapsed, stats.count, avg, charge,
            );
        }

        Ok(())
    }

    pub fn send_command(serial: &str, cmd: &str) -> Result<String> {
        let sock_path = socket_path(serial);
        let mut stream = UnixStream::connect(&sock_path)?;
        stream.write_all(cmd.as_bytes())?;
        stream.write_all(b"\n")?;
        stream.flush()?;
        let mut reader = BufReader::new(stream);
        let mut response = String::new();
        reader.read_line(&mut response)?;
        Ok(response.trim().to_string())
    }
}

#[cfg(windows)]
mod windows {
    use crate::error::Result;
    use std::path::PathBuf;

    pub fn socket_path(serial: &str) -> PathBuf {
        PathBuf::from(format!(r"\\.\pipe\ppk2-{}", serial))
    }

    pub fn run_daemon(_port_path: &str, serial: &str, _rate: Option<u32>) -> Result<()> {
        println!("{}", socket_path(serial).display());
        println!("{}", std::process::id());
        Ok(())
    }

    pub fn send_command(_serial: &str, _cmd: &str) -> Result<String> {
        Ok("{}".to_string())
    }
}

#[cfg(unix)]
pub use unix::*;

#[cfg(windows)]
pub use windows::*;
