# Task 8 clean Ubuntu userspace evidence — 2026-11-10

Passed native editor/export/game acceptance in an official Ubuntu 24.04 runtime
image with no Rust/Cargo/compiler/Git, repository or development sysroot. Runtime
container runs as UID 1000 with networking disabled, read-only root, no shared GPU
devices/display and only the archive/evidence mounted. Xvfb, Mesa software graphics
and ALSA null output are test substitutes; physical devices are not certified.

## Reproduce

```sh
python3 tools/test_clean_linux.py target/dist/kiln-portable-development.tar.gz
```

See [CLEAN_MACHINE_TEST.md](../../CLEAN_MACHINE_TEST.md) and
[PORTABLE_EDITOR.md](../../PORTABLE_EDITOR.md). Docker uses its managed local Unix
socket and existing proxy/credential configuration. Build context contains only
four native acceptance scripts, the Dockerfile and shared runtime requirements;
engine source/project and development tooling are not copied. Runtime packages
and GUI test drivers are installed only during image preparation.

This run used `target/dist/kiln-task8-ubuntu-runtime.tar.gz`, SHA-256
`df258efc4c0dd6887101c22f337964f1283e7e7965c096f67b205030919f9cbb`.
container-info.json records pinned Ubuntu digest, exact resulting image ID,
archive identity and runtime isolation options. system-packages.tsv records all
installed packages, including libc6 2.39-0ubuntu8.9. These are build observations,
not a reproducible-build/source-provenance attestation. Generated archives/images
are not committed. Package size is informational and is not a release goal
(owner request, ADR-0062).

## Found and corrected

The first clean launch resolved native DT_NEEDED entries but panicked while
loading X11 keyboard support:

```text
Library libxkbcommon-x11.so could not be loaded.
```

The dependency loads dynamically and is absent from ldd's direct dependency
report. `packaging/linux-runtime-packages.txt` now includes
`libxkbcommon-x11-0`; the same list is shipped in the installation and used to
prepare the test image. Acceptance asserts both copies match. Startup/real GUI
coverage complements static dependency inspection. The package list is the
tested baseline, not a claim of a universal/minimal dependency closure for every
Linux desktop, driver, display server or plugin path. Test drivers also add
system packages; all versions are retained for review.

## Observed

- No developer tools on PATH; no repository/development sysroot mounts.
- Extract/move installation into a path with spaces, use unrelated cwd, verify
  hashes/modes/required files/browser descriptor and all three tool identities.
  Native ldd reports have no missing libraries (acceptance.json).
- Installed editor opens and saves an actual Player X=88 edit during Native
  Save & Export. Export uses installed sibling CLI/player with an empty tools
  PATH. Current artifacts verify, failed rebuild preserves prior output and UI
  Cancel removes incomplete staging/output (editor-export.json).
- Relocated game runs without an archive argument with source project copy
  unavailable. Title/settings/gameplay/pause, inventory and independent settings
  work. Saving and fresh-process Continue retain health 66 and script Start count
  1, resume movement, and preserve a corrupt slot after load error (gameplay.json).
- Installed files verify again after use. acceptance.json records completion;
  selected actual export/resumed-game PNGs were inspected.
- `just check`, 657 Rust tests (zero failed/skipped), four producer tests and Python
  syntax checks pass. CI is configured; remote execution has not been observed.

The container shares the host kernel. Physical Intel HD 520/performance/audio/
controllers, other distributions and Windows/macOS remain unverified. This image
runs native gameplay only; the separately tested relocated Browser contract and
its physics/material restrictions remain unchanged. Full logs and disposable
profiles are in target/task8-clean-linux-evidence. Temporary paths in results
were inside the now-removed test container and are evidence, not installation
instructions.
