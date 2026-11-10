# Kiln justfile
# Run `just --list` to see available recipes

# fmt + clippy + build
check:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo check --workspace
    # Native editor/player enable audio/fonts/gamepad (Linux: libasound2-dev libudev-dev).
    # The standalone audible device sample remains a separate workspace.
    cargo check --manifest-path samples/audio/Cargo.toml

# run tests with cargo-nextest
test:
    cargo nextest run --workspace

# run the native editor
run:
    cargo run --bin kiln-editor

# run the standalone demo
demo:
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

# Real browser check; requires matching wasm-bindgen CLI and Playwright Chromium
web-smoke:
    cargo build --target wasm32-unknown-unknown -p kiln-web
    wasm-bindgen target/wasm32-unknown-unknown/debug/kiln_web.wasm --target web --out-dir target/web-smoke
    npm ci --prefix tests/web --ignore-scripts
    node tests/web/smoke.cjs target/web-smoke
    node tests/web/task7.cjs target/web-smoke

# Prepare the reusable browser export runtime beside development editor binaries.
web-runtime:
    cargo build --target wasm32-unknown-unknown -p kiln-web
    wasm-bindgen target/wasm32-unknown-unknown/debug/kiln_web.wasm --target web --out-dir target/debug/web-runtime
    cargo run -p kiln-cli -- web-runtime --input target/debug/web-runtime

# Linux development bundle from prebuilt default-feature tools and prepared browser runtime.
# Refuses to replace the output; see docs/PORTABLE_EDITOR.md for release gates.
portable-editor:
    cargo metadata --locked --format-version 1 --filter-platform x86_64-unknown-linux-gnu > target/portable-metadata.json
    cargo metadata --locked --format-version 1 --filter-platform wasm32-unknown-unknown > target/portable-web-metadata.json
    python3 tools/package_editor.py --binaries target/debug --web-runtime target/debug/web-runtime --metadata target/portable-metadata.json --metadata target/portable-web-metadata.json --output target/dist/kiln-portable-development.tar.gz
