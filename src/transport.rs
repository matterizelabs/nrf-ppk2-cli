use crate::error::{Error, Result};
use serialport::{available_ports, SerialPort, SerialPortInfo, SerialPortType};

pub struct Ppk2Port {
    pub(crate) inner: Box<dyn SerialPort>,
}

impl Ppk2Port {
    pub fn open(path: &str) -> Result<Self> {
        let mut inner = serialport::new(path, 115200)
            .timeout(std::time::Duration::from_secs(2))
            .open()
            .map_err(|e| {
                let is_perm = e.kind()
                    == serialport::ErrorKind::Io(std::io::ErrorKind::PermissionDenied);
                if is_perm {
                    Error::InvalidArg(format!(
                        "{}: permission denied (try adding user to dialout group)",
                        path
                    ))
                } else {
                    Error::Serial(e)
                }
            })?;

        // Force DTR high to prevent PPK2 reset on macOS
        inner.write_data_terminal_ready(true).ok();
        inner.write_request_to_send(false).ok();

        Ok(Self { inner })
    }

    pub fn write_command(&mut self, cmd: &[u8]) -> Result<()> {
        self.inner.write_all(cmd)?;
        self.inner.flush()?;
        Ok(())
    }

    pub fn read_bytes(&mut self, buf: &mut [u8]) -> Result<usize> {
        Ok(self.inner.read(buf)?)
    }

    pub fn read_until_end(&mut self) -> Result<String> {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 128];
        loop {
            let n = self.inner.read(&mut chunk)?;
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);
            let text = String::from_utf8_lossy(&buf);
            if text.contains("END") {
                break;
            }
        }
        Ok(String::from_utf8_lossy(&buf).to_string())
    }

    pub fn set_read_timeout(&mut self, dur: std::time::Duration) {
        self.inner.set_timeout(dur).ok();
    }

    pub fn drain_input(&mut self) {
        let mut buf = [0u8; 256];
        self.inner.set_timeout(std::time::Duration::from_millis(10)).ok();
        while self.inner.read(&mut buf).is_ok_and(|n| n > 0) {}
        self.inner.set_timeout(std::time::Duration::from_secs(2)).ok();
    }
}

fn is_ppk2_port(port: &SerialPortInfo) -> bool {
    match &port.port_type {
        SerialPortType::UsbPort(info) => {
            info.vid == 0x1915 && info.pid == 0xC00A
        }
        _ => false,
    }
}

fn extract_serial(path: &str) -> Option<String> {
    // /dev/serial/by-id/usb-Nordic_Semiconductor_PPK2_682294737-if01
    // /dev/cu.usbmodem6822947371
    if path.contains("PPK2_") {
        if let Some(s) = path.split("PPK2_").nth(1) {
            let sn = s.split('-').next().unwrap_or(s);
            return Some(sn.to_string());
        }
    }
    if path.contains("usbmodem") {
        if let Some(s) = path.split("usbmodem").nth(1) {
            let mut digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
            if digits.len() > 1 {
                digits.pop(); // strip trailing interface number
            }
            if !digits.is_empty() {
                return Some(digits);
            }
        }
    }
    None
}

pub fn find_ppk2_ports() -> Vec<(String, String)> {
    let mut result = Vec::new();
    let ports = available_ports().unwrap_or_default();

    for port in &ports {
        if is_ppk2_port(port) {
            let serial = extract_serial(&port.port_name)
                .unwrap_or_else(|| "unknown".to_string());
            result.push((serial, port.port_name.clone()));
        }
    }

    // Sort: prefer by-id paths first
    result.sort_by(|a, b| {
        let a_by_id = a.1.contains("/by-id/");
        let b_by_id = b.1.contains("/by-id/");
        b_by_id.cmp(&a_by_id)
    });

    result
}

pub fn resolve_port(port: Option<&str>, serial: Option<&str>) -> Result<String> {
    if let Some(p) = port {
        return Ok(p.to_string());
    }
    let devices = find_ppk2_ports();
    if let Some(sn) = serial {
        for (dev_sn, path) in &devices {
            if dev_sn == sn {
                return Ok(path.clone());
            }
        }
        return Err(Error::DeviceNotFound);
    }
    devices
        .first()
        .map(|(_, path)| path.clone())
        .ok_or(Error::DeviceNotFound)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_serial_by_id() {
        let sn = extract_serial(
            "/dev/serial/by-id/usb-Nordic_Semiconductor_PPK2_682294737-if01"
        );
        assert_eq!(sn, Some("682294737".to_string()));
    }

    #[test]
    fn extract_serial_usbmodem() {
        let sn = extract_serial("/dev/cu.usbmodem6822947371");
        assert_eq!(sn, Some("682294737".to_string()));
    }

    #[test]
    fn extract_serial_none() {
        let sn = extract_serial("/dev/ttyACM0");
        assert_eq!(sn, None);
    }
}
