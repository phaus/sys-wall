/// sys-wall configuration loader.
///
/// Loads config from `$HOME/.config/sys-wall/config.toml`. Creates the
/// file and a per-system UUID on first run, then persists on every load.
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use toml::map::Map;
use toml::Value;

const DEFAULT_REFRESH_INTERVAL_MS: u64 = 1000;

#[derive(Clone, Debug)]
pub struct Config {
    pub system_id: String,
    /// True when this file was just created on this run (first-time install).
    pub is_first_run: bool,
    pub refresh_interval_ms: u64,
    pub default_tab: String,
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_dir = Self::config_dir();
        fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join("config.toml");
        let mut is_first_run = !config_path.exists();
        let mut system_id_set = false;

        // Defaults
        let mut refresh_interval_ms = DEFAULT_REFRESH_INTERVAL_MS;
        let mut default_tab = String::from("summary");
        let mut system_id = String::new();

        // Parse existing config if present.
        let parsed: Value = if config_path.exists() {
            fs::read_to_string(&config_path)?
                .parse::<Value>()?
        } else {
            Value::Table(Map::new())
        };

        // Extract system_id (top-level key).
        if let Some(sid) = parsed.get("system_id").and_then(|v| v.as_str()) {
            system_id = sid.to_string();
            system_id_set = true;
        }

        // Extract general section.
        if let Some(general) = parsed.get("general").and_then(|v| v.as_table()) {
            if let Some(v) = general
                .get("refresh_interval_ms")
                .and_then(|v| v.as_integer())
            {
                if v > 0 {
                    refresh_interval_ms = v as u64;
                }
            }
            if let Some(v) = general.get("default_tab").and_then(|v| v.as_str()) {
                if !v.is_empty() {
                    default_tab = v.to_string();
                }
            }
        }

        // Environment-variable overrides.
        if let Ok(val) = std::env::var("SYSWALL_GENERAL_REFRESH_INTERVAL_MS") {
            if let Ok(v) = val.parse::<u64>() {
                refresh_interval_ms = v;
            }
        }
        if let Ok(val) = std::env::var("SYSWALL_GENERAL_DEFAULT_TAB") {
            default_tab = val;
        }

        // Generate UUID when missing.
        if !system_id_set {
            system_id = uuid::Uuid::new_v4().to_string();
            is_first_run = true;
        }

        // Persist (new file or on every load for correctness).
        let mut table = Map::new();
        table.insert(
            "system_id".into(),
            Value::String(system_id.clone()),
        );

        let mut general = Map::new();
        general.insert(
            "refresh_interval_ms".into(),
            Value::Integer(refresh_interval_ms as i64),
        );
        if !default_tab.is_empty() {
            general.insert("default_tab".into(), Value::String(default_tab.clone()));
        }
        table.insert("general".into(), Value::Table(general));

        let output = toml::to_string_pretty(&Value::Table(table))?;
        let mut f = fs::File::create(&config_path)?;
        write!(f, "{}", output)?;
        Ok(Self {
            system_id,
            is_first_run,
            refresh_interval_ms,
            default_tab,
        })
    }

    /// Return `$HOME/.config/sys-wall/`.
    fn config_dir() -> PathBuf {
        let home = std::env::var("HOME")
            .unwrap_or_else(|_| String::from("/tmp/sys-wall-data"));
        let dir = format!("{}/.config/sys-wall", home);
        PathBuf::from(dir)
    }
}
