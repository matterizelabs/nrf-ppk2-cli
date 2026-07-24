use std::path::PathBuf;
use std::sync::Arc;
use std::thread::JoinHandle;
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
    writer: Option<JoinHandle<()>>,
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
        let path = dir.join(format!("ppk2-{}.ppk2", secs));

        Ok(Self {
            path,
            enabled: config.enabled,
            interval_s: config.interval_s,
            last_flush_count: 0,
            total_frames: 0,
            frames: Vec::new(),
            chunks: Vec::new(),
            writer: None,
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
        if let Some(ref h) = self.writer {
            if !h.is_finished() {
                return;
            }
            self.writer = None;
        }
        let chunk = Arc::new(std::mem::take(&mut self.frames));
        self.chunks.push(Arc::clone(&chunk));
        self.last_flush_count = self.total_frames;
        let path = self.path.clone();
        let handle = std::thread::spawn(move || {
            let _ = fileio::write_ppk2(path.to_str().unwrap_or("autosave.ppk2"), &chunk, 0);
        });
        self.writer = Some(handle);
    }

    fn join_writer(&mut self) {
        if let Some(h) = self.writer.take() {
            let _ = h.join();
        }
    }

    pub fn finalize(mut self, save_path: Option<&str>) -> Result<String> {
        self.join_writer();

        let total: usize = self.chunks.iter().map(|c| c.len()).sum::<usize>() + self.frames.len();
        let mut all = Vec::with_capacity(total);
        for chunk in &self.chunks {
            all.extend_from_slice(chunk);
        }
        all.extend_from_slice(&self.frames);

        fileio::write_ppk2(self.path.to_str().unwrap_or("autosave.ppk2"), &all, 0)?;
        if let Some(sp) = save_path {
            std::fs::copy(&self.path, sp)?;
            let _ = std::fs::remove_file(&self.path);
            Ok(sp.to_string())
        } else {
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
