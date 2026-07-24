use crate::daemon;
use crate::error::Result;
use crate::transport::{find_ppk2_ports, resolve_port};

fn resolve_daemon_serial(serial: Option<&str>) -> String {
    if let Some(sn) = serial {
        return sn.to_string();
    }
    find_ppk2_ports()
        .first()
        .map(|d| d.serial.clone())
        .unwrap_or_else(|| "unknown".to_string())
}

pub fn run_start(_json: bool, port: Option<&str>, serial: Option<&str>) -> Result<()> {
    let port_path = resolve_port(port, serial)?;
    let sn = resolve_daemon_serial(serial);
    daemon::run_daemon(&port_path, &sn)
}

pub fn run_stop(json: bool, save: Option<&str>, serial: Option<&str>) -> Result<()> {
    let sn = resolve_daemon_serial(serial);
    let cmd = if let Some(sp) = save {
        format!(r#"{{"cmd":"stop","save":"{}"}}"#, sp)
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

pub fn run_status(_json: bool, serial: Option<&str>) -> Result<()> {
    let sn = resolve_daemon_serial(serial);
    let resp = daemon::send_command(&sn, r#"{"cmd":"status"}"#)?;
    println!("{}", resp);
    Ok(())
}
