# Deployment

## Target Environment

- Headless Linux servers or embedded systems
- No desktop environment, no X11/Wayland
- Runs on TTY1 (framebuffer console)

## Binary Installation

```bash
install -m 755 sys-wall /sbin/sys-wall
mkdir -p /etc/sys-wall
cp config.toml /etc/sys-wall/config.toml
```

## Making sys-wall Start First on Boot

There are multiple strategies depending on the init system and how "first" you need it to be.

### Option A: Replace Getty on TTY1 (Recommended)

This is the simplest and most common approach. sys-wall replaces the login prompt on TTY1 while other TTYs remain available for login.

```bash
# 1. Install the service
cat > /etc/systemd/system/sys-wall.service << 'EOF'
[Unit]
Description=sys-wall System Dashboard
After=systemd-logind.service
Conflicts=getty@tty1.service
After=getty@tty1.service

[Service]
Type=simple
ExecStart=/sbin/sys-wall
StandardInput=tty
StandardOutput=tty
TTYPath=/dev/tty1
TTYReset=yes
TTYVHangup=yes
TTYVTDisallocate=yes
UtmpIdentifier=tty1
Restart=always
RestartSec=2

# Security hardening (sys-wall only needs read access to /proc, /sys)
ProtectHome=yes
ProtectSystem=strict
ReadWritePaths=/etc/sys-wall /etc/hostname /etc/resolv.conf
CapabilityBoundingSet=CAP_NET_ADMIN CAP_SYS_ADMIN
AmbientCapabilities=CAP_NET_ADMIN

[Install]
WantedBy=multi-user.target
EOF

# 2. Disable getty on TTY1, enable sys-wall
systemctl disable getty@tty1
systemctl enable sys-wall
systemctl start sys-wall

# Login is still available on TTY2-6 (Ctrl+Alt+F2)
```

### Option B: Getty Autologin + .profile (Simpler, Less Clean)

Use getty's autologin to run sys-wall as a user's shell:

```bash
# /etc/systemd/system/getty@tty1.service.d/override.conf
[Service]
ExecStart=
ExecStart=-/sbin/agetty --autologin root --noclear %I $TERM

# /root/.bash_profile (add at the end)
if [ "$(tty)" = "/dev/tty1" ]; then
    exec /sbin/sys-wall
fi
```

### Option C: Init Replacement (Embedded/Appliance)

For appliance-like systems where sys-wall should be the primary interface:

```bash
# Kernel command line (in GRUB or bootloader):
# init=/sbin/sys-wall-init

# /sbin/sys-wall-init is a wrapper:
#!/bin/sh
mount -t proc proc /proc
mount -t sysfs sys /sys
mount -t devtmpfs dev /dev

# Start sys-wall on TTY1
/sbin/sys-wall </dev/tty1 >/dev/tty1 2>&1 &

# Continue with normal init
exec /sbin/init
```

This is rarely needed — Option A covers most use cases.

### Option D: OpenRC (Alpine Linux)

```bash
# /etc/init.d/sys-wall
#!/sbin/openrc-run

command="/sbin/sys-wall"
command_args=""
pidfile="/run/sys-wall.pid"
command_background=true

depend() {
    need localmount
    after bootmisc
}

start_pre() {
    # Take over TTY1
    rc-service agetty.tty1 stop 2>/dev/null || true
}
```

```bash
rc-update add sys-wall default
rc-update del agetty.tty1 default
```

## Console / TTY Configuration

### Kernel Command Line Parameters

Relevant kernel boot parameters for sys-wall:

```
# /etc/default/grub — GRUB_CMDLINE_LINUX additions:

# Ensure framebuffer console is available
fbcon=map:0                    # Map framebuffer to first console
vga=normal                     # Or: vga=0x31B for 1280x1024

# Disable graphical splash (we ARE the splash)
nosplash
plymouth.enable=0

# Keep kernel messages on tty1 minimal (sys-wall handles display)
loglevel=1                     # Only KERN_ALERT and KERN_EMERG
console=tty1                   # Console output goes to tty1
quiet                          # Suppress most boot messages
```

After editing GRUB:
```bash
update-grub
# or for EFI:
grub-mkconfig -o /boot/efi/EFI/<distro>/grub.cfg
```

### Ensuring TTY1 is Available

```bash
# Verify virtual terminals are set up
cat /proc/sys/kernel/printk    # Should show low log level
ls /dev/tty1                   # Must exist

# If using systemd, ensure logind doesn't reserve TTY1
mkdir -p /etc/systemd/logind.conf.d
cat > /etc/systemd/logind.conf.d/sys-wall.conf << 'EOF'
[Login]
NAutoVTs=5
ReserveVT=0
EOF
```

### Capabilities Required

sys-wall needs specific Linux capabilities depending on features:

| Capability | Needed For | Required |
|------------|-----------|----------|
| (none) | Summary, Monitor, QR Code | Baseline — read-only |
| `CAP_NET_ADMIN` | Network configuration changes | Only for Network module |
| `CAP_SYS_ADMIN` | Setting hostname | Only for Network module |

For read-only dashboard mode, sys-wall can run as an unprivileged user.

## Build Targets

| Target | Use Case |
|--------|----------|
| `x86_64-unknown-linux-musl` | Standard amd64 servers |
| `aarch64-unknown-linux-musl` | ARM64 / Raspberry Pi |

## Cross-Compilation

```bash
# Install cross
cargo install cross

# Build for ARM64
cross build --release --target aarch64-unknown-linux-musl

# Strip binary for minimal size
strip target/x86_64-unknown-linux-musl/release/sys-wall
```

## Binary Size Optimization

```toml
# Cargo.toml — [profile.release]
[profile.release]
opt-level = "z"        # Optimize for size
lto = true             # Link-time optimization
codegen-units = 1      # Single codegen unit for better optimization
panic = "abort"        # No unwinding = smaller binary
strip = true           # Strip symbols
```

Expected binary size: **3-6 MB** (statically linked, stripped).
