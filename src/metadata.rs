use crate::error::{Error, Result};
use crate::types::{Metadata, Modifiers};

pub fn parse_metadata(text: &str) -> Result<Metadata> {
    let mut modifiers = Modifiers::default();
    let mut mode: u8 = 0;
    let mut vdd_mv: u16 = 3300;
    let mut hardware = String::new();
    let mut calibrated = false;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line == "END" {
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            let value = value.trim();
            match key.trim() {
                "HW" => hardware = value.to_string(),
                "mode" => mode = value.parse().unwrap_or(0),
                "vdd" | "VDD" => vdd_mv = value.parse().unwrap_or(3300),
                "Calibrated" => calibrated = value == "1",
                "r0" | "R0" => modifiers.r[0] = value.parse().unwrap_or(modifiers.r[0]),
                "r1" | "R1" => modifiers.r[1] = value.parse().unwrap_or(modifiers.r[1]),
                "r2" | "R2" => modifiers.r[2] = value.parse().unwrap_or(modifiers.r[2]),
                "r3" | "R3" => modifiers.r[3] = value.parse().unwrap_or(modifiers.r[3]),
                "r4" | "R4" => modifiers.r[4] = value.parse().unwrap_or(modifiers.r[4]),
                "gs0" | "GS0" => modifiers.gs[0] = value.parse().unwrap_or(1.0),
                "gs1" | "GS1" => modifiers.gs[1] = value.parse().unwrap_or(1.0),
                "gs2" | "GS2" => modifiers.gs[2] = value.parse().unwrap_or(1.0),
                "gs3" | "GS3" => modifiers.gs[3] = value.parse().unwrap_or(1.0),
                "gs4" | "GS4" => modifiers.gs[4] = value.parse().unwrap_or(1.0),
                "gi0" | "GI0" => modifiers.gi[0] = value.parse().unwrap_or(1.0),
                "gi1" | "GI1" => modifiers.gi[1] = value.parse().unwrap_or(1.0),
                "gi2" | "GI2" => modifiers.gi[2] = value.parse().unwrap_or(1.0),
                "gi3" | "GI3" => modifiers.gi[3] = value.parse().unwrap_or(1.0),
                "gi4" | "GI4" => modifiers.gi[4] = value.parse().unwrap_or(1.0),
                "o0" | "O0" => modifiers.o[0] = value.parse().unwrap_or(0.0),
                "o1" | "O1" => modifiers.o[1] = value.parse().unwrap_or(0.0),
                "o2" | "O2" => modifiers.o[2] = value.parse().unwrap_or(0.0),
                "o3" | "O3" => modifiers.o[3] = value.parse().unwrap_or(0.0),
                "o4" | "O4" => modifiers.o[4] = value.parse().unwrap_or(0.0),
                "s0" | "S0" => modifiers.s[0] = value.parse().unwrap_or(0.0),
                "s1" | "S1" => modifiers.s[1] = value.parse().unwrap_or(0.0),
                "s2" | "S2" => modifiers.s[2] = value.parse().unwrap_or(0.0),
                "s3" | "S3" => modifiers.s[3] = value.parse().unwrap_or(0.0),
                "s4" | "S4" => modifiers.s[4] = value.parse().unwrap_or(0.0),
                "i0" | "I0" => modifiers.i[0] = value.parse().unwrap_or(0.0),
                "i1" | "I1" => modifiers.i[1] = value.parse().unwrap_or(0.0),
                "i2" | "I2" => modifiers.i[2] = value.parse().unwrap_or(0.0),
                "i3" | "I3" => modifiers.i[3] = value.parse().unwrap_or(0.0),
                "i4" | "I4" => modifiers.i[4] = value.parse().unwrap_or(0.0),
                "ug0" | "UG0" => modifiers.ug[0] = value.parse().unwrap_or(1.0),
                "ug1" | "UG1" => modifiers.ug[1] = value.parse().unwrap_or(1.0),
                "ug2" | "UG2" => modifiers.ug[2] = value.parse().unwrap_or(1.0),
                "ug3" | "UG3" => modifiers.ug[3] = value.parse().unwrap_or(1.0),
                "ug4" | "UG4" => modifiers.ug[4] = value.parse().unwrap_or(1.0),
                _ => {}
            }
        }
    }

    if hardware.is_empty() {
        return Err(Error::Timeout("metadata response incomplete".into()));
    }

    Ok(Metadata {
        modifiers,
        hardware,
        mode,
        vdd_mv,
        calibrated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_full_metadata() {
        let text = "\
HW: PCA63100 v1.2.4-db16a94
mode: 1
vdd: 5000
r0: 1031.64
r1: 101.65
r2: 10.15
r3: 0.94
r4: 0.043
gs0: 1.0
gs1: 1.0
gs2: 1.0
gs3: 1.0
gs4: 1.0
gi0: 1.0
gi1: 1.0
gi2: 1.0
gi3: 1.0
gi4: 1.0
o0: 0.0
o1: 0.0
o2: 0.0
o3: 0.0
o4: 0.0
s0: 0.0
s1: 0.0
s2: 0.0
s3: 0.0
s4: 0.0
i0: 0.0
i1: 0.0
i2: 0.0
i3: 0.0
i4: 0.0
ug0: 1.0
ug1: 1.0
ug2: 1.0
ug3: 1.0
ug4: 1.0
Calibrated: 1
END
";
        let meta = parse_metadata(text).unwrap();
        assert_eq!(meta.hardware, "PCA63100 v1.2.4-db16a94");
        assert_eq!(meta.mode, 1);
        assert_eq!(meta.vdd_mv, 5000);
        assert!(meta.calibrated);
        assert!((meta.modifiers.r[0] - 1031.64).abs() < 0.01);
        assert!((meta.modifiers.r[4] - 0.043).abs() < 0.001);
    }

    #[test]
    fn parse_minimal_metadata() {
        let text = "\
HW: PCA63100 v1.1.0
mode: 2
vdd: 3300
Calibrated: 0
END
";
        let meta = parse_metadata(text).unwrap();
        assert_eq!(meta.hardware, "PCA63100 v1.1.0");
        assert_eq!(meta.mode, 2);
        assert!(!meta.calibrated);
        assert!((meta.modifiers.r[0] - 1031.64).abs() < 0.01); // default
    }

    #[test]
    fn parse_empty_is_error() {
        assert!(parse_metadata("").is_err());
    }
}
