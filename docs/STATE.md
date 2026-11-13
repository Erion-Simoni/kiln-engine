# Kiln Session Handoff

**Active work:** A01 hosted/replay repair plus the independent A02 script-recovery
substep, as authorized by the owner's request to work through all tasks in batches.
**Branch:** fix/a01-ci-baseline; previous pushed commit 4285924.
**Owner constraints:** no release-size ceiling, keep resource/performance limits;
MIT OR Apache-2.0 settled; full 3D stays gated on G2D.

## Done: A02 script recovery (ADR-0079)

- Versioned recovery envelopes include project plus dirty shared Script Editor
  source, loaded relative path and original disk baseline. Legacy snapshots and
  public project-only API remain compatible.
- Restore populates the buffer without writing source; invalid/unfinished text can
  be recovered. Failed saves and external edits preserve both disk and draft.
  Newer saved manifests keep their scenes while independent script drafts recover.
- Unique atomic sidecars and interrupted temporary files are excluded from Git and
  Save As copies. Successful Save/explicit Discard clears recovery. Escape keeps
  it; unknown/unreadable snapshots remain visibly pending and are not erased.
- Fixed normal project/exit Discard leaving a recovery snapshot behind.
- Added actual GUI autosave/crash/restart coverage to native CI, with evidence upload.
- Contract: docs/SCRIPT_RECOVERY.md. Evidence: docs/evidence/A02_SCRIPT_RECOVERY.md
  and A02_SCRIPT_RECOVERY.json. Full A02 scene/gizmo/selection/undo acceptance is open.

## Validation

- just check passed; 705 Rust tests passed, zero skipped (five new recovery tests).
- 33 Python tests passed; workflows pass actionlint; mdBook build checked.
- Actual 30-second autosave, OS kill/restart, Restore/Save, Escape preservation,
  explicit recovery Discard, normal exit Cancel/Discard and unsupported-snapshot
  preservation passed in the real editor. No successful snapshot was injected;
  only the unsupported-version error case uses a synthetic invalid sidecar.
- Raw evidence: target/a02-*; generated files are not committed.

## A01 hosted status and replay work

- Source 4b7f488: general CI 43509387416, Linux candidate 43509387420 and Windows
  candidate 43509387540 all PASSED, including actual archives, full configured
  software acceptance, artifact upload and origin attestations. Reference games failed.
- Source 4285924: general CI 43515183192 PASSED; Linux candidate 43515183244 was
  running at observation. Reference games 43515183354 FAILED with a useful annotation:
  Emberwatch export timed out after native export succeeded; its next expected
  overwrite rejection was not observed. Root cause is not proven yet.
- A held hover/press/release repair (matching passing native smoke input delivery)
  and a named overwrite wait are being verified locally; keep assertions intact.
- Durable hosted snapshot: docs/evidence/A01_HOSTED_STATUS.json.

## Next / remaining scope

1. Complete the held-click three-game local replay and push the tested batch.
   Inspect fresh hosted reference-game/candidate results; A01 remains open until
   applicable source-pinned checks pass. Do not promote inferred fixes to acceptance.
2. Continue the rest of A02: two-scene widget authoring, complete transform/parent/
   selection/undo and restart/property-reference acceptance. Recovery completion
   does not close that broader milestone.
3. Keep following docs/ENGINE_COMPLETION_PLAN.md. No full 2D/3D release gate is closed.

Git push/public HTML reads work; GitHub CLI API is Forbidden, so detailed authenticated
logs/PR creation are unavailable. Software graphics/Xvfb/Chromium/null audio and
hosted portable tests do not establish physical-device or signed-installer support.
