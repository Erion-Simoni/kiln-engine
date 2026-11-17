# A02 centered project-error observation — 2026-11-17

Branch `fix/a01-ci-baseline`, following `7796386`. A02 remains PARTIAL.

## Hosted failure and reproduction

[CI 43725520708](https://github.com/Erion-Simoni/kiln-engine/actions/runs/43725520708)
on `7796386` failed at manifest rescue with
`Save visibly refuses deleted source manifest timed out`. The error window had
been deliberately centered and moved to the foreground in that production change,
but the rescue replay still searched the old top-left rectangle (x=20..699,
y=120..399). Local replay reproduced the same failure. Its screenshot visibly shows
the save refusal in the center and unsaved Player X=88 in the inspector.

An offline check of that exact captured frame reports the original observer false
and the centered observer true. Normal Save did refuse the deleted manifest; the
failure was the stale screen observer, not successful unsafe saving.

## Repair

A shared `project_error_visible` screen observer now checks the centered region
(x=400..1039, y=300..599) at the fixture's 1440x900 resolution. It retains the
existing joint/camera replay's stronger red-pixel criteria and threshold (>120).
Both manifest rescue and entity-reference replay use it. No application code,
window placement, project mutation, source/data assertion, timeout or retry budget
was changed. The rescue replay still requires the error to appear and then disappear
on Escape, as well as exact data/resource preservation and restart comparisons.

New regression cases cover centered messages with sRGB color variation; rejection
of the former top-left region, red inspector content and a viewport gizmo; blank,
truncated/wrong-size frames; and the sparse-pixel threshold.

## Executed validation

- Before: manifest-rescue deletion case FAILED with the hosted timeout; screenshot
  inspected. Exact-frame comparison: old false, centered true.
- `just check` PASSED formatting, strict workspace/all-target Clippy and
  workspace/standalone audio checks. No Rust production code changed.
- Python tooling: 44 PASSED (including four new observer regressions).
- `manifest_rescue_smoke.py` PASSED all deleted/corrupt/externally changed cases:
  visible refusal, Escape retaining draft, rescue of unsaved X=88 and exact
  resources, original state preserved, Y=72 saved only to copy, complete restart.
- `entity_reference_smoke.py` PASSED again: invalid/corrected joint Apply, one-step
  Undo, Cancel retaining redo, visible joint/camera deletion guards, resolved
  deletion history and exact full-project restart. Screenshots inspected.
- `py_compile` and `git diff --check` PASSED.

Raw evidence: `target/a02-error-observer-before/`,
`target/a02-error-observer-after/`, `target/a02-error-observer-references/`,
`target/a02-error-observer-logs/`. Existing CI steps/artifact paths are unchanged;
normal CI invocation continues using its default output directories.
These are Linux Xvfb/software-graphics results, not broader GUI/device acceptance.

## Source-pinned hosted boundary

On prior production `7796386`, [Linux candidate 43725520736](https://github.com/Erion-Simoni/kiln-engine/actions/runs/43725520736),
[Windows candidate 43725520747](https://github.com/Erion-Simoni/kiln-engine/actions/runs/43725520747)
and [reference games 43725520724](https://github.com/Erion-Simoni/kiln-engine/actions/runs/43725520724)
PASSED. General CI failed at the observer described above. It advanced through the
local-axis step that failed on prior `bf206f9`; the older intermittent timeout's
cause is not established by a later pass. Authenticated detailed logs/artifact
contents were not independently retrieved. Matching hosted results for this repair
remain required; no old result is promoted to newer source acceptance.
