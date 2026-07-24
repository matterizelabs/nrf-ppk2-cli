use crate::daemon;
use crate::error::Result;
use crate::transport::resolve_port;

pub fn run_start(_json: bool, port: Option<&str>, serial: Option<&str>) -> Result<()> {
    let port_path = resolve_port(port, serial)?;
    let sn = serial.unwrap_or("unknown");
    daemon::run_daemon(&port_path, sn)
}

pub fn run_stop(json: bool, save: Option<&str>, serial: Option<&str>) -> Result<()> {
    let sn = serial.unwrap_or("unknown");
    let cmd = if let Some(sp) = save {
        format!("{{\"cmd\":\"stop\",\"save\":\"{}\"}}", sp)
    } else {
        r#"{"cmd":"stop"}"#.to_string()
    };
    let resp = daemon::send_command(sn, &cmd)?;
    if json {
        println!("{}", resp);
    } else {
        println!("daemon stopped");
    }
    Ok(())
}

pub fn run_status(json: bool, serial: Option<&str>) -> Result<()> {
    let sn = serial.unwrap_or("unknown");
    let resp = daemon::send_command(sn, r#"{"cmd":"status"}"#)?;
    if json {
        println!("{}", resp);
    } else {
        eprintln!("daemon status: {}", resp);
    }
    Ok(())
}
