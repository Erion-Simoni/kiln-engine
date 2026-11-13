# Kiln Session Handoff

**Active milestone:** A01 — Restore trustworthy CI and baseline (PARTIAL).
**Branch:** fix/a01-ci-baseline; previous pushed repair 4b7f488.
**Owner preferences:** batch small tasks. No package-size ceiling; retain resource
and performance bounds. MIT OR Apache-2.0 settled. Full 3D gated on G2D.

## Verified hosted progress

Public source-pinned snapshot for 4b7f488 (2026-11-13):

- General CI 43509387416 SUCCESS. All three OS check/build/archive matrices,
  docs, Rust/Python tests, native UI and browser/portable compatibility passed.
  Actual desktop archives now supersede earlier planning-only evidence.
- Windows portable candidate 43509387540 SUCCESS, including upload/attestation.
  Its documented hosted contract does not establish full Windows GUI/hardware.
- Reference games 43509387450 FAILED at editor authoring/export, step 9. Quality
  gates and matched tool preparation passed; gameplay steps were skipped.
  Detailed exception remains unreadable without authenticated step-log access.
- Linux release candidate 43509387420 IN PROGRESS at latest follow-up. Artifact
  service preflight passed; candidate acceptance/attestation not yet confirmed.
- Durable snapshot: docs/evidence/A01_HOSTED_STATUS.json.

## Done this batch

- Replay waits for the rendered floating-panel title before dragging it.
- Input authoring saves and verifies each action creation and exact binding before
  advancing. Project files are read as postconditions only; actual edits remain
  GUI-driven. Full expected-project equality and exporter failure assertions remain.
- Failed replays retain failure.json and expose game/phase/type/message as escaped
  GitHub annotations; mismatch errors include a bounded actual/expected diff and
  input waits name the action. Export failure JSON is retained by the workflow.
- Two diagnostic regressions cover local/hosted output and command-injection-safe
  escaping of multiline error content. No engine Rust behavior changed.
- Exact hosted failure cause is unconfirmed. Synchronization changes address known
  replay weaknesses; do not claim the unread remote exception was reproduced.

## Local validation

- just check passed; 33 Python tests passed; actionlint and mdBook passed.
- Previous 700 Rust tests passed for unchanged engine source (zero skipped).
- Updated authoring/export replay passed for all three games: exact expected
  project equality, all six exports/notices, and deliberate overwrite rejection.
  New per-action saved-input postconditions passed in Skybridge.
- Prior full three-game native/browser victories and persistence acceptance:
  docs/evidence/A01_REFERENCE_GAMES.json. Full moved installation integrity,
  template/export/relocated-play acceptance: docs/evidence/A01_PORTABLE_ACCEPTANCE.json.
- Current raw evidence: target/a01-fourth-*. Combined diagnosis:
  docs/evidence/A01_CI_BASELINE.md. Generated artifacts stay outside Git.

## Next / known issues

1. Push after local replay verification and inspect fresh reference-game annotations.
   A01 stays open until required hosted reference-game and candidate/artifact checks
   pass on applicable source. General CI/desktop archives/Windows candidate now pass.
2. Git push and approved public GitHub HTML reads work. GitHub CLI API is Forbidden;
   authenticated logs/PR creation unavailable. Preserve unknown/failed outcomes.
3. Linux candidate uses expensive fresh/replay release builds; do not infer success
   from its passing artifact preflight or replace its checks with cached binaries.
4. After A01 acceptance, continue A02 durable authoring/unsaved source recovery.
   docs/ENGINE_COMPLETION_PLAN.md owns ordering and all later requirements.

Software graphics/Xvfb/Chromium/null audio do not establish physical acceptance.
Full installers/signing and broad device/platform acceptance remain later gates.
No release gate is closed and full engine parity is not claimed.
