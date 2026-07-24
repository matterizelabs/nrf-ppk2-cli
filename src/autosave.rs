use crate::config::AutosaveConfig;
use crate::error::Result;
use crate::fileio;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Autosave {
    path: PathBuf,
    enabled: bool,
    interval_s: u64,
    last_flush_count: usize,
    total_frames: usize,
    frames: Vec<(f32, u8)>,
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
        })
    }

    pub fn push(&mut self, frame: (f32, u8)) {
        self.frames.push(frame);
        self.total_frames += 1;
    }

    pub fn maybe_flush(&mut self) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        let since = self.total_frames - self.last_flush_count;
        if since >= (100_000 * self.interval_s as usize) {
            self.do_flush()?;
        }
        Ok(())
    }

    fn do_flush(&mut self) -> Result<()> {
        fileio::write_ppk2(
            self.path.to_str().unwrap_or("autosave.ppk2"),
            &self.frames,
            0,
        )?;
        self.last_flush_count = self.total_frames;
        Ok(())
    }

    pub fn finalize(mut self, save_path: Option<&str>) -> Result<()> {
        self.do_flush()?;

        if let Some(sp) = save_path {
            std::fs::copy(&self.path, sp)?;
            let _ = std::fs::remove_file(&self.path);
        }

        Ok(())
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
