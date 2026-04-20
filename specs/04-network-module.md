# Network Configuration Module (F3)

## Purpose

Allow basic network configuration from the console without requiring SSH or external tools.

## Layout

```
┌─ Configure (Ctrl+Q) ──┬─ Existing Config (Ctrl+W) ─┬─ New Config (Ctrl+E) ──┐
│                        │                             │                        │
│ Hostname  [________]   │ (current network config     │ (YAML preview of       │
│ DNS       [________]   │  displayed as YAML or       │  the new config that   │
│ Time Srv  [________]   │  key-value pairs)           │  will be applied)      │
│ Interface [eth0    v]   │                             │                        │
│ Mode      [DHCP    v]   │                             │                        │
│                        │                             │                        │
│ --- Static Mode ---    │                             │                        │
│ IP Addr   [________]   │                             │                        │
│ Netmask   [________]   │                             │                        │
│ Gateway   [________]   │                             │                        │
│                        │                             │                        │
│ [ Save ]               │                             │                        │
└────────────────────────┴─────────────────────────────┴────────────────────────┘
```

## Features

- List available network interfaces
- Toggle between DHCP and static IP
- Configure: hostname, DNS servers, NTP servers, IP, netmask, gateway
- Show current config vs. new config side-by-side
- Apply config via `networkd`, `NetworkManager`, or direct `ip` commands

## Data Sources & Actions

| Field | Read From | Write To |
|-------|-----------|----------|
| Hostname | `gethostname()` | `sethostname()` + `/etc/hostname` |
| DNS | `/etc/resolv.conf` | `/etc/resolv.conf` |
| NTP | `/etc/chrony.conf` or `/etc/ntp.conf` | Same |
| Interface | `sysinfo` / `getifaddrs()` | `ip addr` / networkd config |
| Mode | Current interface state | networkd / NetworkManager |
| IP/Mask/GW | Interface query | `ip addr add`, `ip route` |

## Input Handling

- `Tab` / `Shift+Tab`: Navigate between fields
- `Enter`: Activate field / confirm selection
- `Ctrl+S` or selecting `Save`: Apply configuration
- `Esc`: Cancel / return to summary
- Dropdown fields use arrow keys for selection

## Validation

- IP addresses: valid IPv4/IPv6 format
- Netmask: valid CIDR or dotted notation
- Hostname: RFC 1123 compliant
- Show validation errors inline

## Configuration

```toml
[network]
backend = "networkd"  # "networkd" | "networkmanager" | "direct"
allowed_interfaces = ["eth*", "en*"]  # glob patterns
```
