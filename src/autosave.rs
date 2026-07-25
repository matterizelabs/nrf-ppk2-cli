use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::AutosaveConfig;
use crate::error::Result;
use crate::fileio;

pub struct Autosave {
    path: PathBuf,
    enabled: bool,
    interval_s: u64,
    last_flush_count: usize,
    total_frames: usize,
    frames: Vec<(f32, u8)>,
    chunks: Vec<Arc<Vec<(f32, u8)>>>,
    start_time_ms: u64,
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
        })
    }

    pub fn push(&mut self, frame: (f32, u8)) {
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

        if let Some(sp) = save_path {
            let dest = std::path::Path::new(sp);
            let tmp = dest.with_extension("ppk2.tmp");
            let tmp_str = tmp.to_str().unwrap_or("autosave.tmp.ppk2");
            fileio::write_ppk2(tmp_str, &all, self.start_time_ms)?;
            if std::fs::rename(tmp_str, sp).is_err() {
                std::fs::copy(tmp_str, sp)?;
                let _ = std::fs::remove_file(tmp_str);
            }
            let _ = std::fs::remove_file(&self.path);
            Ok(sp.to_string())
        } else {
            let tmp_path = self.path.with_extension("ppk2.tmp");
            let tmp_str = tmp_path.to_str().unwrap_or("autosave.tmp.ppk2");
            fileio::write_ppk2(tmp_str, &all, self.start_time_ms)?;
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
