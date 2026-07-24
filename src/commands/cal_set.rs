use crate::device::Ppk2Device;
use crate::error::{Error, Result};
use crate::transport::resolve_port;

pub fn run(
    json: bool,
    range: u8,
    ohms: f32,
    port: Option<&str>,
    serial: Option<&str>,
) -> Result<()> {
    if range > 4 {
        return Err(Error::InvalidArg("range must be 0-4".into()));
    }
    if ohms <= 0.0 {
        return Err(Error::InvalidArg("ohms must be positive".into()));
    }
    let port_path = resolve_port(port, serial)?;
    let mut device = Ppk2Device::open(&port_path)?;
    device.set_user_resistor(range, ohms)?;
    if json {
        println!(r#"{{"ok":true}}"#);
    } else {
        println!("ok");
    }
    Ok(())
}
