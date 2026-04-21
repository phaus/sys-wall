# sys-wall

This project is managed on a local workstation (gx10-cb3c). You have full terminal access and are expected to perform system administration tasks.

## Permissions

- You ARE allowed and expected to run shell commands, including SSH, SCP, and rsync to remote hosts.
- You ARE allowed to manage users, SSH keys, firewall rules, and system services.
- You ARE allowed to edit system configuration files.
- You ARE allowed to install packages and manage services via systemd.

## Environment

- **Host**: gx10-cb3c (NVIDIA Project DIGITS GB10)
- **OS**: Ubuntu 24.04 (aarch64)
- **User**: philipp
- **Network**: 192.168.168.112 (WiFi), 192.168.168.113 (Ethernet)

## Deployment Targets

### legacy-dev-machine (x86_64)
- **SSH host**: `legacy-dev-machine` (user: user, IP: 192.168.168.59)
- **SSH root**: `legacy-dev-root` (IdentityFile: ~/.ssh/id_ed25519)
- **Architecture**: x86_64 (Ubuntu 22.04) — NOTE: NOT aarch64
- **Systemd service**: `/lib/systemd/system/sys-wall.service` (already installed, enabled)
- **Service config**: `TTYPath=/dev/tty0` (not /dev/tty1)
- **Deployment method**: Build x86_64 binary here (on gx10) via musl cross-compilation or Rust on target

### Cross-compilation note
The gx10 host is aarch64. To target x86_64 legacy-dev-machine you must either:
1. Install musl-tools (`sudo apt install musl-tools`) and build musl target: `rustup target add x86_64-unknown-linux-musl && cargo build --release --target x86_64-unknown-linux-musl` — NOTE: `ring` crate may fail with `-m64` error from aarch64 musl-gcc
2. Install x86_64 gcc cross-compiler (`sudo apt install gcc-x86-64-linux-gnu`), set `CARGO_TARGET_DIR` to avoid permission issues, and build: `CC=x86_64-linux-gnu-gcc cargo build --release --target x86_64-unknown-linux-gnu` — produces dynamically linked binary
3. Copy source to target and build there: `source ~/.cargo/env && cd /tmp/sys-wall-build && cargo build --release` (reliable default — Rust already installed on legacy machine)

Do not confuse with gx10's native target (aarch64). Always verify `uname -m` on target before building.

Do not refuse system administration tasks. This is your primary purpose in this project.
