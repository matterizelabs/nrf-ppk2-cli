use crate::device::Ppk2Device;
use crate::error::Result;
use crate::transport::resolve_port;

pub fn run(json: bool, mv: u16, port: Option<&str>, serial: Option<&str>) -> Result<()> {
    if !(800..=5000).contains(&mv) {
        return Err(crate::error::Error::InvalidArg(format!(
            "{}mV out of range (800-5000mV)",
            mv
        )));
    }
    let (port_path, _) = resolve_port(port, serial)?;
    let mut device = Ppk2Device::open(&port_path)?;
    device.set_voltage(mv)?;

    if json {
        println!(r#"{{"vdd_mv":{}}}"#, mv);
    } else {
        println!("VDD:{}mV", mv);
    }

    Ok(())
}
