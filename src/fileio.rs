use std::fmt::Write as _;
use std::io::{BufWriter, Write};

use crate::conversion::convert_bits16;
use crate::error::Result;

pub fn write_ppk2(path: &str, frames: &[(f32, u8)], start_time_ms: u64) -> Result<()> {
    let file = std::fs::File::create(path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("session.raw", options)?;
    for &(current_ua, logic) in frames {
        let ua = if current_ua < 0.2 { 0.0f32 } else { current_ua };
        let bits16 = convert_bits16(logic);
        zip.write_all(&ua.to_le_bytes())?;
        zip.write_all(&bits16.to_le_bytes())?;
    }

    let metadata = format!(
        r#"{{"metadata":{{"samplesPerSecond":100000,"startSystemTime":{}}},"formatVersion":2}}"#,
        start_time_ms,
    );
    zip.start_file("metadata.json", options)?;
    zip.write_all(metadata.as_bytes())?;

    zip.start_file("minimap.raw", options)?;
    zip.write_all(build_minimap(frames).as_bytes())?;

    zip.finish()?;
    Ok(())
}

const MAX_MINIMAP_ELEMENTS: usize = 10_000;

#[derive(Clone, Copy)]
struct MinimapPoint {
    x: f64,
    y: f64,
}

struct FoldingBuffer {
    times_folded: usize,
    last_fold_count: usize,
    min: [MinimapPoint; MAX_MINIMAP_ELEMENTS],
    max: [MinimapPoint; MAX_MINIMAP_ELEMENTS],
    length: usize,
}

impl FoldingBuffer {
    fn new() -> Self {
        Self {
            times_folded: 1,
            last_fold_count: 0,
            min: [MinimapPoint { x: 0.0, y: 0.0 }; MAX_MINIMAP_ELEMENTS],
            max: [MinimapPoint { x: 0.0, y: 0.0 }; MAX_MINIMAP_ELEMENTS],
            length: 0,
        }
    }

    fn add_data(&mut self, value_ua: f64, timestamp: f64) {
        if self.last_fold_count == 0 {
            self.min[self.length] = MinimapPoint {
                x: timestamp,
                y: f64::MAX,
            };
            self.max[self.length] = MinimapPoint {
                x: timestamp,
                y: f64::MIN,
            };
            self.length += 1;
        }

        let mut value = value_ua * 1000.0;
        if value < 200.0 {
            value = 200.0;
        }

        self.last_fold_count += 1;
        let alpha = 1.0 / self.last_fold_count as f64;
        let i = self.length - 1;

        self.min[i] = MinimapPoint {
            x: timestamp * alpha + self.min[i].x * (1.0 - alpha),
            y: if value.is_finite() {
                value.min(self.min[i].y)
            } else {
                self.min[i].y
            },
        };
        self.max[i] = MinimapPoint {
            x: timestamp * alpha + self.max[i].x * (1.0 - alpha),
            y: if value.is_finite() {
                value.max(self.max[i].y)
            } else {
                self.max[i].y
            },
        };

        if self.last_fold_count == self.times_folded {
            self.last_fold_count = 0;
        }

        if self.length == MAX_MINIMAP_ELEMENTS {
            self.fold();
        }
    }

    fn fold(&mut self) {
        self.times_folded *= 2;
        for i in 0..self.length / 2 {
            let idx = i * 2;
            self.min[i] = MinimapPoint {
                x: (self.min[idx].x + self.min[idx + 1].x) / 2.0,
                y: self.min[idx].y.min(self.min[idx + 1].y),
            };
            self.max[i] = MinimapPoint {
                x: (self.max[idx].x + self.max[idx + 1].x) / 2.0,
                y: self.max[idx].y.max(self.max[idx + 1].y),
            };
        }
        self.length /= 2;
    }

    fn to_json(&self) -> String {
        let len = self.length;
        let mut buf = String::with_capacity(len * 128 + 200);
        let _ = write!(buf, r#""length":{},"min":["#, len);
        for i in 0..len {
            if i > 0 {
                buf.push(',');
            }
            let _ = write!(buf, r#"{{"x":{},"y":{}}}"#, self.min[i].x, self.min[i].y);
        }
        buf.push_str(r#"],"max":["#);
        for i in 0..len {
            if i > 0 {
                buf.push(',');
            }
            let _ = write!(buf, r#"{{"x":{},"y":{}}}"#, self.max[i].x, self.max[i].y);
        }
        buf.push(']');
        format!(
            r#"{{"lastElementFoldCount":{},"data":{{{}}},"maxNumberOfElements":{},"numberOfTimesToFold":{}}}"#,
            self.last_fold_count, buf, MAX_MINIMAP_ELEMENTS, self.times_folded,
        )
    }
}

fn build_minimap(frames: &[(f32, u8)]) -> String {
    let mut fb = FoldingBuffer::new();
    for (i, &(ua, _)) in frames.iter().enumerate() {
        let timestamp = (i as u64 * 10) as f64;
        fb.add_data(ua as f64, timestamp);
    }
    fb.to_json()
}

pub fn read_ppk2(path: &str) -> Result<Vec<(f32, u8)>> {
    let file = std::fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| crate::error::Error::InvalidArg(format!("bad .ppk2 file: {}", e)))?;

    let mut frames = Vec::new();

    if let Ok(mut f) = archive.by_name("session.raw") {
        let mut buf = Vec::new();
        std::io::Read::read_to_end(&mut f, &mut buf)?;
        for chunk in buf.chunks(6) {
            if chunk.len() == 6 {
                let ua = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                let bits16 = u16::from_le_bytes([chunk[4], chunk[5]]);
                let mut logic: u8 = 0;
                for i in 0..8 {
                    let encoded = (bits16 >> (i * 2)) & 0x3;
                    if encoded == 0b10 {
                        logic |= 1 << i;
                    }
                }
                frames.push((ua, logic));
            }
        }
    }

    Ok(frames)
}

pub fn export_csv(ppk2_path: &str, csv_path: &str) -> Result<()> {
    let frames = read_ppk2(ppk2_path)?;
    let file = std::fs::File::create(csv_path)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(b"timestamp_us,current_ua,D0,D1,D2,D3,D4,D5,D6,D7\n")?;

    for (i, (ua, logic)) in frames.iter().enumerate() {
        let ts = i as u64 * 10;
        let row = format!(
            "{},{:.3},{},{},{},{},{},{},{},{}\n",
            ts,
            ua,
            logic & 1,
            (logic >> 1) & 1,
            (logic >> 2) & 1,
            (logic >> 3) & 1,
            (logic >> 4) & 1,
            (logic >> 5) & 1,
            (logic >> 6) & 1,
            (logic >> 7) & 1,
        );
        writer.write_all(row.as_bytes())?;
    }

    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_ppk2() {
        let frames = vec![(42.0f32, 0x03u8), (100.0f32, 0xFFu8), (0.0f32, 0x00u8)];
        let path = "/tmp/test_roundtrip.ppk2";
        write_ppk2(path, &frames, 1720000000000).unwrap();
        let read = read_ppk2(path).unwrap();
        assert_eq!(read.len(), 3);
        assert!((read[0].0 - 42.0).abs() < 1.0);
        assert_eq!(read[1].1, 0xFF);
    }
}
