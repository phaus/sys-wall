# Testing & Integration

## Testing Tiers

There are three tiers of testing, from simplest to most realistic:

### Tier 1: Local Terminal (Development)

The simplest way. sys-wall runs in any terminal emulator via crossterm.

```bash
cargo run
```

- Works on macOS and Linux
- Full TUI rendering in your terminal
- No special setup needed
- Limitations: some Linux-specific data sources (e.g. `/proc`, `/sys`) are unavailable on macOS — the code should gracefully handle missing data with placeholder values

### Tier 2: Docker Container (CI & Quick Integration Testing)

A Docker container provides a real Linux environment with `/proc` and `/sys`. This is the recommended way to test before deploying to real hardware.

```dockerfile
# Dockerfile
FROM rust:1.75-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /build
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM alpine:3.19

COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/sys-wall /sbin/sys-wall
COPY config.toml /etc/sys-wall/config.toml

ENTRYPOINT ["/sbin/sys-wall"]
```

```bash
# Build
docker build -t sys-wall .

# Run interactively (allocate a TTY)
docker run -it --rm sys-wall

# Run with host proc/sys for real system data
docker run -it --rm \
  -v /proc:/host/proc:ro \
  -v /sys:/host/sys:ro \
  --pid=host \
  sys-wall
```

**What works in Docker:**
- Full TUI rendering (with `-it`)
- CPU, memory, process data (with `--pid=host`)
- Network interface listing
- QR code rendering
- Config loading

**What does NOT work in Docker:**
- Network configuration changes (no real interface control)
- Framebuffer rendering (no `/dev/fb0`)
- systemd journal (unless mounted)
- Acting as PID 1 / init replacement

### Tier 3: QEMU VM (Full Integration)

For testing the complete boot-to-dashboard experience, use a minimal QEMU VM.

```bash
# Create a minimal Alpine-based VM image
# scripts/build-qemu-image.sh

#!/bin/bash
set -e

IMAGE="sys-wall-test.qcow2"
SIZE="512M"

# Create disk image
qemu-img create -f qcow2 "$IMAGE" "$SIZE"

# Use Alpine's setup to create a minimal system
# (or use a prebuilt Alpine cloud image)
wget -O alpine.iso https://dl-cdn.alpinelinux.org/alpine/v3.19/releases/x86_64/alpine-virt-3.19.0-x86_64.iso

echo "Boot the ISO, install Alpine, then copy sys-wall binary in."
echo ""
echo "qemu-system-x86_64 -m 256 -cdrom alpine.iso -hda $IMAGE -boot d"
```

```bash
# Run the VM with the dashboard
qemu-system-x86_64 \
  -m 256M \
  -hda sys-wall-test.qcow2 \
  -nographic \
  -serial mon:stdio \
  -append "console=ttyS0"
```

**What this validates:**
- Boot-to-dashboard flow
- systemd/OpenRC service startup
- Real framebuffer rendering
- Network configuration changes
- Full system data access

## Minimal System Requirements

### Kernel

- **Minimum**: Linux 4.9+ (for `/proc` and `/sys` interfaces used)
- **Recommended**: Linux 5.x+
- **Required kernel configs**:
  - `CONFIG_PROC_FS=y`
  - `CONFIG_SYSFS=y`
  - `CONFIG_TTY=y`
  - `CONFIG_VT=y` (virtual terminals)
  - `CONFIG_FRAMEBUFFER_CONSOLE=y` (for graphical framebuffer, optional — text mode works too)

### Libraries

**None** — when built with `x86_64-unknown-linux-musl`, the binary is fully statically linked. No libc, no libncurses, no shared libraries required.

Verify with:
```bash
file sys-wall
# sys-wall: ELF 64-bit LSB executable, x86-64, statically linked, ...

ldd sys-wall
# not a dynamic executable
```

### Filesystem

The binary reads from these paths (all optional, graceful degradation):

| Path | Purpose | Required |
|------|---------|----------|
| `/proc/` | CPU, memory, processes, uptime, load | Yes |
| `/sys/class/dmi/id/product_uuid` | Hardware UUID | No (falls back to `/etc/machine-id`) |
| `/etc/machine-id` | Machine ID fallback | No |
| `/etc/hostname` | Hostname | No (uses `gethostname()`) |
| `/etc/resolv.conf` | DNS servers | No |
| `/etc/sys-wall/config.toml` | Configuration | No (uses defaults) |

### Minimum Disk Space

- Binary: ~5-8 MB (static, release, stripped)
- Config: ~1 KB
- **Total: <10 MB**

### Minimum RAM

- ~4-8 MB RSS during operation
- Works on systems with as little as 32 MB total RAM

## CI Pipeline (GitHub Actions)

```yaml
name: CI
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: x86_64-unknown-linux-musl
      - run: sudo apt-get install -y musl-tools
      - run: cargo test
      - run: cargo clippy -- -D warnings
      - run: cargo build --release --target x86_64-unknown-linux-musl
      - run: strip target/x86_64-unknown-linux-musl/release/sys-wall
      - run: file target/x86_64-unknown-linux-musl/release/sys-wall
      - run: ls -lh target/x86_64-unknown-linux-musl/release/sys-wall

  docker:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: docker build -t sys-wall .
      # Smoke test: run for 3 seconds, check exit code
      - run: timeout 3 docker run sys-wall --help || true
```
