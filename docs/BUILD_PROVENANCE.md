# Captured portable builds — Task 8

Use `just portable-release` to build a Linux x86_64 candidate from a captured
source snapshot into a fresh directory. It builds rather than accepting arbitrary
prebuilt binaries. This produces an unsigned local execution record; it does not
authenticate a publisher or establish byte-reproducible or hermetic builds.

## Prepare and build

Install Rust with the wasm32-unknown-unknown target, matching wasm-bindgen CLI,
Python 3.11+, Linux development headers and Rust standard-library copyright
documentation. Cache the workspace's locked host/WASM dependencies first. The
builder deliberately uses `--offline --locked` and fails if inputs are missing.
It performs no dependency installation or download. See PORTABLE_EDITOR.md for
the existing packaging/notices requirements.

```sh
just portable-release target/my-release-candidate
# Equivalent:
python3 tools/build_editor.py --profile release --output target/my-release-candidate --jobs 2
```

The output directory must not exist. Failed builds retain logs there; choose a
new directory for a retry. Native and WASM Cargo outputs go into its previously
empty target directory. The default release profile in the captured Cargo.toml
is used without silently inheriting profile/target/compiler-wrapper/rustflags
environment overrides. `--profile dev` is available for build-driver diagnostics.
Normal Cargo home, proxy/CA and platform SDK discovery remain available.

## Inspect the result

- `source/` and `source.tar.gz` contain tracked and nonignored untracked files
  plus the otherwise ignored Cargo.lock. Missing tracked files, symlinks and
  special/oversized files fail capture. Executable modes are preserved. Git HEAD
  and dirty status describe the original checkout; the source inventory and
  archive hashes identify the actual inputs, including unfinished local edits.
- `build-record.json` records source paths/hashes/modes, lockfile hash, compiler/
  Cargo/wasm-bindgen versions, builder hash, selected SDK/build settings, Cargo
  configuration hashes, command/log hashes and all three native plus three
  prepared browser payload hashes. It contains no full environment dump or Cargo
  configuration contents. Logs and source may contain project content; review
  them before sharing. Configuration and system/toolchain dependencies are not
  fully captured or made hermetic by this record.
- `kiln-portable.tar.gz` includes the checked record. The packager requires exact
  source inventory, lockfile, artifact hashes, compiler version and compiled build
  profile agreement. Its normal tool/runtime/inventory verification still runs.
  A modified or incomplete payload cannot be assigned this record unnoticed.
- `result.json` is written only after build and package verification succeed. It
  records final archive/source/record hashes. Ordinary installation verification
  hashes the included record just like any other payload file. It does not trust
  its unsigned claims as authentication.

The original prebuilt `just portable-editor` workflow remains available. Its
manifest labels provenance `preparation-observations-only`; a checked local build
record labels it `unsigned-local-build`. Neither label is a signature. Both
bundles still require the functional clean-runtime checks.

## Validate the candidate

```sh
python3 -m unittest discover -s tests/native -p 'test_*.py'
python3 tools/test_clean_linux.py target/my-release-candidate/kiln-portable.tar.gz
```

Also exercise material/physics/browser behavior using the resulting runtime.
The clean Linux runner includes native/player/editor material pixel acceptance
using the installed tools and shipped material playground.
Clean Ubuntu/software graphics is a functional userspace check, not physical
Intel HD 520 performance, audible hardware/controller, other OS/browser or mobile
acceptance. Startup/frame-time/memory must be measured on reference hardware.

For independent rebuilding, extract source.tar.gz, preserve Cargo.lock, use the
recorded compiler/bindgen and commands with new output paths, and compare payload
hashes. A successful first build does not prove two builds match. Trusted CI
attestation, dependency/SDK/toolchain provenance, repeated binary comparison,
licensing review and installer signing remain release work. Archive size is
informational; no size ceiling applies.
