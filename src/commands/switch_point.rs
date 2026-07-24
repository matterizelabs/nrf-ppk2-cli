use crate::device::Ppk2Device;
use crate::error::Result;
use crate::transport::resolve_port;

pub fn run_down(json: bool, value: u8, port: Option<&str>, serial: Option<&str>) -> Result<()> {
    let port_path = resolve_port(port, serial)?;
    let mut device = Ppk2Device::open(&port_path)?;
    device.set_switch_point_down(value)?;
    if json {
        println!(r#"{{"ok":true}}"#);
    } else {
        println!("ok");
    }
    Ok(())
}

pub fn run_up(json: bool, value: u8, port: Option<&str>, serial: Option<&str>) -> Result<()> {
    let port_path = resolve_port(port, serial)?;
    let mut device = Ppk2Device::open(&port_path)?;
    device.set_switch_point_up(value)?;
    if json {
        println!(r#"{{"ok":true}}"#);
    } else {
        println!("ok");
    }
    Ok(())
}
