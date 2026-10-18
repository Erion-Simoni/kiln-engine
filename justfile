# Kiln justfile
# Run `just --list` to see available recipes

# fmt + clippy + build
check:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo check --workspace

# run tests with cargo-nextest
test:
    cargo nextest run --workspace

# run editor or demo window
run:
    cargo run --bin kiln-editor

# wasm demo in browser
web:
    trunk serve --open

# wasm/native size budgets
size:
    @echo "Web WASM size check (after M0):"
    @echo "cargo build --release --target wasm32-unknown-unknown"
    @echo "wasm-bindgen --out-dir web --target web"
    @echo "wasm-opt -Oz -o web/pkg/kiln_bg.wasm web/pkg/kiln_bg.wasm"
    @echo "brotli web/pkg/kiln_bg.wasm -o web/pkg/kiln_bg.wasm.br"
    @echo "ls -l web/pkg/kiln_bg.wasm.br"
    @echo ""
    @echo "Editor installer size (after M12):"
    @echo "cargo-dist build"
    @echo "ls -lh dist/"

# import / pack assets
assets:
    cargo run --bin kiln-cli -- pack

# installers via cargo-dist
package:
    cargo-dist build

# default recipe
default:
    just --list
