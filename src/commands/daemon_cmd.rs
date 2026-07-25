use std::process::{Command, Stdio};

use crate::daemon;
use crate::error::Result;
use crate::transport::{find_ppk2_ports, resolve_port};
use serde::Serialize;

fn resolve_daemon_serial(serial: Option<&str>) -> String {
    if let Some(sn) = serial {
        return sn.to_string();
    }
    find_ppk2_ports()
        .first()
        .map(|d| d.serial.clone())
        .unwrap_or_else(|| "unknown".to_string())
}

#[derive(Serialize)]
struct StopCommand {
    cmd: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    save: Option<String>,
}

pub fn run_start(
    json: bool,
    port: Option<&str>,
    serial: Option<&str>,
    rate: Option<u32>,
) -> Result<()> {
    let (port_path, sn) = resolve_port(port, serial)?;
    let sock = daemon::socket_path(&sn);

    if std::env::var("PPK2_DAEMONIZED").is_ok() {
        daemon::run_daemon(&port_path, &sn, rate)?;
        return Ok(());
    }

    let exe = std::env::current_exe()?;
    let mut cmd = Command::new(exe);
    cmd.arg("daemon")
        .arg("start")
        .arg("--port")
        .arg(&port_path)
        .arg("--serial")
        .arg(&sn)
        .env("PPK2_DAEMONIZED", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    if let Some(r) = rate {
        cmd.arg("--rate").arg(r.to_string());
    }

    let pid = cmd.spawn()?.id();

    if json {
        println!(r#"{{"socket":"{}","pid":{}}}"#, sock.display(), pid,);
    } else {
        println!("{}", sock.display());
        println!("{}", pid);
    }
    Ok(())
}

pub fn run_stop(json: bool, save: Option<&str>, serial: Option<&str>) -> Result<()> {
    let sn = resolve_daemon_serial(serial);
    let cmd = if let Some(sp) = save {
        serde_json::to_string(&StopCommand {
            cmd: "stop",
            save: Some(sp.to_string()),
        })
        .unwrap_or_else(|_| r#"{"cmd":"stop"}"#.into())
    } else {
        r#"{"cmd":"stop"}"#.to_string()
    };
    let resp = daemon::send_command(&sn, &cmd)?;
    if json {
        println!("{}", resp);
    } else {
        println!("daemon stopped");
    }
    Ok(())
}

pub fn run_status(json: bool, serial: Option<&str>) -> Result<()> {
    let sn = resolve_daemon_serial(serial);
    let resp = daemon::send_command(&sn, r#"{"cmd":"status"}"#)?;
    let _ = json;
    println!("{}", resp);
    Ok(())
}
