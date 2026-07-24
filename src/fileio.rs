use crate::conversion::convert_bits16;
use crate::error::Result;
use std::io::Write;

pub fn write_ppk2(
    path: &str,
    frames: &[(f32, u8)],
    start_time_ms: u64,
) -> Result<()> {
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

    let step = (frames.len() / 10_000).max(1);
    let mut minimap_data = String::from("[");
    let mut first = true;
    for chunk in frames.chunks(step) {
        let min_na = chunk.iter().map(|(ua, _)| (ua * 1000.0) as i64).min().unwrap_or(0).max(200);
        let max_na = chunk.iter().map(|(ua, _)| (ua * 1000.0) as i64).max().unwrap_or(0).max(200);
        if !first { minimap_data.push(','); }
        first = false;
        minimap_data.push_str(&format!(r#"{{"min":{},"max":{}}}"#, min_na, max_na));
    }
    minimap_data.push(']');
    let minimap = format!(
        r#"{{"lastElementFoldCount":0,"maxNumberOfElements":10000,"numberOfTimesToFold":1,"data":{}}}"#,
        minimap_data,
    );
    zip.start_file("minimap.json", options)?;
    zip.write_all(minimap.as_bytes())?;

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
        let frames = vec![
            (42.0f32, 0x03u8),
            (100.0f32, 0xFFu8),
            (0.0f32, 0x00u8),
        ];
        let path = "/tmp/test_roundtrip.ppk2";
        write_ppk2(path, &frames, 1720000000000).unwrap();
        let read = read_ppk2(path).unwrap();
        assert_eq!(read.len(), 3);
        assert!((read[0].0 - 42.0).abs() < 1.0);
        assert_eq!(read[1].1, 0xFF);
    }
}
