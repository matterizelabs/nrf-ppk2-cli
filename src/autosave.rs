use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::AutosaveConfig;
use crate::error::Result;
use crate::fileio;

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
    enabled: bool,
    interval_s: u64,
    last_flush_count: usize,
    total_frames: usize,
    frames: Vec<(f32, u8)>,
    chunks: Vec<Arc<Vec<(f32, u8)>>>,
    start_time_ms: u64,
    minimap: FoldingBuffer,
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

        Ok(Self {
            path,
            enabled: config.enabled,
            interval_s: config.interval_s,
            last_flush_count: 0,
            total_frames: 0,
            frames: Vec::new(),
            chunks: Vec::new(),
            start_time_ms,
            minimap: FoldingBuffer::new(),
        })
    }

    pub fn push(&mut self, frame: (f32, u8)) {
        self.minimap.add_data(frame.0 as f64);
        self.frames.push(frame);
        self.total_frames += 1;
    }

    pub fn maybe_flush(&mut self) {
        if !self.enabled {
            return;
        }
        let since = self.total_frames - self.last_flush_count;
        if since < (100_000 * self.interval_s as usize) {
            return;
        }
        let chunk = Arc::new(std::mem::take(&mut self.frames));
        self.chunks.push(chunk);
        self.last_flush_count = self.total_frames;
    }

    pub fn finalize(self, save_path: Option<&str>) -> Result<String> {
        let total: usize = self.chunks.iter().map(|c| c.len()).sum::<usize>() + self.frames.len();
        let mut all = Vec::with_capacity(total);
        for chunk in &self.chunks {
            all.extend_from_slice(chunk);
        }
        all.extend_from_slice(&self.frames);

        let minimap_json = self.minimap.to_json();

        if let Some(sp) = save_path {
            let dest = std::path::Path::new(sp);
            let tmp = dest.with_extension("ppk2.tmp");
            let tmp_str = tmp.to_str().unwrap_or("autosave.tmp.ppk2");
            fileio::write_ppk2(tmp_str, &all, &minimap_json, self.start_time_ms)?;
            if std::fs::rename(tmp_str, sp).is_err() {
                std::fs::copy(tmp_str, sp)?;
                let _ = std::fs::remove_file(tmp_str);
            }
            let _ = std::fs::remove_file(&self.path);
            Ok(sp.to_string())
        } else {
            let tmp_path = self.path.with_extension("ppk2.tmp");
            let tmp_str = tmp_path.to_str().unwrap_or("autosave.tmp.ppk2");
            fileio::write_ppk2(tmp_str, &all, &minimap_json, self.start_time_ms)?;
            std::fs::rename(tmp_str, &self.path)?;
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
