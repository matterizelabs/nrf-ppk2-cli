use crate::error::Result;
use crate::transport::resolve_port;

pub fn firmware_info(port: Option<&str>, serial: Option<&str>) -> Result<()> {
    let (port_path, _) = resolve_port(port, serial)?;
    let device = crate::device::Ppk2Device::open(&port_path)?;
    let hw = &device.metadata().hardware;
    let cal = if device.metadata().calibrated {
        "calibrated"
    } else {
        "uncalibrated"
    };
    println!("firmware: {} ({})", hw, cal);
    Ok(())
}
