# A01 CI baseline repair — 2026-11-13

Status: partial. Branch: `fix/a01-ci-baseline`, starting from `93b0ee0`.
Engine source is unchanged from master `ae492f1`; workflow, documentation and
smoke-script repairs are in the commit containing this record.

## Diagnosis and changes

Public run 43405013257 identifies failed steps: documentation build, cargo-dist,
saved UI authoring, and Browser export in the native editor. Public job pages
were retrieved; detailed step-log endpoints returned HTTP 404 without authenticated
log access. This is not evidence of the detailed hosted error text.

Local reproduction established these concrete faults:

- mdBook could not parse the space-containing Fixes Log link, then rejected
  repeated chapter files. The introduction and AI integration chapter referenced
  nonexistent files. Added a real introduction, repaired the link, retained one
  chapter per file, and removed the nonexistent AI chapter. The overview still
  documents engine structure. Pinned mdBook to the locally tested 0.4.52 and
  ignored its generated output.
- The saved-UI smoke opened Sprite Editor instead of UI Designer and subsequently
  missed the Add Label button. Updated menu, widget, drag and history coordinates
  against observed screenshots. Existing property/gameplay assertions remain.
- Native/browser export smokes used stale Build/Export menu and field positions.
  Updated actual widget clicks, including native cancellation. Existing save,
  artifact inventory, relocation and failure-preservation assertions remain.
- Reference games now run on master/main pushes and pull requests targeting those
  branches, as well as manual dispatch and the original reference-game branch.
  Removed path filters so sample/template or other build-input changes cannot
  silently skip game acceptance.

## Local validation

Linux cloud software graphics, isolated Xvfb displays, SwiftShader where available,
null audio; this establishes no physical GPU/audio/controller acceptance.

- `just check`: passed (fmt, strict all-target workspace clippy, workspace and audio sample checks).
- `cargo nextest run --workspace --locked`: 700 passed, zero skipped.
- `python3 -m unittest discover -s tests/native -p 'test_*.py'`: 29 passed.
- Native editor/player `--no-default-features` checks: passed.
- WASM strict clippy with default features and `--no-default-features`: passed.
- `mdbook build docs` with 0.4.52: passed. Generated book stays untracked.
- `actionlint -shellcheck= .github/workflows/*.yml`: passed with actionlint 1.7.7;
  shellcheck was disabled because it is not installed.
- `task7_editor_smoke.py`: passed. Added widget dragged to [104, 80], undo/redo,
  save/reopen, Play save/resume, health 66 and one script start verified.
- `browser_export_smoke.py`: passed. Inspector value 88 saved by export, previous
  output preserved on deliberate failure, exported inventory checked and relocated.
- `export_smoke.py`: passed. Same save/failure preservation, real cancellation,
  staging cleanup and relocation verified.

Local command logs/screenshots/results are under `target/a01-*`; generated files
are deliberately excluded from the commit.

## Still required before A01 acceptance

Retrieve authenticated cargo-dist and candidate/reference-game failure logs;
repair and reproduce remaining causes. Existing installer workflow has no checked-in
cargo-dist metadata, while the supported portable packaging path is separate;
confirm the actual failure and intended installer contract before restructuring it.
Run the full three-game author/export/native/browser acceptance on this revision.
Require fresh hosted checks and artifact verification to pass before accepting
A01. Existing remote failures remain failures; local repairs do not close them.
