use crate::device::Ppk2Device;
use crate::error::Result;
use crate::transport::resolve_port;

pub fn run(json: bool, state: &str, port: Option<&str>, serial: Option<&str>) -> Result<()> {
    let (port_path, _) = resolve_port(port, serial)?;
    let mut device = Ppk2Device::open(&port_path)?;

    let on = state == "on";
    device.set_power(on)?;

    let status = if on { "on" } else { "off" };
    if json {
        println!(r#"{{"dut":"{}"}}"#, status);
    } else {
        println!("DUT:{}", status);
    }

    Ok(())
}
