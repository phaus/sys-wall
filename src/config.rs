/// sys-wall configuration loader.
///
/// Loads config from `$HOME/.config/sys-wall/config.toml`. Creates the
/// file and a per-system UUID on first run, then persists on every load.
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use crate::collect_kernel_version;

use toml::map::Map;
use toml::Value;

const DEFAULT_REFRESH_INTERVAL_MS: u64 = 1000;
const DEFAULT_SYSTEM_URL: &str = "https://debug.consolving.net/system";

#[derive(Clone, Debug)]
pub struct Config {
    pub system_id: String,
    /// True when this file was just created on this run (first-time install).
    pub is_first_run: bool,
    pub refresh_interval_ms: u64,
    pub default_tab: String,
    pub system_url: String,
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_dir = Self::config_dir();
        fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join("config.toml");
        let mut is_first_run = !config_path.exists();

        // Defaults
        let mut refresh_interval_ms = DEFAULT_REFRESH_INTERVAL_MS;
        let mut default_tab = String::from("summary");
        let mut system_id = String::new();
        let system_url = Self::load_system_url();

        // Parse existing config if present.
        let parsed: Value = if config_path.exists() {
            fs::read_to_string(&config_path)?
                .parse::<Value>()?
        } else {
            Value::Table(Map::new())
        };

        // Extract fingerprint from existing config.
        let parsed_fingerprint: Option<String> = parsed
            .get("fingerprint")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Extract system_id (top-level key).
        if let Some(sid) = parsed.get("system_id").and_then(|v| v.as_str()) {
            system_id = sid.to_string();
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

        // Compute current hardware fingerprint.
        let current_fingerprint = Self::compute_fingerprint();

        // Check if fingerprint changed → regenerate UUID.
        if fingerprint_changed(&parsed_fingerprint, &current_fingerprint) {
            system_id = uuid::Uuid::new_v4().to_string();
            is_first_run = true;
        }

        // Persist with fingerprint.
        let mut table = Map::new();
        table.insert(
            "system_id".into(),
            Value::String(system_id.clone()),
        );
        table.insert(
            "fingerprint".into(),
            Value::String(current_fingerprint.clone()),
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
            system_url,
        })
    }

    /// Return `$HOME/.config/sys-wall/`.
    fn config_dir() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| {
            // When running as root (systemd service), default to /root
            std::fs::read_to_string("/proc/self/loginuid")
                .map(|uid| {
                    if uid.trim() == "0" {
                        return "/root".to_string();
                    }
                    String::from("/tmp/sys-wall-data")
                })
                .unwrap_or_else(|_| String::from("/tmp/sys-wall-data"))
        });
        let dir = format!("{}/.config/sys-wall", home);
        PathBuf::from(dir)
    }

    /// Compute a fingerprint from hostname, MAC, and kernel version.
    fn compute_fingerprint() -> String {
        let hostname = std::fs::read_to_string("/etc/hostname")
            .map(|h| h.trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        let mac = Self::collect_hostname_iface_mac();
        let kernel = collect_kernel_version();
        format!("{hostname}|{mac}|{kernel}")
    }

    /// Read MAC of first non-loopback iface, used for fingerprinting.
    fn collect_hostname_iface_mac() -> String {
        if let Ok(dir) = fs::read_dir("/sys/class/net") {
            for entry in dir.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name == "lo" {
                    continue;
                }
                if let Ok(mac) =
                    fs::read_to_string(entry.path().join("address"))
                {
                    return mac.trim().to_string();
                }
             }
         }
          String::new()
      }

    /// Load system_url from sysid.toml config file.
    /// Creates the file if it doesn't exist, always persists the current URL.
    fn load_system_url() -> String {
        let config_dir = Self::config_dir();
        let _ = fs::create_dir_all(&config_dir);
        let config_path = config_dir.join("sysid.toml");

        // Environment variable override
        if let Ok(val) = std::env::var("SYSWALL_SYSTEM_URL") {
            if !val.is_empty() {
                let _ = Self::write_sysid_config(&config_path, &val);
                return val;
            }
        }

        let mut url = String::from(DEFAULT_SYSTEM_URL);

        // Check /etc/sys-wall/sysid.toml first (system-level config, highest precedence)
        let etc_path = PathBuf::from("/etc/sys-wall/sysid.toml");
        if etc_path.exists() {
            if let Ok(content) = fs::read_to_string(&etc_path) {
                if let Ok(parsed) = content.parse::<toml::Value>() {
                    if let Some(s) = parsed.get("system_url").and_then(|v| v.as_str()) {
                        if !s.is_empty() {
                            url = s.to_string();
                        }
                    }
                }
            }
        }

        // Fall back to user config dir (overwrites system config if user sets one)
        if config_path.exists() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                if let Ok(parsed) = content.parse::<toml::Value>() {
                    if let Some(s) = parsed.get("system_url").and_then(|v| v.as_str()) {
                        if !s.is_empty() {
                            url = s.to_string();
                        }
                    }
                }
            }
        }

        // Always persist (creates file if missing, updates if changed)
        let _ = Self::write_sysid_config(&config_path, &url);
        url
    }

    /// Write the sysid.toml config file. Returns Ok on success, ignores errors.
    fn write_sysid_config(config_path: &PathBuf, system_url: &str) {
        let _ = (|| -> Result<(), Box<dyn std::error::Error>> {
            let mut table = toml::map::Map::new();
            table.insert("system_url".into(), toml::Value::String(system_url.to_string()));
            let output = toml::to_string_pretty(&toml::Value::Table(table))?;
            fs::write(config_path, output)?;
            Ok(())
        })();
    }
}

/// Check if stored fingerprint differs from current hardware fingerprint.
fn fingerprint_changed(stored: &Option<String>, current: &str) -> bool {
    match stored {
        Some(s) => s != current,
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_changed_no_stored() {
        assert!(fingerprint_changed(&None, "anything"));
    }

    #[test]
    fn fingerprint_changed_identical_strings() {
        assert!(!fingerprint_changed(
            &Some("LinuxLab|00:11:22:33:44:55|5.10.0".to_string()),
            "LinuxLab|00:11:22:33:44:55|5.10.0"
        ));
    }

    #[test]
    fn fingerprint_changed_different_mac() {
        assert!(fingerprint_changed(
            &Some("LinuxLab|00:11:22:33:44:55|5.10.0".to_string()),
            "LinuxLab|aa:bb:cc:dd:ee:ff|5.10.0"
        ));
    }

    #[test]
    fn fingerprint_changed_different_kernel() {
        assert!(fingerprint_changed(
            &Some("LinuxLab|00:11:22:33:44:55|5.10.0".to_string()),
            "LinuxLab|00:11:22:33:44:55|5.15.0"
        ));
    }

    #[test]
    fn fingerprint_changed_different_hostname() {
        assert!(fingerprint_changed(
            &Some("LinuxLab|00:11:22:33:44:55|5.10.0".to_string()),
            "Server2|00:11:22:33:44:55|5.10.0"
        ));
    }

    #[test]
    fn config_toml_parse_system_id() {
        let toml_str = r#"
system_id = "abcd-1234"

[general]
refresh_interval_ms = 2000
default_tab = "monitor"
"#;
        let parsed: Value = toml_str.parse().unwrap();
        assert_eq!(
            parsed.get("system_id").and_then(|v| v.as_str()),
            Some("abcd-1234")
        );
    }

    #[test]
    fn config_toml_parse_general_section() {
        let toml_str = r#"
system_id = "uuid-here"

[general]
refresh_interval_ms = 5000
default_tab = "network"
"#;
        let parsed: Value = toml_str.parse().unwrap();
        let general = parsed.get("general").unwrap().as_table().unwrap();
        let interval = general
            .get("refresh_interval_ms")
            .and_then(|v| v.as_integer())
            .unwrap();
        assert_eq!(interval, 5000);
        let tab = general
            .get("default_tab")
            .and_then(|v| v.as_str())
            .unwrap();
        assert_eq!(tab, "network");
    }

    #[test]
    fn config_toml_missing_section() {
        let toml_str = r#"
system_id = "test"
"#;
        let parsed: Value = toml_str.parse().unwrap();
        assert!(parsed.get("general").is_none());
        assert!(parsed
            .get("general")
            .and_then(|v| v.as_table())
            .is_none());
    }

    #[test]
    fn config_toml_missing_fingerprint() {
        let toml_str = r#"
system_id = "my-id"

[general]
refresh_interval_ms = 1000
"#;
        let parsed: Value = toml_str.parse().unwrap();
        assert!(parsed.get("fingerprint").is_none());
    }

    #[test]
    fn config_toml_all_fields() {
        let toml_str = r#"
system_id = "abcd-1234"
fingerprint = "host|mac|5.10"

[general]
refresh_interval_ms = 3000
default_tab = "monitor"
"#;
        let parsed: Value = toml_str.parse().unwrap();
        assert_eq!(
            parsed.get("system_id").and_then(|v| v.as_str()),
            Some("abcd-1234")
        );
        assert_eq!(
            parsed.get("fingerprint").and_then(|v| v.as_str()),
            Some("host|mac|5.10")
        );
    }
}
