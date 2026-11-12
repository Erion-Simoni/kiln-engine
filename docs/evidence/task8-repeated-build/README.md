# Task 8 repeated release payloads — 2026-11-11

**Passed:** all six shipped native/prepared-browser payloads match byte for byte
between two fresh builds in different source/target directories on this host.
This is same-host/toolchain repetition, not trusted remote attestation, a hermetic
dependency audit or general independent-machine reproducibility certification.

## Reproduce

```sh
just portable-replay target/task8-release-build target/task8-replayed-release
just compare-builds target/task8-release-build target/task8-replayed-release target/task8-repeatability/comparison.json
```

Both output paths must be new. The comparison rehashes actual on-disk inputs and
outputs rather than trusting recorded artifact hashes. Replaying a record never
executes its command strings; the current driver uses fixed offline/locked builds.
**19 Python tests** cover capture, replay, corruption/missing/link
inputs, unequal native/WASM results, incompatible tools, failing CLI exit/report
and preservation of existing evidence. `just check` passed; `just test` ran
**665 Rust tests, all passed, none skipped**.

## Identity and result

- Frozen source archive SHA-256 in both builds:
  `966223e8534b274670c62bb0d1e139741bd5781ee48fdf6a08f0449aec9914e0`.
  Its 540-file inventory and Cargo.lock match exactly. Original snapshot was dirty
  on a0e199b; the exact captured inputs, not the final Git HEAD, identify the engine
  being repeated. Original record/evidence: ../task8-release-build/.
- First distribution archive SHA-256:
  `d5ca032d975c5761a267c24c60d03b97efad1d019e46915b89baab5c4ce15b4a`.
- Second distribution archive SHA-256:
  `af30962be428316c0e87bac43ff292a99f319289be0c5ab8a3e0db64200012b5`.
  Second record SHA-256:
  `d1abd2ae3f0ccb7530c468e44073de9f1882c05b1f2d00a190c6ae8fb0fb4d22`.
- comparison.json has passed=true, inputs_match=true, no input differences and
  no different_artifacts. Matching files: kiln-editor, kiln-cli, kiln-player,
  web-runtime/kiln_web.js, kiln_web_bg.wasm and runtime.json. Compiled release
  profile, host/tool versions, SDK settings/configuration and normalized build
  command inputs match. No Rust source/dependency changed this checkpoint.
- Both builds used fresh target directories; compiler outputs were not copied.
  Rust/bindgen/SDK/dependency source cache were shared. Native editor/player/CLI,
  WASM/bindgen, descriptor preparation, metadata/notices and package verification
  executed again. New installation verifies 1543 listed files plus bundle.json.
- Archive equality is **false**, reported independently of payload equality.
  Per-execution unsigned records, log hashes/path observations and rebuild_of
  differ. No whole-archive equality claim is made. The current replaying driver
  is hashed/copied separately from the older engine snapshot; its file hash
  matches tools/build_editor.py for this checkpoint.

The prior clean Ubuntu native/editor/gameplay/material and actual Browser export/
loading/input/audio/save acceptance exercised these exact, now byte-identical
payloads. This checkpoint reuses that functional evidence and executes the rebuilt
archive's CLI installation verifier. Binaries,
source archive and logs remain under target and are not committed.

Remaining Task 8 gates: physical Intel HD 520/other GPU/audio/controller,
Windows/macOS/other browsers/mobile and reference startup/frame-time/memory,
internal license declarations and trusted CI/toolchain/SDK/dependency provenance/
signing. Remote CI was not observed. Package sizes are informational; no size
ceiling applies. Task 9/full 3D remain unstarted.
