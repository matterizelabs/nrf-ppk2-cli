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

    pub fn socket_path(serial: &str) -> PathBuf {
        Config::state_dir().join(serial).join("daemon.sock")
    }

    pub fn run_daemon(port_path: &str, serial: &str) -> Result<()> {
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
            if let Err(e) = measure_loop(&port, &sn, &state_m) {
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

    fn extract_json_val<'a>(line: &'a str, key: &str) -> Option<&'a str> {
        let pat = format!(r#""{}":"#, key);
        let start = line.find(&pat)? + pat.len();
        let rest = &line[start..];
        if let Some(stripped) = rest.strip_prefix('"') {
            let end = stripped.find('"')?;
            Some(&stripped[..end])
        } else {
            let end = rest.find(',').unwrap_or(rest.len());
            let end = rest[..end].find('}').unwrap_or(end);
            let val = rest[..end].trim();
            if val.is_empty() {
                None
            } else {
                Some(val)
            }
        }
    }

    fn parse_json_cmd(line: &str) -> (&str, Option<&str>) {
        let cmd = extract_json_val(line, "cmd").unwrap_or("");
        let save = extract_json_val(line, "save");
        (cmd, save)
    }

    fn handle_command(line: &str, state: &SharedState) -> String {
        let (cmd, save) = parse_json_cmd(line);

        match cmd {
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
                if let Some(s) = save {
                    *state.save_path.lock().unwrap() = Some(s.to_string());
                }
                state.running.store(false, Ordering::SeqCst);
                r#"{"status":"stopping"}"#.to_string()
            }
            _ => r#"{"error":"unknown command"}"#.to_string(),
        }
    }

    fn measure_loop(port_path: &str, serial: &str, state: &SharedState) -> Result<()> {
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

        let mut autosave = if config.autosave.enabled {
            Some(Autosave::new(serial, &config.autosave)?)
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
                            Ok(ua) => converted.push((ua, sample.logic)),
                            Err(e) => eprintln!("conversion error: {}", e),
                        }
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
                            asv.push((ua as f32, logic));
                        }
                        asv.maybe_flush();
                    }
                }
                Ok(None) => continue,
                Err(_) => break,
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

    pub fn run_daemon(_port_path: &str, serial: &str) -> Result<()> {
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
