# Source cargo env if not already in PATH
ifndef CARGO_HOME
  ifeq ($(shell which cargo 2>/dev/null),)
    include ~/.cargo/env 2>/dev/null || true
  endif
endif

TARGET := x86_64-unknown-linux-gnu
BINARY := sys-wall
RELEASE_TARGET := target/$(TARGET)/release/$(BINARY)

.PHONY: all build clean run release deploy deploy-clean help

all: build

build:
	cargo build

release:
	rustup target add $(TARGET) 2>/dev/null
	CC=x86_64-linux-gnu-gcc cargo build --release --target $(TARGET)

deploy: release
	scp $(RELEASE_TARGET) legacy-dev-root:/tmp/sys-wall-new
	ssh legacy-dev-root 'install -m 755 /tmp/sys-wall-new /sbin/sys-wall && systemctl restart sys-wall'

deploy-clean: release
	rm -rf /tmp/sys-wall-build
	rsync -avz --exclude='target' --exclude='.git' . legacy-dev-machine:/tmp/sys-wall-build/
	ssh legacy-dev-root "source \$$HOME/.cargo/env && cd /tmp/sys-wall-build && cargo build --release && install -m 755 /tmp/sys-wall-build/target/release/sys-wall /sbin/sys-wall && rm -rf /root/.config/sys-wall && systemctl restart sys-wall"

clean:
	cargo clean

run:
	cargo run

help:
	@echo "Targets:"
	@echo "  build         - Build debug binary"
	@echo "  release       - Build x86_64 binary (for legacy-dev-machine)"
	@echo "  deploy        - Copy and restart service on legacy machine"
	@echo "  deploy-clean  - Full build+deploy (sync source, build, install)"
	@echo "  clean         - Remove build artifacts"
	@echo "  run           - Run in development mode"
	@echo "  help          - Show this help"
