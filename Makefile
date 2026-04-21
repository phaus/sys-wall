TARGET := x86_64-unknown-linux-musl
BINARY := sys-wall
RELEASE_TARGET := target/$(TARGET)/release/$(BINARY)

.PHONY: all build clean run release help

all: build

build:
	cargo build

release:
	rustup target add $(TARGET) 2>/dev/null
	cargo build --release --target $(TARGET)

install: release
	install -m 0755 $(RELEASE_TARGET) /sbin/$(BINARY)

uninstall:
	rm -f /sbin/$(BINARY)

clean:
	cargo clean

run:
	cargo run

help:
	@echo "Targets:"
	@echo "  build     - Build debug binary"
	@echo "  release   - Build musl static binary"
	@echo "  install   - Install to /sbin/"
	@echo "  clean     - Remove build artifacts"
	@echo "  run       - Run in development mode"
	@echo "  help      - Show this help"
