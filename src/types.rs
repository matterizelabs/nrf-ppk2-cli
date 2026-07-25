use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementMode {
    Ampere = 1,
    Source = 2,
}

impl MeasurementMode {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(Self::Ampere),
            2 => Some(Self::Source),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Sample {
    pub adc: u16,
    pub range: u8,
    pub counter: u8,
    pub logic: u8,
}

impl Sample {
    pub fn from_raw(raw: u32) -> Self {
        Self {
            adc: (raw & 0x3FFF) as u16,
            range: ((raw >> 14) & 0x7) as u8,
            counter: ((raw >> 18) & 0x3F) as u8,
            logic: ((raw >> 24) & 0xFF) as u8,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Modifiers {
    pub r: [f64; 5],
    pub gs: [f64; 5],
    pub gi: [f64; 5],
    pub o: [f64; 5],
    pub s: [f64; 5],
    pub i: [f64; 5],
    pub ug: [f64; 5],
}

impl Default for Modifiers {
    fn default() -> Self {
        Self {
            r: [1031.64, 101.65, 10.15, 0.94, 0.043],
            gs: [1.0; 5],
            gi: [1.0; 5],
            o: [0.0; 5],
            s: [0.0; 5],
            i: [0.0; 5],
            ug: [1.0; 5],
        }
    }
}

#[derive(Debug, Clone)]
pub struct Metadata {
    pub modifiers: Modifiers,
    pub hardware: String,
    pub mode: u8,
    pub vdd_mv: u16,
    #[allow(dead_code)]
    pub calibrated: bool,
}

#[derive(Debug, Clone)]
pub struct MeasurementStats {
    pub duration_s: f64,
    pub samples: u64,
    pub avg_ua: f64,
    pub charge_uah: f64,
    pub power_uw: Option<f64>,
    pub min_ua: f64,
    pub max_ua: f64,
}

impl fmt::Display for MeasurementStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let power_str = self
            .power_uw
            .map(|p| format!("{:.0}", p))
            .unwrap_or_else(|| "—".to_string());
        write!(
            f,
            "duration {:.1}s  samples {}  avg {:.1}uA  charge {:.3}uAh  power {}uW",
            self.duration_s, self.samples, self.avg_ua, self.charge_uah, power_str,
        )
    }
}

impl MeasurementStats {
    pub fn to_json(&self) -> String {
        let power = self
            .power_uw
            .map(|p| p.to_string())
            .unwrap_or_else(|| "null".to_string());
        format!(
            r#"{{"duration_s":{},"samples":{},"avg_ua":{:.3},"charge_uah":{:.6},"power_uw":{},"min_ua":{:.3},"max_ua":{:.3}}}"#,
            self.duration_s,
            self.samples,
            self.avg_ua,
            self.charge_uah,
            power,
            self.min_ua,
            self.max_ua,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_from_raw_parses_adc() {
        let s = Sample::from_raw(0x00000FFF);
        assert_eq!(s.adc, 0x0FFF);
        assert_eq!(s.range, 0);
        assert_eq!(s.counter, 0);
        assert_eq!(s.logic, 0);
    }

    #[test]
    fn sample_from_raw_parses_range() {
        let s = Sample::from_raw(0x0001C000);
        assert_eq!(s.range, 7);
        assert_eq!(s.adc, 0);
    }

    #[test]
    fn sample_from_raw_parses_counter() {
        let s = Sample::from_raw(0x00FC0000);
        assert_eq!(s.counter, 63);
        assert_eq!(s.adc, 0);
    }

    #[test]
    fn sample_from_raw_parses_logic() {
        let s = Sample::from_raw(0xFF000000);
        assert_eq!(s.logic, 0xFF);
        assert_eq!(s.adc, 0);
    }

    #[test]
    fn sample_from_raw_all_fields() {
        let s = Sample::from_raw(0xAABBCCDD);
        assert_eq!(s.adc, 0x0CDD);
        assert_eq!(s.range, 7);
        assert_eq!(s.counter, 46);
        assert_eq!(s.logic, 0xAA);
    }
}
