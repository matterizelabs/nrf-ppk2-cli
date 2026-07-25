use std::fmt::Write as _;
use std::io::Write as IoWrite;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::AutosaveConfig;
use crate::conversion::convert_bits16;
use crate::error::Result;
use crate::fileio;

const MAX_MINIMAP_ELEMENTS: usize = 10_000;
const PAGE_FRAMES: usize = 10_000;

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
    total_samples: usize,
}

impl FoldingBuffer {
    fn new() -> Self {
        Self {
            times_folded: 1,
            last_fold_count: 0,
            min: [MinimapPoint { x: 0.0, y: 0.0 }; MAX_MINIMAP_ELEMENTS],
            max: [MinimapPoint { x: 0.0, y: 0.0 }; MAX_MINIMAP_ELEMENTS],
            length: 0,
            total_samples: 0,
        }
    }

    fn add_data(&mut self, value_ua: f64) {
        let timestamp = (self.total_samples as u64 * 10) as f64;
        self.total_samples += 1;

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

pub struct Autosave {
    path: PathBuf,
    start_time_ms: u64,
    minimap: FoldingBuffer,
    raw_path: PathBuf,
    raw_file: Option<std::fs::File>,
    page_buf: Vec<u8>,
}

impl Autosave {
    pub fn new(serial: &str, config: &AutosaveConfig) -> Result<Self> {
        let dir = if let Some(ref d) = config.dir {
            PathBuf::from(d)
        } else {
            crate::config::Config::default().autosave_dir()
        };
        let dir = dir.join(serial);
        std::fs::create_dir_all(&dir)?;

        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let start_time_ms = secs * 1000;
        let path = dir.join(format!("ppk2-{}.ppk2", secs));
        let raw_path = dir.join(format!("ppk2-{}.raw", secs));
        let raw_file = Some(std::fs::File::create(&raw_path)?);

        Ok(Self {
            path,
            start_time_ms,
            minimap: FoldingBuffer::new(),
            raw_path,
            raw_file,
            page_buf: Vec::with_capacity(PAGE_FRAMES * 6),
        })
    }

    pub fn push(&mut self, frame: (f32, u8)) -> Result<()> {
        self.minimap.add_data(frame.0 as f64);

        let ua = if frame.0 < 0.2 { 0.0f32 } else { frame.0 };
        let bits16 = convert_bits16(frame.1);
        self.page_buf.extend_from_slice(&ua.to_le_bytes());
        self.page_buf.extend_from_slice(&bits16.to_le_bytes());

        if self.page_buf.len() >= PAGE_FRAMES * 6 {
            if let Some(ref mut file) = self.raw_file {
                file.write_all(&self.page_buf)?;
            }
            self.page_buf.clear();
        }
        Ok(())
    }

    fn flush_page(&mut self) -> Result<()> {
        if let Some(ref mut file) = self.raw_file {
            if !self.page_buf.is_empty() {
                file.write_all(&self.page_buf)?;
            }
            file.flush()?;
        }
        self.page_buf.clear();
        Ok(())
    }

    pub fn finalize(mut self, save_path: Option<&str>) -> Result<String> {
        self.flush_page()?;
        let file = self.raw_file.take().expect("raw_file must be Some");
        file.sync_all()?;
        drop(file);

        let minimap_json = self.minimap.to_json();

        if let Some(sp) = save_path {
            let dest = std::path::Path::new(sp);
            let tmp = dest.with_extension("ppk2.tmp");
            let tmp_str = tmp.to_str().unwrap_or("autosave.tmp.ppk2");
            fileio::write_ppk2_from_raw(
                tmp_str,
                &self.raw_path.to_string_lossy(),
                &minimap_json,
                self.start_time_ms,
            )?;
            if std::fs::rename(tmp_str, sp).is_err() {
                std::fs::copy(tmp_str, sp)?;
                let _ = std::fs::remove_file(tmp_str);
            }
            let _ = std::fs::remove_file(&self.raw_path);
            let _ = std::fs::remove_file(&self.path);
            Ok(sp.to_string())
        } else {
            let tmp_path = self.path.with_extension("ppk2.tmp");
            let tmp_str = tmp_path.to_str().unwrap_or("autosave.tmp.ppk2");
            fileio::write_ppk2_from_raw(
                tmp_str,
                &self.raw_path.to_string_lossy(),
                &minimap_json,
                self.start_time_ms,
            )?;
            std::fs::rename(tmp_str, &self.path)?;
            let _ = std::fs::remove_file(&self.raw_path);
            Ok(self.path.to_string_lossy().to_string())
        }
    }

    pub fn recover(base_dir: &PathBuf) -> Result<Vec<String>> {
        let mut files = Vec::new();
        if !base_dir.exists() {
            return Ok(files);
        }
        for entry in std::fs::read_dir(base_dir)? {
            let entry = entry?;
            let fname = entry.file_name().to_string_lossy().to_string();
            if fname.starts_with("ppk2-") && fname.ends_with(".ppk2") {
                files.push(entry.path().to_string_lossy().to_string());
            }
        }
        files.sort();
        Ok(files)
    }
}
