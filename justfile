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

# Optional artifact size observations; no package-size goal or release threshold.
size:
    cargo build --release --target wasm32-unknown-unknown -p kiln-web
    cargo build --release --bin kiln-editor --bin kiln-player
    python3 tools/report_sizes.py

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
