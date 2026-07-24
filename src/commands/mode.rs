use crate::device::Ppk2Device;
use crate::error::Result;
use crate::transport::resolve_port;
use crate::types::MeasurementMode;

pub fn run(
    json: bool,
    mode_str: &str,
    port: Option<&str>,
    serial: Option<&str>,
) -> Result<()> {
    let port_path = resolve_port(port, serial)?;
    let mut device = Ppk2Device::open(&port_path)?;

    let mode = match mode_str {
        "source" => MeasurementMode::Source,
        "ampere" => MeasurementMode::Ampere,
        _ => return Err(crate::error::Error::InvalidArg(format!("unknown mode: {}", mode_str))),
    };

    device.set_mode(mode)?;

    if json {
        println!(r#"{{"mode":"{}"}}"#, mode_str);
    } else {
        println!("mode:{}", mode_str);
    }

    Ok(())
}
