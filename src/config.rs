use crate::error::Result;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub defaults: DefaultsConfig,
    pub behavior: BehaviorConfig,
    pub autosave: AutosaveConfig,
    pub port: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DefaultsConfig {
    pub mode: String,
    pub voltage_mv: u16,
}

#[derive(Debug, Clone)]
pub struct BehaviorConfig {
    pub auto_power: String,
}

#[derive(Debug, Clone)]
pub struct AutosaveConfig {
    pub enabled: bool,
    pub interval_s: u64,
    pub dir: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            defaults: DefaultsConfig {
                mode: "source".into(),
                voltage_mv: 3300,
            },
            behavior: BehaviorConfig {
                auto_power: "session".into(),
            },
            autosave: AutosaveConfig {
                enabled: true,
                interval_s: 30,
                dir: None,
            },
            port: None,
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        config_path()
    }

    pub fn load() -> Result<Self> {
        let config_path = config_path();

        let mut config = Config::default();
        if config_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                config = parse_config(&content);
            }
        }

        if let Ok(v) = std::env::var("PPK2_VOLTAGE") {
            config.defaults.voltage_mv = v.parse().unwrap_or(config.defaults.voltage_mv);
        }
        if let Ok(v) = std::env::var("PPK2_MODE") {
            config.defaults.mode = v;
        }
        if let Ok(v) = std::env::var("PPK2_AUTOSAVE_DIR") {
            config.autosave.dir = Some(v);
        }
        if let Ok(v) = std::env::var("PPK2_PORT") {
            config.port = Some(v);
        }

        Ok(config)
    }

    pub fn autosave_dir(&self) -> PathBuf {
        if let Some(ref d) = self.autosave.dir {
            return PathBuf::from(d);
        }
        data_dir().join("autosave")
    }

    pub fn state_dir() -> PathBuf {
        #[cfg(target_os = "macos")]
        {
            home_dir()
                .join("Library")
                .join("Application Support")
                .join("ppk2")
        }
        #[cfg(target_os = "linux")]
        {
            if let Ok(xdg) = std::env::var("XDG_STATE_HOME") {
                PathBuf::from(xdg).join("ppk2")
            } else {
                home_dir().join(".local").join("state").join("ppk2")
            }
        }
        #[cfg(target_os = "windows")]
        {
            if let Ok(appdata) = std::env::var("LOCALAPPDATA") {
                PathBuf::from(appdata).join("ppk2")
            } else {
                home_dir().join("AppData").join("Local").join("ppk2")
            }
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            home_dir().join(".local").join("state").join("ppk2")
        }
    }
}

fn config_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        home_dir()
            .join("Library")
            .join("Application Support")
            .join("ppk2")
            .join("config.toml")
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(xdg).join("ppk2").join("config.toml")
        } else {
            home_dir().join(".config").join("ppk2").join("config.toml")
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            PathBuf::from(appdata).join("ppk2").join("config.toml")
        } else {
            home_dir()
                .join("AppData")
                .join("Roaming")
                .join("ppk2")
                .join("config.toml")
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        home_dir().join(".config").join("ppk2").join("config.toml")
    }
}

fn data_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        home_dir()
            .join("Library")
            .join("Application Support")
            .join("ppk2")
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            PathBuf::from(xdg).join("ppk2")
        } else {
            home_dir().join(".local").join("share").join("ppk2")
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("LOCALAPPDATA") {
            PathBuf::from(appdata).join("ppk2")
        } else {
            home_dir().join("AppData").join("Local").join("ppk2")
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        home_dir().join(".local").join("share").join("ppk2")
    }
}

fn home_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home)
    } else if let Ok(profile) = std::env::var("USERPROFILE") {
        PathBuf::from(profile)
    } else {
        PathBuf::from(".")
    }
}

fn toml_value(value: &str) -> String {
    let v = value.trim();
    if v.starts_with('"') && v.ends_with('"') && v.len() >= 2 {
        v[1..v.len() - 1].to_string()
    } else {
        v.to_string()
    }
}

fn toml_bool(value: &str) -> bool {
    let v = value.trim();
    v == "true" || v == "\"true\""
}

fn toml_u64(value: &str) -> u64 {
    let v = value.trim().trim_matches('"');
    v.parse().unwrap_or(0)
}

fn toml_u16(value: &str) -> u16 {
    let v = value.trim().trim_matches('"');
    v.parse().unwrap_or(0)
}

fn parse_config(content: &str) -> Config {
    let mut config = Config::default();
    let mut section = "";

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = &line[1..line.len() - 1];
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();

            match (section, key) {
                ("defaults", "mode") => {
                    let v = toml_value(value);
                    if v == "source" || v == "ampere" {
                        config.defaults.mode = v;
                    } else {
                        eprintln!("warning: invalid mode '{}', using default 'source'", v);
                    }
                }
                ("defaults", "voltage_mv") => {
                    config.defaults.voltage_mv = toml_u16(value);
                }
                ("behavior", "auto_power") => {
                    let v = toml_value(value);
                    if v == "never" || v == "session" || v == "always" {
                        config.behavior.auto_power = v;
                    } else {
                        eprintln!(
                            "warning: invalid auto_power '{}', using default 'session'",
                            v
                        );
                    }
                }
                ("autosave", "enabled") => config.autosave.enabled = toml_bool(value),
                ("autosave", "interval_s") => {
                    config.autosave.interval_s = toml_u64(value);
                }
                ("autosave", "dir") => config.autosave.dir = Some(toml_value(value)),
                _ => {}
            }
        }
    }

    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let c = Config::default();
        assert_eq!(c.defaults.mode, "source");
        assert_eq!(c.defaults.voltage_mv, 3300);
        assert_eq!(c.behavior.auto_power, "session");
        assert!(c.autosave.enabled);
        assert_eq!(c.autosave.interval_s, 30);
    }

    #[test]
    fn parse_toml_config() {
        let toml_str = r#"
[defaults]
mode = "ampere"
voltage_mv = 5000

[behavior]
auto_power = "never"
"#;
        let c = parse_config(toml_str);
        assert_eq!(c.defaults.mode, "ampere");
        assert_eq!(c.defaults.voltage_mv, 5000);
        assert_eq!(c.behavior.auto_power, "never");
    }

    #[test]
    fn parse_toml_bare_boolean() {
        let toml_str = r#"
[autosave]
enabled = true
interval_s = 60
"#;
        let c = parse_config(toml_str);
        assert!(c.autosave.enabled);
        assert_eq!(c.autosave.interval_s, 60);
    }

    #[test]
    fn parse_toml_quoted_boolean() {
        let toml_str = r#"
[autosave]
enabled = "true"
"#;
        let c = parse_config(toml_str);
        assert!(c.autosave.enabled);
    }

    #[test]
    fn parse_toml_bare_integer() {
        let toml_str = r#"
[defaults]
voltage_mv = 1800
"#;
        let c = parse_config(toml_str);
        assert_eq!(c.defaults.voltage_mv, 1800);
    }
}
