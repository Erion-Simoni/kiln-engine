# Portable development editor — Task 8

The Linux development bundle contains the native editor, CLI exporter, player,
prepared browser JS/WASM, three project templates, saved-UI, physics and material examples, and
notices. Keep the extracted directory together. Authoring and both export targets
use those prebuilt files without Cargo, Rust, Python, Node or wasm-bindgen.

Development and captured release-profile candidates pass relocation and clean
Ubuntu software acceptance. A hosted Linux candidate also passed publication and
GitHub build-origin attestation (run 43107026471 at 0b9a67b). Installers/OS signing,
independent-builder/dependency provenance,
physical machines/devices and Windows/macOS remain open. Task 8 is not complete.

## Prepare once

Requirements for preparation: Rust/Cargo, Python 3.11+, matching wasm-bindgen CLI,
Linux `readelf`, cached dependency sources/crate archives, and the compiler's
`share/doc/rust/COPYRIGHT-library.html` (install Rust documentation if omitted).
Runtime use does not require these preparation tools.

```sh
CARGO_PROFILE_DEV_DEBUG=0 cargo build -p kiln-editor -p kiln-cli -p kiln-player --bins
just web-runtime
just portable-editor
```

The recipe uses host/WASM filtered, locked Cargo metadata. It can fetch missing
metadata dependencies; the Python producer performs no build or download.
`just portable-editor` currently targets Linux x86_64 development builds. Its
Python interface accepts explicit `--binaries`, `--web-runtime`, repeated
`--metadata` and `--output` paths. The producer requires matching tool build profiles and records compiled debug
flags. Supplying release binaries creates a release candidate; it does not
certify provenance, licensing, performance or clean-machine acceptance. It refuses an
existing output, including symlinks; select a new output path for another bundle.
Generated archives stay under `target/dist`, outside Git.

All three binaries must report matching tool/version/host/build-profile identity.
Editor/player require audio, fonts and gamepad support. Browser files must pass
existing prepared-runtime descriptor/hash/embedded-version checks. Staging is
private, the completed installation is verified, and archive publication is
exclusive. Tar/gzip entries have sorted paths, normalized times/ownership and
preserved executable modes. Equal inputs/manifest observations produce equal
archives; differing source state/compiler observations change the manifest.

## Use the extracted installation

```sh
tar -xzf kiln-portable-development.tar.gz
cd kiln-0.1.0-linux-x86_64
./kiln-cli verify-editor-bundle --input .
./kiln-editor
```

The editor finds the exporter/player/browser runtime beside its executable even
when launched from an unrelated directory. Open
`examples/ui2d_editor/kiln.toml`, or create a project. Save working projects outside
the installation so installation verification can continue to detect changes.
Playgrounds are at `examples/browser_physics2d/kiln.toml` and
`examples/materials2d/kiln.toml`.
CLI templates also work from elsewhere:

```sh
/path/to/kiln-cli init --name MyGame --template empty_2d --output /games/MyGame
/path/to/kiln-cli export --project /games/MyGame/kiln.toml --output /games/native
/path/to/kiln-cli export --platform web --project /games/MyGame/kiln.toml --output /games/browser
```

The empty template receives a main scene and script. Other templates retain their
scenes. Init refuses a nonempty output to protect existing projects. Native games
launch their included executable; serve complete Browser outputs over HTTP(S).
Browser physics requires a matching physics-enabled prepared runtime. Default
builds provide it; opted-out/legacy bundles reject authored physics. Sprite
materials similarly require a capable runtime; current tools provide the shared
validated fragment pipeline. See [SPRITE_MATERIALS_2D.md](SPRITE_MATERIALS_2D.md). See [NATIVE_EXPORT_2D.md](NATIVE_EXPORT_2D.md) and
[BROWSER_EXPORT_2D.md](BROWSER_EXPORT_2D.md) for target contracts.

## Integrity and provenance

`bundle.json` inventories file sizes, SHA-256, executable flags and tool metadata.
Verification rejects incompatible/missing tool identities, changed/missing files,
unlisted files, duplicate/traversing paths, symlinks/special files and changed
browser runtime pairs. It hashes files in bounded chunks and never executes
installation tools. Limits: 8 MiB manifest, 20,000 files, 512 MiB per file, 2 GiB
listed contents. `SHA256SUMS` covers payload files before it and bundle.json are
written; bundle.json also covers SHA256SUMS. Neither is signed. Obtain downloads
through a trusted channel; checksums alone do not authenticate them.

The source revision/dirty flag and preparing compiler are observations of the
packaging checkout. They do not attest that arbitrary supplied binaries came
from that checkout/compiler. Exact build provenance and reproducible release
builds remain release gates. `--tool-info` works without opening devices and
records the actual compiled feature/debug flags, but is not a signature.

For a fresh source-to-bundle execution record, use `just portable-release`.
It captures source and Cargo.lock, builds offline/locked into a new directory,
checks native/browser artifact hashes against the build record and includes the
record in the installation. See [BUILD_PROVENANCE.md](BUILD_PROVENANCE.md).
This narrows the prebuilt-input gap without claiming signed attestation or
byte-reproducible/hermetic builds. Physical/platform acceptance remains separate.

The captured release payloads were rebuilt in a second source/target directory
on 2026-11-11 and all six native/browser files matched byte for byte. Execution
archives differ due to their records. See [repeated-build evidence](evidence/task8-repeated-build/README.md).
This is same-host/toolchain repetition; trusted independent builders remain open.

The release-candidate Actions workflow runs the same checks on a hosted Linux
runner, retaining the accepted candidate/evidence and creating a build-origin
attestation in a separate job. Run 43107026471 at 0b9a67b completed successfully,
including candidate upload, attestation creation and bundle upload. Public job
conclusions are recorded in [hosted evidence](evidence/task8-hosted-ci/README.md);
candidate signatures were not independently downloaded/verified here.
Root dependencies are pinned in tracked Cargo.lock.
See [BUILD_PROVENANCE.md](BUILD_PROVENANCE.md#hosted-release-candidate-workflow).
For a successful hosted candidate, follow the download/verification recipe in
[BUILD_PROVENANCE.md](BUILD_PROVENANCE.md#hosted-release-candidate-workflow). It
selects a completed run, downloads into a new directory, verifies all five
attested files against that run's source/workflow SHA, then extracts and checks
the installation. Branch HEAD can be newer than the candidate after documentation
commits; use the run's SHA rather than assuming the latest branch has an archive.

## Notices

`licenses/inventory.json` preserves package license expressions and repositories,
Cargo.lock checksums/source URLs and original notice paths. It covers a
conservative union of supplied host/WASM workspace graphs, including build/test
dependencies; it is not an exact linked-code report. Original license, copyright,
notice, README, manifest and nested font license files accompany pinned SPDX
reference texts. MPL dependencies include unmodified `.crate` sources verified
against Cargo.lock. The preparing compiler's original standard-library copyright
report is included. The example font retains its license.

The owner delegated the project license choice on 2026-11-12. Current project
code uses the existing workspace's `MIT OR Apache-2.0` dual license, with missing
engine/sample declarations now inheriting it. Full root LICENSE-MIT and
LICENSE-APACHE texts are copied into `licenses/project` and listed in the
inventory. Packaging rejects missing project license files or unlicensed path
package metadata; it does not silently assign a license while copying notices.
Third-party/font grants remain unchanged. No OS/driver libraries or preparation
tools are redistributed by this archive.

Historical inventories, including source 0b9a67b's candidate, contain the original
null project package declarations and lack the newly added project grant files.
Use a newly prepared candidate for the updated license payload. Existing evidence
is not retroactively presented as having shipped those files. Exact linked-code,
SDK/dependency provenance and platform release review remain separate.

When distributing an exported game, also copy the complete installed `licenses`
directory beside its `game`/`game.exe`, or beside the browser output's index.html.
Keep original notices for your game's own assets too. The current exporter writes
the runtime and project resources; automatic propagation of the installed runtime
notice inventory remains Task 8 work. The additional license copy is manual and
is not covered by the existing export receipt. The engine's dual license does
not choose a license for your own game code or assets.

## Tested requirements and remaining platform gates

Tested: Linux x86_64, X11 via Xvfb, Chromium SwiftShader/software graphics and
ALSA null output. Linux needs compatible glibc and display/GPU/audio/controller
system libraries. Bundles include `linux-runtime-packages.txt` for Ubuntu 24.04.
Install that list with `sudo xargs -a linux-runtime-packages.txt apt-get install -y`
after updating APT. It includes runtime-loaded `libxkbcommon-x11-0`; absence of
that X11 dependency can panic inside winit despite successful `ldd` output. `bundle.json.system_observations` records each native ELF's
direct DT_NEEDED libraries and highest referenced GLIBC symbol. This local build
references GLIBC 2.39, libasound.so.2 and (editor/player) libudev.so.1, in addition
to libgcc_s, libm, libc and the ELF loader. It does not
resolve dynamic graphics/audio plugins or prove complete library closure. Driver
compatibility, physical audio and Intel HD 520 performance are unverified.
Windows/macOS bundles and installation are unverified. Package-size goals were removed by the project owner (ADR-0062); archive sizes
are informational and do not block completion.

Run `python3 -m unittest discover -s tests/native -p test_portable_package.py`
for producer regressions. Full software acceptance:

```sh
python3 tests/native/portable_editor_smoke.py target/dist/kiln-portable-development.tar.gz
```

It extracts then moves the installation outside the repository, checks identities
and corruption rejection, initializes/exports all three templates with empty
PATH, launches the installed editor from an unrelated directory, edits/saves and
exports Native and Browser through the real UI, then plays only relocated
artifacts with original project copies unavailable. Menus/audio APIs/settings,
save/restart/Continue and browser consent/storage/error behavior reuse existing
acceptance tests. The installation must still verify after use. Evidence lives
under `target/task8-portable-evidence`; selected inspected evidence is retained in
[evidence/task8-portable/README.md](evidence/task8-portable/README.md).

## Isolated Linux runtime acceptance

Run `python3 tools/test_clean_linux.py ARCHIVE` to exercise the installation in a
pinned Ubuntu 24.04 runtime userspace without mounting the repository or development
sysroot. It runs offline and unprivileged with only archive/evidence mounts and
executes real native editor export and relocated game save/resume. See
[CLEAN_MACHINE_TEST.md](CLEAN_MACHINE_TEST.md) for requirements, evidence and the
separate physical-machine/platform checks. Package-size targets are removed
(ADR-0062); current archive sizes are informational.
