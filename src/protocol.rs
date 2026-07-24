pub enum Command {
    TriggerSet { level_ua: u16 },
    AvgNumSet { count: u8 },
    TriggerWindowSet { window: u8 },
    TriggerIntervalSet { interval: u8 },
    TriggerSingleSet,
    AverageStart,
    AverageStop,
    RangeSet { range: u8 },
    TriggerStop,
    DeviceRunningSet { on: bool },
    RegulatorSet { mv: u16 },
    SwitchPointDown { value: u8 },
    SwitchPointUp { value: u8 },
    TriggerExtToggle,
    SetPowerMode { mode: u8 },
    ResUserSet { range: u8, resistor: f32 },
    SpikeFilteringOn,
    SpikeFilteringOff,
    GetMetadata,
    Reset,
    SetUserGains { range: u8, gain: f32 },
}

impl Command {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Self::TriggerSet { level_ua } => {
                let b = (level_ua >> 8) as u8;
                let c = (level_ua & 0xFF) as u8;
                vec![0x01, b, c]
            }
            Self::AvgNumSet { count } => vec![0x02, *count],
            Self::TriggerWindowSet { window } => vec![0x03, *window],
            Self::TriggerIntervalSet { interval } => vec![0x04, *interval],
            Self::TriggerSingleSet => vec![0x05],
            Self::AverageStart => vec![0x06],
            Self::AverageStop => vec![0x07],
            Self::RangeSet { range } => vec![0x08, *range],
            Self::TriggerStop => vec![0x0A],
            Self::DeviceRunningSet { on } => vec![0x0C, if *on { 1 } else { 0 }],
            Self::RegulatorSet { mv } => {
                let (b1, b2) = encode_regulator_voltage(*mv);
                vec![0x0D, b1, b2]
            }
            Self::SwitchPointDown { value } => vec![0x0E, *value],
            Self::SwitchPointUp { value } => vec![0x0F, *value],
            Self::TriggerExtToggle => vec![0x10],
            Self::SetPowerMode { mode } => vec![0x11, *mode],
            Self::ResUserSet { range, resistor } => {
                let mut v = vec![0x12, *range];
                v.extend_from_slice(&resistor.to_le_bytes());
                v
            }
            Self::SpikeFilteringOn => vec![0x15],
            Self::SpikeFilteringOff => vec![0x16],
            Self::GetMetadata => vec![0x19],
            Self::Reset => vec![0x20],
            Self::SetUserGains { range, gain } => {
                let mut v = vec![0x25, *range];
                v.extend_from_slice(&gain.to_le_bytes());
                v
            }
        }
    }
}

pub fn encode_regulator_voltage(mv: u16) -> (u8, u8) {
    let mv = mv.clamp(800, 5000);
    let diff = mv - 800 + 32;
    let b1 = 3u8 + (diff / 256) as u8;
    let b2 = (diff % 256) as u8;
    (b1, b2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn average_start() {
        assert_eq!(Command::AverageStart.to_bytes(), vec![0x06]);
    }

    #[test]
    fn average_stop() {
        assert_eq!(Command::AverageStop.to_bytes(), vec![0x07]);
    }

    #[test]
    fn device_running_set_on() {
        assert_eq!(
            Command::DeviceRunningSet { on: true }.to_bytes(),
            vec![0x0C, 1]
        );
    }

    #[test]
    fn device_running_set_off() {
        assert_eq!(
            Command::DeviceRunningSet { on: false }.to_bytes(),
            vec![0x0C, 0]
        );
    }

    #[test]
    fn regulator_set_3300mv() {
        let cmd = Command::RegulatorSet { mv: 3300 };
        assert_eq!(cmd.to_bytes(), vec![0x0D, 12, 0xE4]);
    }

    #[test]
    fn regulator_set_800mv_min() {
        let (b1, b2) = encode_regulator_voltage(800);
        assert_eq!((b1, b2), (3, 32));
    }

    #[test]
    fn regulator_set_5000mv_max() {
        let (b1, b2) = encode_regulator_voltage(5000);
        assert_eq!((b1, b2), (19, 136));
    }

    #[test]
    fn set_power_mode_ampere() {
        assert_eq!(Command::SetPowerMode { mode: 1 }.to_bytes(), vec![0x11, 1]);
    }

    #[test]
    fn set_power_mode_source() {
        assert_eq!(Command::SetPowerMode { mode: 2 }.to_bytes(), vec![0x11, 2]);
    }

    #[test]
    fn get_metadata() {
        assert_eq!(Command::GetMetadata.to_bytes(), vec![0x19]);
    }

    #[test]
    fn reset() {
        assert_eq!(Command::Reset.to_bytes(), vec![0x20]);
    }

    #[test]
    fn set_user_gains() {
        let cmd = Command::SetUserGains {
            range: 2,
            gain: 1.5,
        };
        let bytes = cmd.to_bytes();
        assert_eq!(bytes[0], 0x25);
        assert_eq!(bytes[1], 2);
        assert_eq!(bytes.len(), 6);
    }

    #[test]
    fn trigger_set() {
        let cmd = Command::TriggerSet { level_ua: 15000 };
        assert_eq!(cmd.to_bytes(), vec![0x01, 0x3A, 0x98]);
    }

    #[test]
    fn spike_filtering_on() {
        assert_eq!(Command::SpikeFilteringOn.to_bytes(), vec![0x15]);
    }

    #[test]
    fn spike_filtering_off() {
        assert_eq!(Command::SpikeFilteringOff.to_bytes(), vec![0x16]);
    }

    #[test]
    fn opcode_count() {
        // Ensure all 19 opcode types can be constructed
        let _ = Command::TriggerSet { level_ua: 0 };
        let _ = Command::AvgNumSet { count: 0 };
        let _ = Command::TriggerWindowSet { window: 0 };
        let _ = Command::TriggerIntervalSet { interval: 0 };
        let _ = Command::TriggerSingleSet;
        let _ = Command::AverageStart;
        let _ = Command::AverageStop;
        let _ = Command::RangeSet { range: 0 };
        let _ = Command::TriggerStop;
        let _ = Command::DeviceRunningSet { on: false };
        let _ = Command::RegulatorSet { mv: 800 };
        let _ = Command::SwitchPointDown { value: 0 };
        let _ = Command::SwitchPointUp { value: 0 };
        let _ = Command::TriggerExtToggle;
        let _ = Command::SetPowerMode { mode: 0 };
        let _ = Command::ResUserSet {
            range: 0,
            resistor: 0.0,
        };
        let _ = Command::SpikeFilteringOn;
        let _ = Command::SpikeFilteringOff;
        let _ = Command::GetMetadata;
        let _ = Command::Reset;
        let _ = Command::SetUserGains {
            range: 0,
            gain: 0.0,
        };
    }
}
