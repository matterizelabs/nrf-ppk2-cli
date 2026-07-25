use std::time::Instant;

use crate::conversion::Converter;
use crate::error::{Error, Result};
use crate::metadata;
use crate::protocol::Command;
use crate::transport::Ppk2Port;
use crate::types::{MeasurementMode, Metadata, Sample};

const MAX_READ_RETRIES: u8 = 3;

pub struct Ppk2Device {
    port: Ppk2Port,
    metadata: Metadata,
    converter: Converter,
    current_mode: MeasurementMode,
    source_vdd_mv: u16,
    ampere_vdd_mv: Option<u16>,
    power_on: bool,
    measuring: bool,
    start_time: Option<Instant>,
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
                    let parts: Vec<u32> = v.split('.').filter_map(|p| p.parse().ok()).collect();
                    if parts.len() >= 2 {
                        let major = parts[0];
                        let minor = parts[1];
                        let patch = parts.get(2).copied().unwrap_or(0);
                        let ver_num =
                            (major as f64) + (minor as f64) / 10.0 + (patch as f64) / 100.0;
                        if !(1.1..=1.4).contains(&ver_num) {
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
            &Command::RegulatorSet {
                mv: metadata.vdd_mv,
            }
            .to_bytes(),
        )?;

        for range in 0..5u8 {
            let ug = metadata.modifiers.ug[range as usize];
            if !(0.9..=1.1).contains(&ug) {
                eprintln!(
                    "warning: user gain for range {} ({:.3}) reset to 1.0",
                    range, ug
                );
                port.write_command(&Command::SetUserGains { range, gain: 1.0 }.to_bytes())?;
            }
        }

        let current_mode =
            MeasurementMode::from_u8(metadata.mode).unwrap_or(MeasurementMode::Source);
        port.write_command(
            &Command::SetPowerMode {
                mode: metadata.mode,
            }
            .to_bytes(),
        )?;

        port.write_command(&Command::DeviceRunningSet { on: true }.to_bytes())?;

        let converter = Converter::new(metadata.modifiers.clone(), metadata.vdd_mv);
        let source_vdd_mv = metadata.vdd_mv;

        if !metadata.calibrated {
            eprintln!("warning: device is not calibrated, measurements may be inaccurate");
        }

        Ok(Self {
            port,
            metadata,
            converter,
            current_mode,
            source_vdd_mv,
            ampere_vdd_mv: None,
            power_on: true,
            measuring: false,
            start_time: None,
        })
    }

    pub fn set_power(&mut self, on: bool) -> Result<()> {
        self.port
            .write_command(&Command::DeviceRunningSet { on }.to_bytes())?;
        self.power_on = on;
        Ok(())
    }

    pub fn set_mode(&mut self, mode: MeasurementMode) -> Result<()> {
        let mode_byte = mode as u8;
        self.port
            .write_command(&Command::SetPowerMode { mode: mode_byte }.to_bytes())?;

        self.current_mode = mode;

        let vdd = match mode {
            MeasurementMode::Source => self.source_vdd_mv,
            MeasurementMode::Ampere => self.ampere_vdd_mv.unwrap_or(self.source_vdd_mv),
        };
        self.set_voltage(vdd)?;

        if self.power_on {
            self.port
                .write_command(&Command::DeviceRunningSet { on: true }.to_bytes())?;
        }

        Ok(())
    }

    pub fn set_voltage(&mut self, mv: u16) -> Result<()> {
        let mv = mv.clamp(800, 5000);
        self.port
            .write_command(&Command::RegulatorSet { mv }.to_bytes())?;

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

        self.port.set_timeout(std::time::Duration::from_millis(0));
        self.measuring = true;
        self.start_time = Some(Instant::now());
        Ok(())
    }

    pub fn stop_measurement(&mut self) -> Result<()> {
        self.measuring = false;
        self.port.write_command(&Command::AverageStop.to_bytes())?;
        let mut buf = [0u8; 4];
        self.port
            .with_timeout(std::time::Duration::from_millis(50), |port| {
                while port.read_exact(&mut buf).is_ok() {}
            });
        Ok(())
    }

    pub fn read_sample_raw(&mut self) -> Result<Option<u32>> {
        let mut buf = [0u8; 4];
        let mut retries = MAX_READ_RETRIES;
        loop {
            match self.port.read_exact(&mut buf) {
                Ok(()) => return Ok(Some(u32::from_le_bytes(buf))),
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => return Ok(None),
                Err(_) if retries > 0 => {
                    retries -= 1;
                    continue;
                }
                Err(_) => {
                    let elapsed = self
                        .start_time
                        .as_ref()
                        .map(|t| t.elapsed().as_secs_f64())
                        .unwrap_or(0.0);
                    return Err(Error::Disconnected(elapsed));
                }
            }
        }
    }

    pub fn convert_sample(&mut self, sample: &Sample) -> Result<f64> {
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

    pub fn spike_filter_on(&mut self) -> Result<()> {
        self.port
            .write_command(&Command::SpikeFilteringOn.to_bytes())?;
        self.converter.set_ema_enabled(true);
        Ok(())
    }

    pub fn spike_filter_off(&mut self) -> Result<()> {
        self.port
            .write_command(&Command::SpikeFilteringOff.to_bytes())?;
        self.converter.set_ema_enabled(false);
        Ok(())
    }

    pub fn set_range(&mut self, range: u8) -> Result<()> {
        self.port
            .write_command(&Command::RangeSet { range }.to_bytes())
    }

    pub fn set_avg_num(&mut self, count: u8) -> Result<()> {
        self.port
            .write_command(&Command::AvgNumSet { count }.to_bytes())
    }

    pub fn set_switch_point_down(&mut self, value: u8) -> Result<()> {
        self.port
            .write_command(&Command::SwitchPointDown { value }.to_bytes())
    }

    pub fn set_switch_point_up(&mut self, value: u8) -> Result<()> {
        self.port
            .write_command(&Command::SwitchPointUp { value }.to_bytes())
    }

    pub fn set_user_resistor(&mut self, range: u8, ohms: f32) -> Result<()> {
        self.port.write_command(
            &Command::ResUserSet {
                range,
                resistor: ohms,
            }
            .to_bytes(),
        )
    }

    pub fn firmware_trigger_set(&mut self, level_ua: u32) -> Result<()> {
        self.port
            .write_command(&Command::TriggerSet { level_ua }.to_bytes())
    }

    pub fn firmware_trigger_window(&mut self, window: u8) -> Result<()> {
        self.port
            .write_command(&Command::TriggerWindowSet { window }.to_bytes())
    }

    pub fn firmware_trigger_interval(&mut self, interval: u8) -> Result<()> {
        self.port
            .write_command(&Command::TriggerIntervalSet { interval }.to_bytes())
    }

    pub fn firmware_trigger_single(&mut self) -> Result<()> {
        self.port
            .write_command(&Command::TriggerSingleSet.to_bytes())
    }

    pub fn firmware_trigger_stop(&mut self) -> Result<()> {
        self.port.write_command(&Command::TriggerStop.to_bytes())
    }

    pub fn trigger_ext_toggle(&mut self) -> Result<()> {
        self.port
            .write_command(&Command::TriggerExtToggle.to_bytes())
    }

    pub fn reset_device(&mut self) -> Result<()> {
        self.port.write_command(&Command::Reset.to_bytes())
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
        eprintln!(
            "HW: {} mode: {} vdd: {}mV",
            meta.hardware, meta.mode, meta.vdd_mv
        );
    }
}
