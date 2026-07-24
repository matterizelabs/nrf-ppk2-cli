use crate::device::Ppk2Device;
use crate::error::Result;
use crate::transport::resolve_port;

pub fn run(json: bool, on: bool, port: Option<&str>, serial: Option<&str>) -> Result<()> {
    let port_path = resolve_port(port, serial)?;
    let mut device = Ppk2Device::open(&port_path)?;
    if on {
        device.spike_filter_on()?;
    } else {
        device.spike_filter_off()?;
    }
    if json {
        println!(r#"{{"ok":true}}"#);
    } else {
        println!("ok");
    }
    Ok(())
}
