# Kiln Session Handoff

**Active milestone:** A01 — Restore trustworthy CI and baseline (PARTIAL).
**Branch:** fix/a01-ci-baseline; previous pushed repair 01a272c.
**Owner preferences:** batch several small tasks per session. No release-size ceiling;
resource/performance limits remain. MIT OR Apache-2.0 settled. Full 3D gated on G2D.

## Done this batch

- Corrected ignored cargo-dist.toml configuration by moving to supported Cargo
  metadata. Only editor/CLI/player opt into experimental desktop binary archives;
  samples/demos excluded. Corrected tool command and target/distrib upload path;
  missing artifacts fail. Removed nonexistent/unreachable upload command.
- ADR-0078 preserves complete portable bundle workflows and future installer
  acceptance. Archive planning does not establish installer/target acceptance.
- Corrected distribution repository links and package recipe/help/docs.
- Pinned CI Rust/nextest and root dependency graphs; updated pinned Node 24-capable
  checkout/cache/Node/artifact actions. General CI and reference-game checks now
  run on fix/** pushes, so PR API access is unnecessary to start these checks.
- Failed reference-game replay cleanup now terminates/kills/reaps children with
  bounded waits, preserving original failures. Two real child-process tests added.
- Rebuilt a matching portable bundle and replayed all three reference games through
  real editor widgets. Each authored manifest equals its checked-in project; all
  six native/browser exports verify and reject overwriting previous good output.
- Input Rebinding replay now captures the settled panel before clicking controls,
  avoiding a reproduced immediate-click/layout race. Prior failed runs retained.

## Validation

- just check passed; 700 Rust tests passed, zero skipped; 31 Python tests passed.
- actionlint 1.7.7 passed (shellcheck disabled); mdBook 0.4.52 passed.
- Real cargo-dist 0.33.0 plans pass for Linux x64, Windows x64 and Intel macOS,
  with exactly the intended three packages; complete archive builds remain unrun.
- Matching portable bundle verified: 1,546 files, SHA-256
  9442954eb9e39f5645fd9df490821fe7f32022c27d8bf7f00971809aba53919e.
- All three games pass standalone and Chromium gameplay: victories, scene/progress
  restoration and restart/reload continuation. Durable record:
  docs/evidence/A01_REFERENCE_GAMES.json.
- Raw evidence under target/a01-*; durable summary in docs/evidence/A01_CI_BASELINE.md.
- These are software graphics/Xvfb/Chromium/null-audio checks, not physical hardware.

## Known issues / next

1. A01 remains open until fresh hosted quality, archive builds, candidate/artifact
   verification and reference-game checks pass. Push this batch; fix/** triggers
   general CI and reference-game acceptance. No release gate is closed.
2. Git push works; GitHub CLI API returned Forbidden. Authenticated log/PR access
   is unavailable. Public pages previously identified ae492f1 general CI and Linux
   release failure, f7c8f9d reference-game failure, and Windows candidate success.
   Those old results do not establish acceptance of this batch.
3. Full portable Linux/Windows acceptance remains in candidate workflows. macOS
   archive planning does not establish full macOS editor/export support. Real
   DMG/AppImage/MSI installers, signing and install/uninstall acceptance remain open.
4. Continue A01 diagnosis from fresh hosted results, then A02 durable authoring and
   unsaved source recovery. The unified ENGINE_COMPLETION_PLAN.md owns later scope.

Engine Rust behavior is unchanged in this batch. Retain failed/unrun evidence and
physical-device limitations; do not mark broad engine parity or full 2D complete.
