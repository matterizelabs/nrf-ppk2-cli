use crate::config::Config;
use crate::error::Result;

pub fn run_init(_json: bool) -> Result<()> {
    let path = Config::config_path();
    if path.exists() {
        println!("config already exists at {}", path.display());
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let default_content = "[defaults]\n\
mode = \"source\"\n\
voltage_mv = 3300\n\
\n\
[behavior]\n\
auto_power = \"session\"\n\
\n\
[autosave]\n\
enabled = true\n\
interval_s = 30\n";
    std::fs::write(&path, default_content)?;
    println!("created {}", path.display());
    Ok(())
}

pub fn run_show(json: bool) -> Result<()> {
    let config = Config::load()?;
    let path = Config::config_path();
    let from_file = path.exists();

    if json {
        println!(
            r#"{{"file":"{}","from_file":{},"defaults":{{"mode":"{}","voltage_mv":{}}},"behavior":{{"auto_power":"{}"}},"autosave":{{"enabled":{},"interval_s":{},"dir":{}}}}}"#,
            path.display(),
            from_file,
            config.defaults.mode,
            config.defaults.voltage_mv,
            config.behavior.auto_power,
            config.autosave.enabled,
            config.autosave.interval_s,
            match &config.autosave.dir {
                Some(d) => format!("\"{}\"", d),
                None => "null".to_string(),
            },
        );
    } else {
        println!(
            "file: {} ({})",
            path.display(),
            if from_file { "ok" } else { "defaults" }
        );
        println!();
        println!("[defaults]");
        println!("mode = \"{}\"", config.defaults.mode);
        println!("voltage_mv = {}", config.defaults.voltage_mv);
        println!();
        println!("[behavior]");
        println!("auto_power = \"{}\"", config.behavior.auto_power);
        println!();
        println!("[autosave]");
        println!("enabled = {}", config.autosave.enabled);
        println!("interval_s = {}", config.autosave.interval_s);
        if let Some(ref d) = config.autosave.dir {
            println!("dir = \"{}\"", d);
        }
        println!();
        println!("env overrides: PPK2_VOLTAGE PPK2_MODE PPK2_AUTOSAVE_DIR PPK2_PORT");
    }
    Ok(())
}
