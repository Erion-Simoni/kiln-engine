# Task 8 portable development editor evidence — 2026-11-10

Linux x86_64 software acceptance using Rust 1.99.1, Xvfb, SwiftShader, ALSA null
and Chromium. These captures/results establish same-machine relocation; they do
not establish physical Intel HD 520/GPU/audio/controller behavior, another OS,
clean-machine library closure, release budgets or signed distribution.

## Reproduce

Build default-feature editor/CLI/player with CARGO_PROFILE_DEV_DEBUG=0, prepare
the browser bundle (`just web-runtime`), then `just portable-editor`. Run:

```sh
python3 -m unittest discover -s tests/native -p test_portable_package.py
python3 tests/native/portable_editor_smoke.py target/dist/kiln-portable-development.tar.gz
```

Preparation needs filtered host/WASM Cargo metadata and original compiler notices;
see [PORTABLE_EDITOR.md](../../PORTABLE_EDITOR.md). The tested archive was
`target/dist/kiln-portable-development-verified.tar.gz`, **56,605,832 bytes**,
SHA-256 `769de03139341bb8eadfeb61d74e41208988a30a57c131ba7b4ff8dd1fdafa06`.
It was built from the dirty working checkout based on 566aad7; this records
preparation observations, not an attestation of binary provenance. Generated
archives/binaries are not committed. bundle-summary.json records tool flags,
file counts, dependency inventory counts and actual ELF/GLIBC observations.

## Observed acceptance

- Extracted the complete archive and moved its directory to a path containing
  spaces under /tmp; ran installation verification from an unrelated cwd with
  an empty PATH. All three tools' device-free --tool-info reports matched.
- Rejected changed executable bytes, removed execute permission, mixed tool
  version metadata, missing example manifest, a symlink within the installation
  and a symlinked installation root. Restored inputs verify successfully.
- Initialized empty_2d, platformer and topdown through the installed CLI and
  exported each to Native and Browser using sibling runtime defaults with empty
  PATH. Initialization refuses nonempty destinations and preserves the project.
- Launched only the installed editor from an unrelated cwd, using a copy of the
  shipped example. Real Native and Browser UI exports saved unsaved Player X=88,
  verified current-job artifacts and retained existing outputs on failure.
  Native UI cancellation also passed. Test helpers require Xvfb/xdotool, but the
  editor and export subprocess have no development tools on PATH.
- Moved both games outside the repository and made their source project copies
  unavailable. Native no-archive-argument play passed title/settings/pause,
  health 66, inventory, independent settings and restart/Continue with script
  Start count 1; corrupt-slot error preservation also passed.
- Served only the relocated Browser artifact at a nested HTTP URL. Passed
  loading/error/retry and explicit audio-consent behavior, held-Enter protection,
  controller API/touch navigation, saved TTF UI, real Web Audio API looping and
  overlap, independent settings/save/reload/Continue, quota/corrupt preservation
  and no unhandled page errors. Physical output/devices were not tested.
- The original installed files still verify after these operations. The GUI
  uses disposable working project copies, preserving shipped example files.
- `just check`, **657 Rust tests / zero skipped**, four producer tests, minimal
  editor/player checks and strict WASM Clippy pass. Remote CI is configured but
  its execution was not observed.

The selected PNGs were inspected. Full logs/screenshots and disposable profiles
are under target/task8-portable-evidence. acceptance.json and target result files
retain the temporary artifact paths used by this run; those paths are evidence,
not reusable installation instructions. Task 8 remains open for release and
clean-machine/platform/runtime parity gates.
