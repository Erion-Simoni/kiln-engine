# Kiln justfile
# Run `just --list` to see available recipes

# fmt + clippy + build
check:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo check --workspace
    # The audio sample is its own workspace, because it needs a system audio
    # backend. It is checked on its own so the main workspace builds everywhere.
    cargo check --manifest-path samples/audio/Cargo.toml

# run tests with cargo-nextest
test:
    cargo nextest run --workspace

# run editor or demo window
run:
    cargo run --bin demo

# wasm demo in browser
web:
    cargo build --target wasm32-unknown-unknown --release -p kiln-web

# wasm/native size budgets
size:
    @echo "=== Web WASM size check ==="
    cargo build --release --target wasm32-unknown-unknown -p kiln-web
    @echo "Uncompressed WASM:"
    @ls -lh target/wasm32-unknown-unknown/release/kiln_web.wasm
    @echo ""
    @echo "Note: wasm-opt and brotli tools required for full size check"
    @echo "Install with: cargo install wasm-opt; cargo install brotli"
    @echo ""
    @echo "=== Editor installer size check ==="
    @echo "Note: cargo-dist requires GitHub Actions CI for full installer builds"
    @echo "Local build can estimate binary size:"
    cargo build --release --bin kiln-editor
    @echo "Editor binary:"
    @ls -lh target/release/kiln-editor.exe 2>/dev/null || ls -lh target/release/kiln-editor 2>/dev/null || echo "Binary not found"
    @echo ""
    @echo "Budgets: WASM <1.5 MB brotli, Editor <40 MB installer"

# import / pack assets
assets:
    cargo run --bin kiln-cli -- import --source assets --cache .kiln-cache
    cargo run --bin kiln-cli -- pack --source assets --output assets.kar

# the audio sample needs a system audio backend, so it is its own workspace
audio:
    cargo run --manifest-path samples/audio/Cargo.toml

# installers via cargo-dist
package:
    cargo-dist build

# default recipe
default:
    just --list
