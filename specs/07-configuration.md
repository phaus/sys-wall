# Configuration

## Config File Location

```
/etc/sys-wall/config.toml
```

Falls back to `./config.toml` for development.

## Full Example

```toml
[general]
hostname_override = ""      # Empty = use system hostname
refresh_interval_ms = 1000
default_tab = "summary"     # "summary" | "monitor" | "network" | "qrcode"

[summary]
log_source = "journal"      # "journal" | "syslog" | "kmsg"
log_lines = 200
custom_fields = [
    { key = "CLUSTER", value = "n/a" },
    { key = "STAGE", value = "Running" },
]

[monitor]
update_interval_ms = 1000
process_count = 10
history_seconds = 60

[network]
backend = "networkd"        # "networkd" | "networkmanager" | "direct"
allowed_interfaces = ["eth*", "en*", "wl*"]

[qrcode]
target_url = "https://example.com/api/register"
mode = "url"
auto_post = false
extra_fields = { environment = "production", site = "dc-01" }

[modules]
# Enable/disable built-in modules
summary = true
monitor = true
network = true
qrcode = true

# External module paths (future)
# external = ["/usr/lib/sys-wall/modules/custom.so"]
```

## Environment Variable Overrides

All config values can be overridden via environment variables with the prefix `SYSWALL_`:

```bash
SYSWALL_GENERAL_DEFAULT_TAB=monitor
SYSWALL_QRCODE_TARGET_URL=https://other.example.com/reg
```

## Command-Line Arguments

```
sys-wall [OPTIONS]

Options:
    -c, --config <PATH>    Path to config file (default: /etc/sys-wall/config.toml)
    -t, --tab <TAB>        Start on specific tab
    -v, --version          Print version
    -h, --help             Print help
```
