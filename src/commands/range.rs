use crate::device::Ppk2Device;
use crate::error::{Error, Result};
use crate::transport::resolve_port;

pub fn run(json: bool, value: u8, port: Option<&str>, serial: Option<&str>) -> Result<()> {
    if value > 4 {
        return Err(Error::InvalidArg("range must be 0-4".into()));
    }
    let port_path = resolve_port(port, serial)?;
    let mut device = Ppk2Device::open(&port_path)?;
    device.set_range(value)?;
    if json {
        println!(r#"{{"ok":true}}"#);
    } else {
        println!("ok");
    }
    Ok(())
}
