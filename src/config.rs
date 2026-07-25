use serde::{Deserialize, Serialize};

use crate::error::Result;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub defaults: DefaultsConfig,
    #[serde(default)]
    pub behavior: BehaviorConfig,
    #[serde(default)]
    pub autosave: AutosaveConfig,
    #[serde(skip)]
    pub port: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsConfig {
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default = "default_voltage")]
    pub voltage_mv: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorConfig {
    #[serde(default = "default_auto_power")]
    pub auto_power: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutosaveConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_interval")]
    pub interval_s: u64,
    #[serde(default)]
    pub dir: Option<String>,
}

fn default_mode() -> String {
    "source".into()
}
fn default_voltage() -> u16 {
    3300
}
fn default_auto_power() -> String {
    "session".into()
}
fn default_true() -> bool {
    true
}
fn default_interval() -> u64 {
    30
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            mode: default_mode(),
            voltage_mv: default_voltage(),
        }
    }
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            auto_power: default_auto_power(),
        }
    }
}

impl Default for AutosaveConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            interval_s: default_interval(),
            dir: None,
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        config_path()
    }

    pub fn load() -> Result<Self> {
        let config_path = config_path();

        let mut config = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            toml::from_str(&content).unwrap_or_default()
        } else {
            Config::default()
        };

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
        let c: Config = toml::from_str(toml_str).unwrap();
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
        let c: Config = toml::from_str(toml_str).unwrap();
        assert!(c.autosave.enabled);
        assert_eq!(c.autosave.interval_s, 60);
    }

    #[test]
    fn parse_toml_missing_section_defaults() {
        let toml_str = r#"
[defaults]
mode = "ampere"
"#;
        let c: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(c.defaults.mode, "ampere");
        assert_eq!(c.defaults.voltage_mv, 3300);
        assert!(c.autosave.enabled);
    }

    #[test]
    fn parse_toml_bare_integer() {
        let toml_str = r#"
[defaults]
voltage_mv = 1800
"#;
        let c: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(c.defaults.voltage_mv, 1800);
    }
}
