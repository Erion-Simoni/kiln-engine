# A02 complete floating-panel header observation

2026-11-18, branch `fix/a01-ci-baseline`, following `892d7ff`. A02 remains PARTIAL.

## Trigger and repair

Source-pinned hosted reference run 44083869121 failed during Skybridge authoring:
`Verify floating panel reached widget origin timed out`. Windows 44083869158 and
Linux candidate 44083869194 passed. CI 44083869329 failed its native UI job with
only a generic exit status available; its precise cause is not established here.
A normal local Skybridge replay passed before this change, so the hosted symptom
was intermittent rather than an unconditional authoring failure.

A controlled real-window overlap exposed the observer defect: an enlarged Source
Editor occupied x15..1101, while the foreground Animation Editor occupied
x839..1197. The longest dark title segment belonged to the background window
(origin 33); the complete foreground segment began at 857. The previous helper
therefore accepted the wrong window as already positioned.

The shared replay helper now checks the disclosure triangle and close X around a
complete header, refuses ambiguous disjoint headers, and waits for three identical
position-and-width observations. Its triangle check handles hovered antialiasing.
Pointer positioning and button-down are separate commands so the drag starts at
the observed header. Failed verification records menu, row and observed bounds.
Authoring uses held clicks when closing tabs to avoid retaining Source Editor.
No application hooks, production behavior changes, dependencies, longer retry
budgets or weakened project assertions were introduced.

## Executed evidence

- Before/after unit controls: overlap targeting and hovered-arrow cases failed
  before repair; all 49 Python reference-process tests now pass.
- The same final real-widget overlap scenario fails with the helper loaded from
  `892d7ff`, then passes with this change. It creates and saves an actual animation
  clip over an enlarged Source Editor, checks its texture/frame, and verifies every
  script byte is unchanged. Final screenshots were inspected.
- The overlap test deliberately leaves Source Editor open and selects File / Save
  Project explicitly. Ctrl+S can be consumed by Source Editor; shortcut routing
  is not changed by this test. The fixture includes its existing default script.
- Full installed portable-editor replay passes for Emberwatch, Switchyard and
  Skybridge: exact authored projects/source, unchanged templates, all six native/
  browser exports, bundled notices/runtime verification and overwrite preservation.
- `just check`, Python compilation and `git diff --check` pass. No Rust production
  code changed; the earlier 781-test Rust result belongs to `892d7ff`.
- CI now runs and uploads evidence from `tests/native/panel_origin_smoke.py`.

Raw local evidence: `target/a02-panel-origin-logs` (including baseline-final and
complete widget logs), `target/a02-panel-origin-baseline`,
`target/a02-panel-origin`, and `target/a02-panel-origin-replay/result.json`.
Portable archive SHA-256:
`b72869987f89858ea2bcf8da629c7e55160c88ce0740051f681e8beed1e136cb`.
Generated artifacts remain outside Git.

## Limits and next work

Linux Xvfb/software graphics only. This is authoring/export evidence, not a new
gameplay, physical-device or cross-platform certification. Matching hosted checks
are required after publication; the generic native UI failure is not claimed
resolved without its matching result. Combined every-property/reference recovery
acceptance and cross-platform GUI coverage remain open. A03/A04 breadth and G2D
remain unchanged.
