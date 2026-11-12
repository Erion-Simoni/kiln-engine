# Clean Linux userspace acceptance

Task 8 tests the prepared editor and its exported native game in a clean Ubuntu
24.04 container. The container contains runtime system libraries and GUI test
drivers. It has no Rust/Cargo, compiler, Git, repository or development sysroot.
Only the prepared archive (read-only) and an evidence directory are mounted.

This isolates library/application dependencies from the development environment.
Containers share the host kernel and this run uses software graphics/ALSA null;
it is not separate physical-machine, audible-device or Intel HD 520 evidence.
Windows, macOS and physical Linux devices need their own acceptance runs.
Use [PHYSICAL_DEVICE_TEST.md](PHYSICAL_DEVICE_TEST.md) to record those checks,
including actual graphics/audio/controller devices and reference measurements.

## Run

Prepare a portable installation following [PORTABLE_EDITOR.md](PORTABLE_EDITOR.md),
then run with access to a local Docker daemon:

```sh
python3 tools/test_clean_linux.py target/dist/kiln-portable-development.tar.gz
```

The runner uses the local Unix Docker socket explicitly, keeping the existing
Docker credential/proxy configuration. It builds a small sanitized context
containing only the Dockerfile and five native test scripts. The pinned official
Ubuntu base and exact runtime/test package versions are recorded in evidence.
The image and shipped archive share `packaging/linux-runtime-packages.txt`; it
includes runtime-loaded X11 support (`libxkbcommon-x11-0`) in addition to ELF
dependencies. A clean run found and corrected that missing runtime requirement.
No engine source is copied into the test image. Runtime package installation may
use the network during image preparation; the acceptance container has networking
disabled and runs unprivileged with a read-only root and temporary working/home
filesystems. The test does not share the host display or GPU devices.

The existing native tools must target the chosen userspace. This checkpoint's
binaries reference GLIBC 2.39, matching Ubuntu 24.04. Building on a newer distro
can require newer symbols; CI pins the producing job to Ubuntu 24.04. This does
not prove compatibility with older distributions. A different supported baseline
requires rebuilding and executing the same acceptance on that baseline.

## What it verifies

- Runtime-only environment: absent Rust/Cargo/compiler/Git tools, isolated
  repository/sysroot and successful native `ldd` library resolution.
- Extract/move the full installation into a path containing spaces; validate the
  manifest/hashes/permissions/browser descriptor and device-free tool identities.
- Launch the installed editor from an unrelated cwd using a working copy of its
  shipped example. Author Player X=88 and use actual Save & Export; child tools
  run with empty PATH and use sibling installed runtimes. Preserve prior output
  on failed rebuild and cancel from the editor UI.
- Relocate the exported native game and make its project source copy unavailable.
  Launch without an archive argument and exercise title/settings/gameplay/pause,
  inventory, saving, fresh-process restart/Continue and corrupt-save preservation.
- Export/play the shipped material playground with the installed CLI/player, check
  numeric/texture overrides, alpha order, flip/rotation and save/scene restoration,
  then open it in the installed editor and verify actual material preview pixels.
- Verify the installed files remain unchanged after use. Browser files are
  validated as part of installation verification; browser gameplay uses the
  separate [browser export acceptance](BROWSER_EXPORT_2D.md).

Evidence is written to `target/task8-clean-linux-evidence`: build/container logs,
image/base identity, archive checksum, exact installed packages, native dependency
resolution, tool identities, actual editor/game results and screenshots. The
runner fails if either GUI/game subprocess or final verification fails. Only this
test's named container is removed on completion or an interrupted run.

## Release and platform checks still needed

Test appropriate Windows/macOS machines, physical GPU/audio/controller behavior,
supported Linux driver stacks and startup/frame-time/memory on reference hardware.
Confirm exact build provenance and package license declarations before publishing
release installers; use [PACKAGING.md](PACKAGING.md) for signing requirements.
Package size is informational and never blocks completion (owner request,
ADR-0062). File/input limits remain technical resource protections.
