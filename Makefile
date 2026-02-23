.PHONY: test lint test-frontend test-backend lint-frontend lint-backend

# On Linux, xcap 0.8.2 requires libpipewire-0.3 and libgbm for Wayland support.
# These variables point to the dev-libs directory committed in src-tauri/dev-libs/
# which contains symlinks to system runtime libraries and pkg-config stubs.
# If the system already has libpipewire-0.3-dev installed, these are ignored.
CARGO_DEV_LIBS := $(CURDIR)/src-tauri/dev-libs
export PKG_CONFIG_PATH := $(CARGO_DEV_LIBS)/pkgconfig
export LIBCLANG_PATH := /usr/lib/x86_64-linux-gnu
export RUSTFLAGS := -L $(CARGO_DEV_LIBS)/lib

test: test-frontend test-backend

test-frontend:
	bun run test:coverage

test-backend:
	cargo test --manifest-path src-tauri/Cargo.toml

lint: lint-frontend lint-backend

lint-frontend:
	bun run lint

lint-backend:
	cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
	cargo check --manifest-path src-tauri/Cargo.toml --all-targets
