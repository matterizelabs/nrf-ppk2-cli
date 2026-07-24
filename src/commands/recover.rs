use crate::autosave;
use crate::config::Config;
use crate::error::Result;

pub fn run(json: bool, serial: Option<&str>) -> Result<()> {
    let config = Config::load()?;
    let base = config.autosave_dir();
    let scan_dir = if let Some(sn) = serial {
        base.join(sn)
    } else {
        base.clone()
    };

    let files = autosave::Autosave::recover(&scan_dir)?;

    if json {
        println!("{{\"recoverable\":{}}}", files.len());
    } else {
        if files.is_empty() {
            println!("no orphaned autosaves found");
        }
        for f in &files {
            println!("{}", f);
        }
    }

    Ok(())
}
