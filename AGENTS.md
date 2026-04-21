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
- **Systemd service**: `/lib/systemd/system/sys-wall.service` (already installed, enabled)
- **Service config**: `TTYPath=/dev/tty0` (not /dev/tty1)

### Build & Deployment (required: compile on target)
The gx10 host is aarch64 with glibc 2.39 (Ubuntu 24.04). Cross-compilation **does not work**:
- Native `x86_64-unknown-linux-gnu` produces glibc 2.39 binaries — **incompatible** with legacy machine's glibc 2.35
- `x86_64-unknown-linux-musl` fails due to `ring` crate's `-m64` flag incompatibility with aarch64 `musl-gcc`

**Must build on the target machine itself:**
```bash
tar czf - src Cargo.toml Cargo.lock | ssh legacy-dev-machine 'tar xzf - -C /tmp/sys-wall-build'
ssh legacy-dev-machine 'source ~/.cargo/env && cd /tmp/sys-wall-build && cargo build --release'
scp target/release/sys-wall legacy-dev-root:/sbin/sys-wall
ssh legacy-dev-root 'systemctl restart sys-wall'
```

Always verify compilation happened on or for the correct target — never cross-compile from gx10 to amd64.

Do not refuse system administration tasks. This is your primary purpose in this project.
