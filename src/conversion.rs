use crate::error::Result;
use crate::types::{Modifiers, Sample};

const ADC_MULT: f64 = 1.8 / 163840.0;

pub struct Converter {
    modifiers: Modifiers,
    vdd_mv: u16,
    spike_state: SpikeFilterState,
    ema_enabled: bool,
}

struct SpikeFilterState {
    ema_fast: [f64; 5],
    prev_range: Option<u8>,
    range_transition_count: u8,
    filter_phase: FilterPhase,
}

#[derive(PartialEq)]
enum FilterPhase {
    Idle,
    Transition,
}

impl SpikeFilterState {
    fn new() -> Self {
        Self {
            ema_fast: [0.0; 5],
            prev_range: None,
            range_transition_count: 0,
            filter_phase: FilterPhase::Idle,
        }
    }
}

impl Converter {
    pub fn new(modifiers: Modifiers, vdd_mv: u16) -> Self {
        Self {
            modifiers,
            vdd_mv,
            spike_state: SpikeFilterState::new(),
            ema_enabled: true,
        }
    }

    pub fn set_vdd(&mut self, vdd_mv: u16) {
        self.vdd_mv = vdd_mv;
    }

    pub fn set_ema_enabled(&mut self, enabled: bool) {
        self.ema_enabled = enabled;
    }

    pub fn adc_to_ua(&mut self, sample: &Sample) -> Result<f64> {
        if sample.range > 4 {
            return Err(crate::error::Error::Other(format!(
                "invalid range value from firmware: {} (valid: 0-4)",
                sample.range
            )));
        }
        let range = sample.range as usize;
        let adc_result = (sample.adc as f64) * 4.0;

        let o = self.modifiers.o[range];
        let r = self.modifiers.r[range];
        let gs = self.modifiers.gs[range];
        let gi = self.modifiers.gi[range];
        let s = self.modifiers.s[range];
        let i = self.modifiers.i[range];
        let ug = self.modifiers.ug[range];

        let no_gain = (adc_result - o) * (ADC_MULT / r);
        let adc = ug * (no_gain * (gs * no_gain + gi) + (s * (self.vdd_mv as f64 / 1000.0) + i));
        let current_ua = adc * 1_000_000.0;

        if self.ema_enabled {
            Ok(self.apply_spike_filter(current_ua, sample.range))
        } else {
            Ok(current_ua)
        }
    }

    fn apply_spike_filter(&mut self, raw_ua: f64, range: u8) -> f64 {
        let range_idx = range as usize;

        if Some(range) != self.spike_state.prev_range {
            self.spike_state.range_transition_count = 0;
            self.spike_state.filter_phase = FilterPhase::Transition;
            self.spike_state.prev_range = Some(range);
        }

        if self.spike_state.filter_phase == FilterPhase::Transition {
            self.spike_state.range_transition_count += 1;
            if self.spike_state.range_transition_count > 3 {
                self.spike_state.filter_phase = FilterPhase::Idle;
            }
        }

        let alpha = if range_idx < 4 { 0.18 } else { 0.06 };
        let ema = &mut self.spike_state.ema_fast[range_idx];
        *ema = alpha * raw_ua + (1.0 - alpha) * *ema;
        *ema
    }
}

pub fn convert_bits16(logic: u8) -> u16 {
    let mut out: u16 = 0;
    for i in 0..8 {
        let bit = (logic >> i) & 1;
        let encoded = if bit == 1 { 0b10 } else { 0b01 };
        out |= (encoded as u16) << (i * 2);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Sample;

    fn make_sample(adc: u16, range: u8) -> Sample {
        Sample {
            adc,
            range,
            counter: 0,
            logic: 0,
        }
    }

    #[test]
    fn convert_zero_current() {
        let modifiers = Modifiers::default();
        let mut converter = Converter::new(modifiers, 3300);
        let sample = make_sample(0, 2);
        let ua = converter.adc_to_ua(&sample).unwrap();
        assert!((ua).abs() < 100.0);
    }

    #[test]
    fn convert_known_value() {
        let modifiers = Modifiers::default();
        let mut converter = Converter::new(modifiers, 3300);
        let sample = make_sample(1000, 2);
        let ua = converter.adc_to_ua(&sample).unwrap();
        assert!(ua > 0.0);
        assert!(ua < 1_000_000.0);
    }

    #[test]
    fn corrupted_range_flagged() {
        let modifiers = Modifiers::default();
        let mut converter = Converter::new(modifiers, 3300);
        let sample = Sample {
            adc: 100,
            range: 7,
            counter: 0,
            logic: 0,
        };
        assert!(converter.adc_to_ua(&sample).is_err());
    }

    #[test]
    fn ema_can_be_disabled() {
        let modifiers = Modifiers::default();
        let mut converter = Converter::new(modifiers, 3300);
        converter.set_ema_enabled(false);
        let sample = make_sample(1000, 2);
        let ua = converter.adc_to_ua(&sample).unwrap();
        assert!(ua > 0.0);
    }

    #[test]
    fn convert_bits16_all_low() {
        assert_eq!(convert_bits16(0x00), 0x5555);
    }

    #[test]
    fn convert_bits16_all_high() {
        assert_eq!(convert_bits16(0xFF), 0xAAAA);
    }

    #[test]
    fn convert_bits16_mixed() {
        assert_eq!(convert_bits16(0x55), 0x6666);
    }

    #[test]
    fn converter_matches_calibration_defaults() {
        let modifiers = Modifiers::default();
        let mut converter = Converter::new(modifiers, 3300);
        let sample = make_sample(0x200, 2);
        let _ = converter.adc_to_ua(&sample).unwrap();
    }
}
