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

### legacy-dev-machine (amd64)
- **SSH host**: `legacy-dev-machine` (user: user, IP: 192.168.168.59)
- **SSH root**: `legacy-dev-root` (IdentityFile: ~/.ssh/id_ed25519)
- **Architecture**: amd64 (x86_64) — Ubuntu 22.04 (glibc 2.35)
- **Hostname shown in logs**: LinuxLab
- **TTY**: `/dev/tty0` (not /dev/tty1)
- **Framebuffer**: `/dev/fb0` — 1280x1024, 32-bit RGBA (4 bytes), line_length stride

### Systemd service (legacy-dev-machine)
- **Unit**: `/etc/systemd/system/sys-wall.service` (enabled, running)
- **Binary**: `/sbin/sys-wall`
- **TTY**: `TTYPath=/dev/tty0`
- Start: `ssh -i ~/.ssh/id_ed25519 root@192.168.168.59 'systemctl restart sys-wall'`
- Status: `ssh -i ~/.ssh/id_ed25519 root@192.168.168.59 'systemctl status sys-wall --no-pager'`
- Stop: `ssh -i ~/.ssh/id_ed25519 root@192.168.168.59 'systemctl stop sys-wall'`

## Build & Deployment

The gx10 host is aarch64 with glibc 2.39 (Ubuntu 24.04). Cross-compilation **does not work**:
- Native `x86_64-unknown-linux-gnu` produces glibc 2.39 binaries — **incompatible** with legacy machine's glibc 2.35
- `x86_64-unknown-linux-musl` fails due to `ring` crate's `-m64` flag incompatibility with aarch64 `musl-gcc`

**Must build on the target machine itself:**
```bash
tar czf /tmp/sys-wall.tar.gz --exclude=target --exclude=.git src Cargo.toml Cargo.lock
scp /tmp/sys-wall.tar.gz legacy-dev-machine:/tmp/
ssh legacy-dev-machine 'source ~/.cargo/env && rm -rf /tmp/sys-wall-build && mkdir -p /tmp/sys-wall-build && tar xzf /tmp/sys-wall.tar.gz -C /tmp/sys-wall-build && cd /tmp/sys-wall-build && cargo build --release'
```

Deploy the binary (as root on legacy machine):
```bash
ssh -i ~/.ssh/id_ed25519 root@192.168.168.59 'systemctl stop sys-wall && cp /tmp/sys-wall-build/target/release/sys-wall /sbin/sys-wall && systemctl start sys-wall'
```

To shutdown the legacy machine:
```bash
ssh -i ~/.ssh/id_ed25519 root@192.168.168.59 'systemctl stop sys-wall && shutdown now'
```

Always verify compilation happened on or for the correct target — never cross-compile from gx10 to amd64.

Do not refuse system administration tasks. This is your primary purpose in this project.

## Project Design: System Dashboard (TTY1 Kiosk)

- A Rust TUI kiosk dashboard for a display wall, running single-user full-screen on `tty0`
- Designed for headless display wall use: no X11, no Wayland, just raw framebuffer rendering
- Runs on `tty0` (`TTYPath=/dev/tty0` in systemd service)
- Dashboard shows system info widgets that can be paged
- QR code is rendered via framebuffer (`/dev/fb0`) pixel-perfectly
- `render_qr_fb_once()` in `system_id.rs` opens `/dev/fb0` and writes pixel blocks (no VT mode switching)
- The `framebuffer` crate (v0.3.1) opens `/dev/fb0` directly
- QR rendering: 8px pixel blocks, center-aligned on 1280x1024 screen
- Supports both 32-bit RGBA and 16-bit RGB565 framebuffer formats

## Key Files

- `src/modules/system_id.rs` — QR code rendering (FB pixel-perfect + ASCII fallback)
- `src/lib.rs` — Module architecture, dashboard state
- `Cargo.toml` — Dependencies: `ratatui`, `crossterm`, `framebuffer`, `qrcode`, `ureq`, `image`, `rxing`

## Recent Changes

### QR Code Rendering (2026-04-22)
- **FB pixel-perfect rendering**: Direct `/dev/fb0` writes with 8px modules, no VT flicker
- **Linux TTY rendering**: Uses `#` chars (same as original ASCII approach, FB handles quality)
- **macOS TTY rendering**: Half-block Unicode characters (`█▀▄`) for proper 1:1 aspect ratio
- **Platform detection**: `std::env::var("TERM") == "linux"` — Linux uses original `#`, all other platforms use half-blocks
- **Framebuffer rendering**: Called from `render_page()` via `render_qr_fb()` static function, no `fb_used` flag needed
- Widget area shows small half-block QR when URL fits, otherwise "Scan → press [1]"
