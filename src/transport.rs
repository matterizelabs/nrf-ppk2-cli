use std::path::Path;

use crate::error::{Error, Result};
use serialport::{available_ports, SerialPort, SerialPortType, UsbPortInfo};

#[derive(Debug, Clone)]
pub struct Ppk2DeviceInfo {
    pub serial: String,
    pub control_port: String,
}

pub struct Ppk2Port {
    inner: Box<dyn SerialPort>,
}

const DEFAULT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

impl Ppk2Port {
    pub fn open(path: &str) -> Result<Self> {
        let mut inner = serialport::new(path, 115200)
            .timeout(DEFAULT_TIMEOUT)
            .open()
            .map_err(|e| {
                let is_perm =
                    e.kind() == serialport::ErrorKind::Io(std::io::ErrorKind::PermissionDenied);
                if is_perm {
                    Error::InvalidArg(format!(
                        "{}: permission denied (try adding user to dialout group)",
                        path
                    ))
                } else {
                    Error::Serial(e)
                }
            })?;

        inner.write_data_terminal_ready(true).ok();
        inner.write_request_to_send(false).ok();

        Ok(Self { inner })
    }

    pub fn with_timeout<T>(
        &mut self,
        dur: std::time::Duration,
        f: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let original = self.inner.timeout();
        self.inner.set_timeout(dur).ok();
        let result = f(self);
        self.inner.set_timeout(original).ok();
        result
    }

    pub fn write_command(&mut self, cmd: &[u8]) -> Result<()> {
        self.inner.write_all(cmd)?;
        self.inner.flush()?;
        Ok(())
    }

    pub fn read_until_end(&mut self) -> Result<String> {
        let mut buf: Vec<u8> = Vec::new();
        let mut chunk = [0u8; 512];
        let mut last_valid: usize = 0;
        loop {
            let n = self.inner.read(&mut chunk)?;
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);
            if let Ok(text) = std::str::from_utf8(&buf) {
                last_valid = buf.len();
                if text.contains("END") {
                    break;
                }
            }
        }
        let text = if last_valid > 0 {
            std::str::from_utf8(&buf[..last_valid])
                .unwrap_or("")
                .to_string()
        } else {
            return Err(Error::Other("non-UTF-8 metadata response".into()));
        };
        Ok(text)
    }

    pub fn set_timeout(&mut self, dur: std::time::Duration) {
        self.inner.set_timeout(dur).ok();
    }

    pub fn drain_input(&mut self) {
        let mut buf = [0u8; 256];
        self.with_timeout(std::time::Duration::from_millis(10), |s| {
            while s.inner.read(&mut buf).is_ok_and(|n| n > 0) {}
        });
    }

    pub fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        self.inner.read_exact(buf)
    }
}

fn read_sysfs_serial(path: &str) -> Option<String> {
    let name = Path::new(path).file_name()?.to_str()?;
    let serial_path = format!("/sys/class/tty/{}/device/serial", name);
    std::fs::read_to_string(&serial_path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn read_sysfs_iface(path: &str) -> Option<u32> {
    let name = Path::new(path).file_name()?.to_str()?;
    let iface_path = format!("/sys/class/tty/{}/device/bInterfaceNumber", name);
    let content = std::fs::read_to_string(&iface_path).ok()?;
    u32::from_str_radix(content.trim(), 16).ok()
}

fn resolve_serial(info: &UsbPortInfo, path: &str) -> String {
    if let Some(ref sn) = info.serial_number {
        if !sn.is_empty() {
            return sn.clone();
        }
    }
    if let Some(sn) = read_sysfs_serial(path) {
        return sn;
    }
    if let Some(sn) = extract_serial(path) {
        return sn;
    }
    "unknown".to_string()
}

fn resolve_iface(path: &str) -> u32 {
    if let Some(iface) = read_sysfs_iface(path) {
        return iface;
    }
    if let Some(iface) = extract_iface_from_path(path) {
        return iface;
    }
    if let Some(name) = Path::new(path).file_name().and_then(|n| n.to_str()) {
        if let Some(rest) = name.strip_prefix("ttyACM") {
            if let Ok(n) = rest.parse::<u32>() {
                return n;
            }
        }
    }
    99
}

fn extract_serial(path: &str) -> Option<String> {
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
                digits.pop();
            }
            if !digits.is_empty() {
                return Some(digits);
            }
        }
    }
    None
}

fn extract_iface_from_path(path: &str) -> Option<u32> {
    if let Some(by_id_seg) = path.split("PPK2_").nth(1) {
        if let Some(iface_str) = by_id_seg.rsplit("-if").next() {
            return u32::from_str_radix(iface_str, 16).ok();
        }
    }
    if let Some(usbmodem_seg) = path.split("usbmodem").nth(1) {
        if let Some(last_char) = usbmodem_seg.chars().last() {
            if let Some(d) = last_char.to_digit(10) {
                return Some(d);
            }
        }
    }
    None
}

pub fn find_ppk2_ports() -> Vec<Ppk2DeviceInfo> {
    use std::collections::BTreeMap;

    let ports = available_ports().unwrap_or_default();
    let mut raw: Vec<(String, String, u32)> = Vec::new();

    for port in &ports {
        if let SerialPortType::UsbPort(ref info) = port.port_type {
            if info.vid == 0x1915 && info.pid == 0xC00A {
                let serial = resolve_serial(info, &port.port_name);
                let iface = resolve_iface(&port.port_name);
                raw.push((serial, port.port_name.clone(), iface));
            }
        }
    }

    let mut groups: BTreeMap<String, Vec<(String, u32)>> = BTreeMap::new();
    for (serial, path, iface) in raw {
        groups.entry(serial).or_default().push((path, iface));
    }

    let mut result = Vec::new();
    for (serial, mut entries) in groups {
        entries.sort_by_key(|(_, iface)| *iface);
        let control_port = entries[0].0.clone();
        result.push(Ppk2DeviceInfo {
            serial,
            control_port,
        });
    }

    result
}

pub fn resolve_port(port: Option<&str>, serial: Option<&str>) -> Result<(String, String)> {
    if let Some(p) = port {
        let sn = serial.map(|s| s.to_string()).unwrap_or_else(|| {
            find_ppk2_ports()
                .into_iter()
                .find(|d| d.control_port == p)
                .map(|d| d.serial)
                .unwrap_or_else(|| "unknown".to_string())
        });
        return Ok((p.to_string(), sn));
    }
    let devices = find_ppk2_ports();
    if let Some(sn) = serial {
        for dev in &devices {
            if dev.serial == sn {
                return Ok((dev.control_port.clone(), sn.to_string()));
            }
        }
        return Err(Error::DeviceNotFound);
    }
    devices
        .first()
        .map(|d| (d.control_port.clone(), d.serial.clone()))
        .ok_or(Error::DeviceNotFound)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_serial_by_id() {
        let sn = extract_serial("/dev/serial/by-id/usb-Nordic_Semiconductor_PPK2_682294737-if01");
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
