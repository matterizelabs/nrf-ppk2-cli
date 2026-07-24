use std::io::Read;

use crate::error::{Error, Result};
use crate::protocol::Command;
use crate::transport::Ppk2Port;
use crate::metadata;
use crate::conversion::Converter;
use crate::types::{MeasurementMode, Metadata, Sample};

pub struct Ppk2Device {
    port: Ppk2Port,
    metadata: Metadata,
    converter: Converter,
    current_mode: MeasurementMode,
    source_vdd_mv: u16,
    ampere_vdd_mv: Option<u16>,
    power_on: bool,
    measuring: bool,
}

impl Ppk2Device {
    pub fn open(port_path: &str) -> Result<Self> {
        let mut port = Ppk2Port::open(port_path)?;

        port.drain_input();

        port.write_command(&Command::GetMetadata.to_bytes())?;
        let response = port.read_until_end()?;
        let metadata = metadata::parse_metadata(&response)?;

        let hw = &metadata.hardware;
        if hw.contains("v") {
            if let Some(ver_str) = hw.split('v').nth(1) {
                let ver: Vec<&str> = ver_str.split(&[' ', '-']).collect();
                if let Some(v) = ver.first() {
                    if let Ok(version) = v.parse::<f64>() {
                        if version < 1.1 || version > 1.3 {
                            eprintln!(
                                "warning: firmware {} may be incompatible (tested 1.1.0–1.2.4)",
                                hw
                            );
                        }
                    }
                }
            }
        }

        port.write_command(
            &Command::RegulatorSet { mv: metadata.vdd_mv }.to_bytes()
        )?;

        for range in 0..5u8 {
            let ug = metadata.modifiers.ug[range as usize];
            if !(0.9..=1.1).contains(&ug) {
                eprintln!("warning: user gain for range {} ({:.3}) reset to 1.0", range, ug);
                port.write_command(
                    &Command::SetUserGains { range, gain: 1.0 }.to_bytes()
                )?;
            }
        }

        let current_mode = MeasurementMode::from_u8(metadata.mode)
            .unwrap_or(MeasurementMode::Source);
        port.write_command(
            &Command::SetPowerMode { mode: metadata.mode }.to_bytes()
        )?;

        port.write_command(&Command::DeviceRunningSet { on: true }.to_bytes())?;

        let converter = Converter::new(metadata.modifiers.clone(), metadata.vdd_mv);
        let source_vdd_mv = metadata.vdd_mv;

        Ok(Self {
            port,
            metadata,
            converter,
            current_mode,
            source_vdd_mv,
            ampere_vdd_mv: None,
            power_on: true,
            measuring: false,
        })
    }

    pub fn set_power(&mut self, on: bool) -> Result<()> {
        self.port.write_command(&Command::DeviceRunningSet { on }.to_bytes())?;
        self.power_on = on;
        Ok(())
    }

    pub fn set_mode(&mut self, mode: MeasurementMode) -> Result<()> {
        let mode_byte = mode as u8;
        self.port.write_command(&Command::SetPowerMode { mode: mode_byte }.to_bytes())?;

        self.current_mode = mode;

        let vdd = match mode {
            MeasurementMode::Source => self.source_vdd_mv,
            MeasurementMode::Ampere => {
                if let Some(av) = self.ampere_vdd_mv {
                    av
                } else {
                    eprintln!("warning: ampere VDD not set, use 'ppk2 voltage <mv>' for accurate calibration");
                    self.source_vdd_mv
                }
            }
        };
        self.set_voltage(vdd)?;

        if self.power_on {
            self.port.write_command(
                &Command::DeviceRunningSet { on: true }.to_bytes()
            )?;
        }

        Ok(())
    }

    pub fn set_voltage(&mut self, mv: u16) -> Result<()> {
        let mv = mv.clamp(800, 5000);
        self.port.write_command(&Command::RegulatorSet { mv }.to_bytes())?;

        match self.current_mode {
            MeasurementMode::Source => self.source_vdd_mv = mv,
            MeasurementMode::Ampere => self.ampere_vdd_mv = Some(mv),
        }

        self.converter.set_vdd(mv);
        Ok(())
    }

    pub fn start_measurement(&mut self) -> Result<()> {
        if self.measuring {
            return Ok(());
        }
        self.port.drain_input();
        self.port.write_command(&Command::AverageStart.to_bytes())?;

        let mut buf = [0u8; 4];
        self.port.set_read_timeout(std::time::Duration::from_secs(1));
        match self.port.inner.read_exact(&mut buf) {
            Ok(()) => {
                self.port.set_read_timeout(std::time::Duration::from_millis(0));
                self.measuring = true;
                Ok(())
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::TimedOut {
                    Err(Error::Timeout("no data from device after AVERAGE_START".into()))
                } else {
                    Err(Error::Disconnected(0.0))
                }
            }
        }
    }

    pub fn stop_measurement(&mut self) -> Result<()> {
        self.measuring = false;
        self.port.write_command(&Command::AverageStop.to_bytes())?;
        Ok(())
    }

    pub fn read_sample_raw(&mut self) -> Result<Option<u32>> {
        let mut buf = [0u8; 4];
        match self.port.inner.read_exact(&mut buf) {
            Ok(()) => Ok(Some(u32::from_le_bytes(buf))),
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => Ok(None),
            Err(_) => Err(Error::Disconnected(0.0)),
        }
    }

    pub fn convert_sample(&mut self, sample: &Sample) -> f64 {
        self.converter.adc_to_ua(sample)
    }

    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    pub fn current_mode(&self) -> MeasurementMode {
        self.current_mode
    }

    pub fn is_power_on(&self) -> bool {
        self.power_on
    }

    pub fn vdd_mv(&self) -> u16 {
        match self.current_mode {
            MeasurementMode::Source => self.source_vdd_mv,
            MeasurementMode::Ampere => self.ampere_vdd_mv.unwrap_or(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn smoke_open_ppk2() {
        let port_path = "/dev/serial/by-id/usb-Nordic_Semiconductor_PPK2_F057566F0FD6-if01";
        let dev = Ppk2Device::open(port_path).expect("failed to open PPK2");
        let meta = dev.metadata();
        assert!(!meta.hardware.is_empty());
        eprintln!("HW: {} mode: {} vdd: {}mV", meta.hardware, meta.mode, meta.vdd_mv);
    }
}
