use std::io::{BufWriter, Write};

use crate::conversion::convert_bits16;
use crate::error::Result;

#[allow(dead_code)]
pub fn write_ppk2(
    path: &str,
    frames: &[(f32, u8)],
    minimap_json: &str,
    start_time_ms: u64,
) -> Result<()> {
    let file = std::fs::File::create(path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("session.raw", options)?;
    const WRITE_CHUNK: usize = 4096;
    let mut buf = Vec::with_capacity(WRITE_CHUNK * 6);
    for &(current_ua, logic) in frames {
        let ua = if current_ua < 0.2 { 0.0f32 } else { current_ua };
        let bits16 = convert_bits16(logic);
        buf.extend_from_slice(&ua.to_le_bytes());
        buf.extend_from_slice(&bits16.to_le_bytes());
        if buf.len() >= WRITE_CHUNK * 6 {
            zip.write_all(&buf)?;
            buf.clear();
        }
    }
    if !buf.is_empty() {
        zip.write_all(&buf)?;
    }

    let metadata = format!(
        r#"{{"metadata":{{"samplesPerSecond":100000,"startSystemTime":{}}},"formatVersion":2}}"#,
        start_time_ms,
    );
    zip.start_file("metadata.json", options)?;
    zip.write_all(metadata.as_bytes())?;

    zip.start_file("minimap.raw", options)?;
    zip.write_all(minimap_json.as_bytes())?;

    zip.finish()?;
    Ok(())
}

pub fn write_ppk2_from_raw(
    path: &str,
    raw_path: &str,
    minimap_json: &str,
    start_time_ms: u64,
) -> Result<()> {
    let output = std::fs::File::create(path)?;
    let mut zip = zip::ZipWriter::new(output);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("session.raw", options)?;
    let mut raw = std::fs::File::open(raw_path)?;
    let mut buf = [0u8; 65536];
    loop {
        let n = std::io::Read::read(&mut raw, &mut buf)?;
        if n == 0 {
            break;
        }
        zip.write_all(&buf[..n])?;
    }

    let metadata = format!(
        r#"{{"metadata":{{"samplesPerSecond":100000,"startSystemTime":{}}},"formatVersion":2}}"#,
        start_time_ms,
    );
    zip.start_file("metadata.json", options)?;
    zip.write_all(metadata.as_bytes())?;

    zip.start_file("minimap.raw", options)?;
    zip.write_all(minimap_json.as_bytes())?;

    zip.finish()?;
    Ok(())
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
        write_ppk2(path, &frames, "{}", 1720000000000).unwrap();
        let read = read_ppk2(path).unwrap();
        assert_eq!(read.len(), 3);
        assert!((read[0].0 - 42.0).abs() < 1.0);
        assert_eq!(read[1].1, 0xFF);
    }
}
