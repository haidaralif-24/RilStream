# Install `just` with: brew install just
default:
    @just --list

# --- host (Rust) ---
test:
    cargo test --workspace

fmt:
    cargo fmt --all

lint:
    cargo clippy --workspace --all-targets -- -D warnings

fake:
    cargo run -p rst-host --example fake_robot

dash:
    cargo run -p rst-host --example dashboard

# --- firmware (PlatformIO) ---
build-mega:
    pio run -d firmware/mega

up-mega:
    pio run -d firmware/mega -t upload

up-esp:
    pio run -d firmware/esp -t upload

mon:
    pio device monitor -d firmware/mega -b 115200

# compile_commands.json for clangd (Zed)
compiledb:
    pio run -d firmware/mega -t compiledb
    pio run -d firmware/esp -t compiledb
