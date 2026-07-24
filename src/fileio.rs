use std::fmt::Write as _;
use std::io::Write;

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

    zip.start_file("minimap.json", options)?;
    zip.write_all(build_minimap(frames).as_bytes())?;

    zip.finish()?;
    Ok(())
}

fn build_minimap(frames: &[(f32, u8)]) -> String {
    let step = (frames.len() / 10_000).max(1);
    let entries = frames.len() / step + usize::from(!frames.len().is_multiple_of(step));
    let mut buf = String::with_capacity(entries * 32 + 100);
    buf.push('[');
    let mut first = true;
    for chunk in frames.chunks(step) {
        let mut min_na = i64::MAX;
        let mut max_na = i64::MIN;
        for &(ua, _) in chunk {
            let na = (ua as f64 * 1000.0) as i64;
            if na < min_na {
                min_na = na;
            }
            if na > max_na {
                max_na = na;
            }
        }
        let min_na = min_na.max(200);
        let max_na = max_na.max(200);
        if !first {
            buf.push(',');
        }
        first = false;
        let _ = write!(buf, r#"{{"min":{},"max":{}}}"#, min_na, max_na);
    }
    buf.push(']');
    format!(
        r#"{{"lastElementFoldCount":0,"maxNumberOfElements":10000,"numberOfTimesToFold":1,"data":{}}}"#,
        buf,
    )
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
    let mut output = String::from("timestamp_us,current_ua,D0,D1,D2,D3,D4,D5,D6,D7\n");

    for (i, (ua, logic)) in frames.iter().enumerate() {
        let ts = i as u64 * 10;
        output.push_str(&format!("{},", ts));
        output.push_str(&format!("{:.3},", ua));
        for bit in 0..8 {
            output.push_str(if (logic >> bit) & 1 == 1 { "1," } else { "0," });
        }
        output.pop();
        output.push('\n');
    }

    std::fs::write(csv_path, output)?;
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
