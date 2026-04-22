# sys-wall

A lightweight, statically-linked TUI dashboard for Linux systems, designed to run directly on the framebuffer console at boot. Inspired by the [Talos OS](https://www.talos.dev/) console dashboard.

## Screenshots

![Dashboard](screenshots/2.png)
![QR Code Registration](screenshots/1.png)

## Overview

sys-wall provides a system information dashboard that starts automatically when a Linux machine boots. It displays system status, resource monitoring, and network configuration — all in a terminal UI with no desktop environment required.

## Features

- **Summary Dashboard** — Hostname, UUID, uptime, CPU, RAM, IP addresses, and system status at a glance
- **System Monitor** — Real-time CPU, memory, disk I/O, and network graphs (similar to htop)
- **Network Configuration** — Configure hostname, DNS, NTP, network interfaces (DHCP/static)
- **QR Code Registration** — Generate a QR code containing system UUID and basic info, POST-able to a configurable URL
- **Modular Architecture** — Plugin/module system for adding custom tabs

## Tech Stack

- **Rust** — Single static binary with musl target, minimal runtime dependencies
- **[Ratatui](https://ratatui.rs/)** — Terminal UI framework
- **crossterm** — Terminal backend (no ncurses dependency)

## Tabs (F-Keys)

| Key | Tab | Description |
|-----|-----|-------------|
| F1 | Summary | System info overview with logs |
| F2 | Monitor | CPU, memory, disk, network graphs |
| F3 | Network Config | Network interface configuration |
| F4 | QR Code | System registration via QR code |

## Quick Start

```bash
# Prerequisites: Rust 1.75+, musl target
rustup target add x86_64-unknown-linux-musl

# Build
cargo build --release --target x86_64-unknown-linux-musl

# Run locally (development — works in any terminal)
cargo run

# Test in Docker
docker build -t sys-wall .
docker run -it --rm sys-wall
```

See [specs/09-setup.md](specs/09-setup.md) for full development setup instructions.

## Testing

| Method | What it validates | Setup effort |
|--------|-------------------|-------------|
| `cargo run` | TUI rendering, layout, input handling | None |
| Docker (`docker run -it`) | Linux /proc /sys data, static binary | Low |
| QEMU VM | Full boot-to-dashboard, TTY1, network config | Medium |

See [specs/10-testing.md](specs/10-testing.md) for details.

## Deployment

The binary is intended to be placed at `/sbin/sys-wall` and started via a systemd service or init script on TTY1. The recommended approach is to replace getty on TTY1. See [specs/06-deployment.md](specs/06-deployment.md) for systemd, OpenRC, and kernel configuration details.

## Configuration

Configuration is read from `/etc/sys-wall/config.toml`. See [specs/07-configuration.md](specs/07-configuration.md).

## Minimal Requirements

- **Kernel**: Linux 4.9+ with `/proc` and `/sys`
- **Libraries**: None (fully static binary)
- **Disk**: ~5-8 MB
- **RAM**: ~4-8 MB RSS

## Specs

Detailed specifications are in the [specs/](specs/) directory.

| Spec | Description |
|------|-------------|
| [01-architecture](specs/01-architecture.md) | System architecture and module trait |
| [02-dashboard](specs/02-dashboard.md) | F1: Dashboard with module widgets |
| [03-monitor-module](specs/03-monitor-module.md) | F2: Resource monitoring |
| [04-network-module](specs/04-network-module.md) | F3: Network configuration |
| [05-qrcode-module](specs/05-qrcode-module.md) | F4: QR code registration |
| [06-deployment](specs/06-deployment.md) | Deployment and auto-start config |
| [07-configuration](specs/07-configuration.md) | Configuration reference |
| [08-module-system](specs/08-module-system.md) | Module/plugin system |
| [09-setup](specs/09-setup.md) | Development setup |
| [10-testing](specs/10-testing.md) | Testing strategy and CI |

## License

TBD
