# Kiln Session Handoff

Active instruction: finish A01 through A04; do not close a gate without its listed
acceptance. Branch `fix/a01-ci-baseline`, baseline `27fa148`. Full 3D stays gated
on G2D; no release-size ceiling; MIT OR Apache-2.0 remains settled.

## Done in this checkpoint

- Persistent optional EditorFlags components implement inherited lock/hide.
  Locks guard hierarchy, component, transform, sprite-property and Sprite Editor
  paths. Hide filters edit-mode sprite/tile/particle drawing and collider outlines;
  player/Play rendering ignores editor hide. Hierarchy context menus unlock/show.
  Scene snapshots and prefab propagation/revert preserve instance editor flags.
- Viewport sprite picking follows painter order and actual rotated/pivoted/scaled
  quads. Q-mode marquee supports additive selection. World-position alignment of
  independent selections is one atomic undo command, without moving children twice.
- Real widget acceptance passed: viewport select/copy, group alignment and undo/
  redo, hide in Edit/show in Play, lock rejecting a viewport drag, two scenes,
  real 30-second autosave, forced kill/restore/save and identical restart properties.
  Raw evidence: target/a02-scene-controls/result.json and editor/reopened captures.
- Added regressions for inherited guards, transaction rollback/history, parent/
  child alignment, negative-scale/rotated picking, player visibility and metadata
  preservation on snapshot, prefab propagation and revert.
- A01 browser-export radio/path entry now uses separate held events. Export requests
  log their actual target/output; receipt waits name the failed game/target.
  New editor-to-browser acceptance publishes exact authoring/browser exceptions,
  retaining browser logs and all assertions. Scene-controls smoke is added to CI.

## Validation

- just check passed (strict workspace all-target Clippy, fmt, native/audio checks).
- Workspace suite: 731 passed, zero skipped; four focused scene-controls tests
  include inherited guards, alignment, player visibility and prefab-flag retention.
- 34 Python regressions and actionlint passed. Native editor/player/CLI built.
- tests/native/scene_controls_smoke.py passed on actual Linux Xvfb/software graphics.
  Existing A02–A04 baseline evidence remains docs/evidence/A02_A04_AUTHORING.md.

## A01 hosted observations / remaining work

On 27fa148: Windows candidate 43574843434 PASSED (7m47); Linux candidate
43574843462 was running. General CI 43574843465 FAILED only in browser job
132386754460, step 13: Verify editor-authored counted loops and dictionaries in
browser. All three-OS checks/build/archive groups, docs, Rust/Python and native UI
passed. Reference games 43574843470 FAILED after Switchyard native export and its
expected overwrite rejection, before the browser receipt. Binding failure progressed.
Public browser-step logs return 404; exact browser exception is not established.
The diagnostic and held-event followups need fresh matching-source hosted results.

Continue A03 resource/dependency coverage, prefab nesting/variants/atlas production
workflow and platform presets; then A04 enums, scheduler, source tooling and actual
live source debugging/graph acceptance. A02's broad combined property/transform and
cross-platform acceptance remains open despite the newly passing Linux workflow.
Keep docs/ENGINE_COMPLETION_PLAN.md authoritative; A01–A04 are not marked accepted.

Toolchain: source /workspace/.kiln-tools/activate.sh; use CARGO_INCREMENTAL=0,
CARGO_PROFILE_DEV_DEBUG=0, CARGO_PROFILE_TEST_DEBUG=0. Rust 1.99.1, nextest0.9.151;
Chromium local tests use CHROMIUM_PATH=/usr/bin/chromium. just check before commits.
Git push/public GitHub HTML work; authenticated API is Forbidden. Software targets
are not physical-device certification. Do not kill unrelated old editor processes.
