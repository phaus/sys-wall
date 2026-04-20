# Development Setup

## Prerequisites

- Rust toolchain (stable, 1.75+)
- For static builds: musl target
- For container testing: Docker or Podman
- For full integration testing: QEMU (optional)

## Initial Setup

```bash
# Clone the repository
git clone https://github.com/phaus/sys-wall.git
cd sys-wall

# Install Rust (if not present)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add musl target for static linking
rustup target add x86_64-unknown-linux-musl
rustup target add aarch64-unknown-linux-musl

# Install musl toolchain (macOS)
brew install filosottile/musl-cross/musl-cross

# Install musl toolchain (Ubuntu/Debian)
sudo apt install musl-tools

# Install cross-compilation tool (optional, for ARM builds)
cargo install cross
```

## Building

```bash
# Development build (native, dynamic linking — runs on your host)
cargo build

# Run locally in your terminal (works on macOS/Linux for development)
cargo run

# Release build (native)
cargo build --release

# Static Linux binary (for deployment)
cargo build --release --target x86_64-unknown-linux-musl

# ARM64 static binary
cross build --release --target aarch64-unknown-linux-musl
```

## Running Locally

sys-wall uses crossterm, so it works in any modern terminal emulator during development:

```bash
# Run with default config
cargo run

# Run with custom config
cargo run -- --config ./dev-config.toml

# Run starting on a specific tab
cargo run -- --tab monitor
```

## Project Structure

```
sys-wall/
├── Cargo.toml
├── config.toml              # Dev config
├── src/
│   ├── main.rs              # Entry point, event loop
│   ├── app.rs               # App state, tab management
│   ├── config.rs            # Config loading (TOML, ENV, CLI)
│   ├── context.rs           # SystemContext (shared data)
│   ├── module.rs            # Module trait definition
│   └── modules/
│       ├── mod.rs            # Module registry
│       ├── summary.rs        # F1: Summary dashboard
│       ├── monitor.rs        # F2: System monitor
│       ├── network.rs        # F3: Network configuration
│       └── qrcode.rs         # F4: QR code registration
├── specs/                    # Specifications
├── Dockerfile                # Container build for testing
├── Dockerfile.qemu           # QEMU test image (optional)
└── scripts/
    └── run-qemu.sh           # Helper to launch QEMU test VM
```

## Code Style

```bash
# Format
cargo fmt

# Lint
cargo clippy -- -D warnings

# Test
cargo test
```
