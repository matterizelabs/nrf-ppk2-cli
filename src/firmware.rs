use crate::device::Ppk2Device;
use crate::error::Result;
use crate::transport::resolve_port;

pub fn firmware_info(port: Option<&str>, serial: Option<&str>) -> Result<()> {
    let port_path = resolve_port(port, serial)?;
    let device = Ppk2Device::open(&port_path)?;
    let hw = &device.metadata().hardware;
    println!("app: {}", hw);
    println!("bootloader: (unknown)");
    println!("latest: check with 'ppk2 firmware upgrade'");
    Ok(())
}

pub fn firmware_upgrade(port: Option<&str>, serial: Option<&str>) -> Result<()> {
    let _port_path = resolve_port(port, serial)?;
    println!("device must be in DFU mode (USB PID 0x521F)");
    println!("run: nrfutil device program --firmware <hex> --traits nordicUsb");
    println!(
        "download firmware from: https://github.com/NordicSemiconductor/pc-nrfconnect-ppk/tree/main/firmware"
    );
    Ok(())
}
