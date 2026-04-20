/// sys-wall configuration loader.
///
/// Loads config from `/etc/sys-wall/config.toml` or `./config.toml`,
/// and supports environment variable overrides with `SYSWALL_` prefix.
use std::fs;

#[derive(Clone, Debug)]
pub struct Config {
    pub refresh_interval_ms: u64,
    pub default_tab: String,
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = Self::find_config_path();
        let mut config = Self {
            refresh_interval_ms: 1000,
            default_tab: "summary".to_string(),
        };

        if let Some(path) = &config_path {
            let content = fs::read_to_string(path)?;
            let parsed: toml::Value = toml::from_str(&content)?;

            if let Some(general) = parsed.get("general").and_then(|v| v.as_table()) {
                if let Some(interval) = general.get("refresh_interval_ms").and_then(|v| u64::try_from(v.as_integer().unwrap_or(1000) as i64).ok()) {
                    config.refresh_interval_ms = interval;
                }
                if let Some(tab) = general.get("default_tab").and_then(|v| v.as_str()) {
                    config.default_tab = tab.to_string();
                }
            }
        }

        // Environment variable overrides
        if let Ok(val) = std::env::var("SYSWALL_GENERAL_REFRESH_INTERVAL_MS") {
            if let Ok(v) = val.parse::<u64>() {
                config.refresh_interval_ms = v;
            }
        }
        if let Ok(val) = std::env::var("SYSWALL_GENERAL_DEFAULT_TAB") {
            config.default_tab = val;
        }

        Ok(config)
    }

    fn find_config_path() -> Option<std::path::PathBuf> {
        let etc = std::path::Path::new("/etc/sys-wall/config.toml");
        if etc.exists() {
            return Some(etc.to_path_buf());
        }
        let local = std::path::Path::new("./config.toml");
        if local.exists() {
            return Some(local.to_path_buf());
        }
        None
    }
}
