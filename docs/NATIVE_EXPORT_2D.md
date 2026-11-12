# Native game export — Task 8 checkpoint

The editor's **View → Build / Export → Save & Export** now runs a real exporter.
It saves authoring changes through the existing project transaction, launches
`kiln-cli` asynchronously, streams bounded diagnostics, checks its actual exit
status and verifies this job's output receipt/archive before showing success.
Closing the panel does not stop supervision. Cancel requests cooperative shutdown;
after three seconds the editor kills/reaps the exporter and cleans private staging.
Exiting the editor also reaps its export process.

The native target supports the current desktop host and matching CPU architecture.
It was exercised on Linux x86_64. Browser export is now available through the
Browser target; see [BROWSER_EXPORT_2D.md](BROWSER_EXPORT_2D.md). Cross-target
toolchains and installers/signing remain Task 8 work; Linux/Windows portable
candidate checkpoints are documented in [PORTABLE_EDITOR.md](PORTABLE_EDITOR.md). Unsupported
platforms/options fail rather than producing a placeholder success.

## Build tools once, then export without Rust

For repository development, prepare the tools once:

```bash
cargo build -p kiln-editor -p kiln-cli -p kiln-player --bins
```

The editor defaults to `kiln-cli` and `kiln-player` beside its executable. Runtime
tools can be changed in the panel. Distributions must ship matching prebuilt tools
and keep their verified installation intact. Export does not invoke Cargo.
A `--no-default-features` player lacks required audio/fonts/gamepad support and is
rejected by the runtime handshake. A release runtime can be selected instead of a
debug one; exporting never silently rebuilds or changes its profile/features.

Stop Play, open Build / Export, choose a **new or empty** output directory, then
Save & Export. Relative editor output paths are relative to the project directory.
The destination text field actually persists edits. Export failure retains any
previous nonempty export; choose a new/versioned directory to build again.

The equivalent CLI command, from any working directory, is:

```bash
/path/to/kiln-cli export \
  --project /path/to/game/kiln.toml \
  --player /path/to/kiln-player \
  --output /path/to/new-export
```

`--platform` defaults to the current OS (`linux`, `windows`, `macos`); only that
host is accepted. `--player` defaults to the exporter executable's sibling player.
Relative CLI arguments resolve from its working directory. `--job-id` is the
editor/receipt correlation token and normally need not be supplied manually.

## What ships

- `game` (`game.exe` on Windows): a copy of the matching prebuilt standalone player.
- `assets.kar`: the normalized project manifest and relative project resources.
- `export.json`: versioned job/project/runtime identity and expected file sizes.
- `licenses/`: complete original project/dependency/Rust notices from the selected
  runtime's verified portable installation, plus a runtime-bound hash manifest.

The CLI and editor verify notice bytes, exact tree/receipt coverage and matching
player hashes before reporting success. Damaged/incomplete installations fail
without publishing an export. A loose developer player has no installation
inventory: export warns explicitly and omits the verified notice claim. Include
matching runtime notices before distributing such output. Keep your game's own
asset notices too. See [the notice contract](PORTABLE_EDITOR.md#notices).

Launch the game executable directly. By default it opens `assets.kar` beside
itself, independent of the working directory. `--archive PATH` remains supported
for development. Players need neither Rust, the editor, the source tree nor the
exporter. Saves/settings use the normal user-data profile, not the export folder.

The export transaction creates a private sibling staging directory, copies the
saved project, validates shared scene/module/entity scripts and hierarchy/UI,
decodes required images/audio/fonts without a device, packs and verifies archive
contents, copies the player and writes the receipt. Only then is the complete
folder renamed into place. Errors/cancellation before publication remove staging;
process kill cleanup is owned by the editor. A job already published at the instant
of cancellation may leave a complete output directory; previous exports are never
overwritten. The exporter/runtime handshake has a two-second deadline.

TOML and RON source manifests normalize to one `kiln.toml` in the archive. Relative
asset/script/font/audio paths retain their directory structure. Symlinks and
escaping resource paths are rejected. Hidden files/directories and `target`,
`export`, `dist`, the selected destination and private staging are excluded,
including `.git`, `.kiln-save`, recovery/cache data. Required resources in excluded
locations fail validation. Export limits are 100,000 visible files and 2 GiB of
copied resources. Keep exported output in the default `export/` tree or outside
the project; custom build folders from earlier jobs should not be kept as source
assets. Only the selected output and standard build folders are excluded.

## Executed acceptance and remaining gates

`kiln-cli/tests/native_export.rs` executes real CLI transactions for nested/spaced
paths, canonical manifests, archive contents, private-data exclusion, missing/
escaping/symlink resources, invalid scripts/media/runtime/platform, cancellation,
staging cleanup and previous-output preservation. Editor process tests verify
stdout/stderr, actual nonzero status, zero exit without a valid artifact, cancel
fallback and process reaping on drop.

`tests/native/export_smoke.py` runs a real editor on Xvfb with **an empty tools PATH**:
change Player X without saving, edit the actual output field, Save & Export,
verify X=88 was saved, inspect the receipt, test failure without overwrite, cancel
an active CLI handshake through the UI and relocate the result outside the repo.
`tests/native/relocated_export.py` then runs that artifact from `/tmp`, with no
archive argument and Cargo/rustc absent from PATH, and exercises menus, fonts,
audio commands/settings, health/inventory saving and fresh-process Continue.
CI runs both scripts after building the three native binaries.

Selected real captures/results are in [evidence/task8-native](evidence/task8-native/README.md).
This is relocated/no-toolchain execution on the cloud runner, not a clean-machine
OS/dependency test. Xvfb/software graphics and ALSA null output do not certify an
Intel HD 520, real controller/speakers, Windows/macOS or a signed distribution.
Package-size goals are removed (ADR-0062); sizes are informational. Clean Ubuntu 24.04 container acceptance now passes (see [CLEAN_MACHINE_TEST.md](CLEAN_MACHINE_TEST.md)); physical-machine
native acceptance and remaining other-platform distribution
must still pass before Task 8 is complete. Task 9 has not started.
