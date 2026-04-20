use std::sync::LazyLock;
use std::time::Duration;

pub mod config;
pub mod modules;

pub use config::Config;
pub use crossterm::event::{Event, KeyCode};
pub use traits::{Module, ModuleCapability, WidgetSize};

/// Shared sysinfo instance reused across ticks for accurate CPU/memory deltas.
static SYSTEM: LazyLock<std::sync::Mutex<sysinfo::System>> =
    LazyLock::new(|| std::sync::Mutex::new(sysinfo::System::new_all()));

/// Shared data passed to all modules on each update cycle.
#[derive(Clone)]
pub struct SystemContext {
    pub hostname: String,
    pub uuid: String,
    pub uptime: Duration,
    pub process_count: u32,
    pub cpu_usage: f32,
    pub cpu_per_core: Vec<f32>,
    pub memory_used: u64,
    pub memory_total: u64,
    pub load_avg: (f64, f64, f64),
    pub kernel_version: String,
    pub os_name: String,
    pub os_version: String,
    pub os_codename: String,
    pub ip_addresses: Vec<String>,
    pub primary_mac: String,
    pub ipv4_gateway: String,
    pub ipv6_gateway: String,
    pub dns_servers: Vec<String>,
    pub tty_path: String,
    pub config: std::sync::Arc<Config>,
}

/// Helper: parse `/etc/os-release`
fn parse_os_release() -> (String, String, String) {
    let content = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    let mut name = "Linux".to_string();
    let mut version = String::new();
    let mut codename = String::new();
    for line in content.lines() {
        match line {
            l if l.starts_with("NAME=") => {
                name = l.strip_prefix("NAME=").unwrap_or_default().trim_matches('"').to_string()
            }
            l if l.starts_with("VERSION=") => {
                version = l.strip_prefix("VERSION=").unwrap_or_default().trim_matches('"').to_string()
            }
            l if l.starts_with("VERSION_CODENAME=") => {
                codename = l
                    .strip_prefix("VERSION_CODENAME=")
                    .unwrap_or_default()
                    .trim_matches('"')
                    .to_string()
            }
            _ => {}
        }
    }
    (name, version, codename)
}

/// Collect non-loopback IPs from `hostname -I`, falling back to `ip addr show`.
fn collect_ips() -> Vec<String> {
    let mut ips = Vec::new();
    if let Ok(out) = std::process::Command::new("hostname").arg("-I").output() {
        if out.status.success() {
            for ip in String::from_utf8_lossy(&out.stdout).split_ascii_whitespace() {
                let ip = ip.trim();
                if !ip.is_empty() && !ip.starts_with("127.") {
                    ips.push(ip.to_string());
                }
            }
        }
    }
    if ips.is_empty() {
        if let Ok(out) = std::process::Command::new("ip")
            .args(&["-o", "-4", "addr", "show"])
            .output()
        {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                if let Some(start) = line.find("inet ") {
                    let rest = &line[start + 5..];
                    let ip = rest.split('/').next().unwrap_or("").trim();
                    if !ip.starts_with("127.") && !ip.is_empty() {
                        ips.push(ip.to_string());
                    }
                }
            }
        }
    }
    ips
}

/// Collect the default IPv4 and IPv6 gateways via `ip route`.
fn collect_gateways() -> (String, String) {
    let mut v4 = String::new();
    let mut v6 = String::new();

    if let Ok(out) = std::process::Command::new("ip")
        .args(&["-4", "route", "list", "default"])
        .output()
    {
        if out.status.success() {
            if let Some(start) = String::from_utf8_lossy(&out.stdout).find("via ") {
                let rest = &String::from_utf8_lossy(&out.stdout)[start + 4..];
                v4 = rest.split_whitespace().next().unwrap_or("").to_string();
            }
        }
    }

    if let Ok(out) = std::process::Command::new("ip")
        .args(&["-6", "route", "list", "default"])
        .output()
    {
        if out.status.success() {
            if let Some(start) = String::from_utf8_lossy(&out.stdout).find("via ") {
                let rest = &String::from_utf8_lossy(&out.stdout)[start + 4..];
                v6 = rest.split_whitespace().next().unwrap_or("").to_string();
            }
        }
    }

    (v4, v6)
}

/// Parse `/etc/resolv.conf` for nameserver lines (and systemd-resolved stub).
fn collect_dns_servers() -> Vec<String> {
    let mut servers = Vec::new();
    if let Ok(content) = std::fs::read_to_string("/etc/resolv.conf") {
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("nameserver ") {
                if let Some(server) = line.split_whitespace().nth(1) {
                    servers.push(server.to_string());
                }
            }
        }
    }

    // Also check systemd-resolved stub if /etc/resolv.conf points there.
    if let Ok(target) = std::fs::read_link("/etc/resolv.conf") {
        if target.ends_with("run/systemd/resolve/resolv.conf") {
            if let Ok(content) = std::fs::read_to_string("/run/systemd/resolve/resolv.conf") {
                for line in content.lines() {
                    let line = line.trim();
                    if line.starts_with("DNS=") {
                        let server = line["DNS=".len()..].split_whitespace().next().unwrap_or("");
                        if !server.is_empty() {
                            let s = server.to_string();
                            if !servers.contains(&s) {
                                servers.push(s);
                            }
                        }
                    }
                }
            }
        }
    }

    servers
}

/// Return the MAC address of the first non-loopback interface.
fn collect_primary_mac() -> String {
    if let Ok(dir) = std::fs::read_dir("/sys/class/net") {
        for entry in dir.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "lo" {
                continue;
            }
            if let Ok(mac) = std::fs::read_to_string(entry.path().join("address")) {
                return mac.trim().to_string();
            }
        }
    }
    "n/a".to_string()
}

/// Detect the TTY device this process is running on.
/// Checks /proc/self/stat for tty_nr field, falls back to service file,
/// then to /dev/pts, and finally to a default.
fn detect_tty() -> String {
    // Method 1: Read /proc/self/stat field 7 (tty_nr)
    if let Ok(stat) = std::fs::read_to_string("/proc/self/stat") {
        if let Some(first_paren) = stat.find('(') {
            if let Some(second_paren) = stat[first_paren..].find(')') {
                let rest = &stat[first_paren + second_paren + 2..];
                let fields: Vec<&str> = rest.split_whitespace().collect();
                if fields.len() >= 1 {
                    if let Ok(tty_nr) = fields[0].parse::<i64>() {
                        if tty_nr != 0 {
                            // tty_nr is a kernel device number; map it if possible
                            // Major 4: ttys, Minor 0-63: tty0-tty63
                            // 4 * 256 + minor = device number
                            let minor = tty_nr % 256;
                            if minor >= 1 && minor <= 63 {
                                return format!("/dev/tty{}", minor);
                            }
                        }
                    }
                }
            }
        }
    }
    // Method 2: Read TTYPath from common systemd service locations
    for path in [
        "/lib/systemd/system/sys-wall.service",
        "/etc/systemd/system/sys-wall.service",
    ] {
        if let Ok(content) = std::fs::read_to_string(path) {
            for line in content.lines() {
                if let Some(value) = line.strip_prefix("TTYPath=") {
                    if !value.is_empty() {
                        return value.trim().to_string();
                    }
                }
            }
        }
    }
    // Method 3: Check /dev/pts (only set for pseudo-terminals, not for VT)
    if let Ok(link) = std::fs::read_link("/proc/self/fd/0") {
        let s = link.to_string_lossy();
        if s.starts_with("/dev/pts/") {
            return s.to_string();
        }
    }
    // Method 4: Default
    "/dev/tty1".to_string()
}

/// Helper: format Duration as d d h h m m.
pub fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    format!("{days}d {hours}h {mins}m", days = days, hours = hours, mins = mins)
}

impl SystemContext {
    pub fn new(config: Config) -> Self {
        let hostname = std::fs::read_to_string("/etc/hostname")
            .map(|h| h.trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        let uuid = uuid::Uuid::new_v4().to_string();
        let kernel = std::env::var("KERNEL_VERSION").unwrap_or_else(|_| "unknown".to_string());
        let (os_name, os_version, os_codename) = parse_os_release();

        // Refresh shared sysinfo for live CPU, memory, uptime, process count.
        let mut sys = SYSTEM.lock().unwrap();
        sys.refresh_cpu_all();
        sys.refresh_memory();
        let cpu_usage = sys.global_cpu_usage();
        let cpu_per_core: Vec<f32> = sys.cpus().iter().map(|c| c.cpu_usage()).collect();
        let memory_total = sys.total_memory();
        let memory_used = sys.used_memory();
        let (uptime_secs, process_count) = {
            let up = sysinfo::System::uptime();
            let pc = sys.processes().len() as u32;
            (up, pc)
        };
        drop(sys);

        let (ipv4_gateway, ipv6_gateway) = collect_gateways();
        let dns_servers = collect_dns_servers();
        let primary_mac = collect_primary_mac();
        let tty_path = detect_tty();

        Self {
            hostname,
            uuid,
            uptime: Duration::from_secs(uptime_secs as u64),
            process_count,
            cpu_usage,
            cpu_per_core,
            memory_used,
            memory_total,
            load_avg: (0.0, 0.0, 0.0),
            kernel_version: kernel,
            config: std::sync::Arc::new(config),
            os_name,
            os_version,
            os_codename,
            ip_addresses: collect_ips(),
            primary_mac,
            ipv4_gateway,
            ipv6_gateway,
            dns_servers,
            tty_path,
        }
    }
}

mod traits {
    use crossterm::event;
    use ratatui::layout::Rect;
    use std::error::Error;

    /// Defines what a module can render.
    #[derive(PartialEq)]
    pub enum ModuleCapability {
        PageOnly,
        WidgetOnly,
        WidgetAndPage,
    }

    /// Size hint for widget layout on the Dashboard.
    pub enum WidgetSize {
        Small,
        Medium,
        Large,
    }

    impl WidgetSize {
        pub fn height(self) -> u16 {
            match self {
                WidgetSize::Small => 5,
                WidgetSize::Medium => 8,
                WidgetSize::Large => 14,
            }
        }
    }

    /// Trait that every sys-wall module must implement.
    pub trait Module {
        fn name(&self) -> &str;

        fn keybinding(&self) -> Option<event::KeyCode>;

        fn capability(&self) -> ModuleCapability;

        fn widget_size(&self) -> WidgetSize {
            WidgetSize::Small
        }

        fn widget_height(&self) -> u16 {
            self.widget_size().height()
        }

        fn update(&mut self, ctx: &super::SystemContext) -> Result<(), Box<dyn Error>>;

        fn render_widget(&self, frame: &mut ratatui::Frame<'_>, area: Rect);

        fn render_page(&self, frame: &mut ratatui::Frame<'_>, area: Rect);

        fn handle_input(
            &mut self,
            _event: &event::Event,
        ) -> Result<bool, Box<dyn Error>> {
            Ok(false)
        }
    }
}
