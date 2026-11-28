# A07 preview, cues, blending and cutout skeletons — 2026-11-28

ADR-0117 implements four authoring/playback workflows:

- Draw-only Scene View preview of property tracks, blends and IK. Playback ticks
  outside the composer tab. RAII restoration preserves local/global transforms,
  tint, logical saves and authored data, including a simulated renderer panic.
  Scene canvas gizmos are inactive during preview and Stop Scene Preview restores
  the normal view. Project/scene scope changes clear the preview draft.
- Chronological sound/signal cues, at most 32. Saved per-cue latches prevent repeats;
  pause holds, seek is silent, restart rearms. Signal listeners use the existing
  next-tick entity lifecycle. queue_sound is callback-atomic; native decoded audio
  and platform callbacks deliver the same request.
- Two-take weighted property/pose animation with editable keys/easing, bounded
  weight and optional saved game variable. IK/FK has its own 0..1 weight. Rotation
  is unwrapped; exact endpoints hold. Legacy unextended source remains canonical.
- Undoable two-bone cutout rig creation, hierarchical local bones and rigid sprite
  skin attachments, FK tracks, target tracks and weighted analytic two-link IK.
  Bone IDs, parents, poses and attachments persist through project/logical saves.
  +X rest links, positive uniform scales and physics-free bone ownership are
  validated. Both bends, unreachable/zero-distance targets and rotated/scaled
  ancestors are covered; rejection and callback failures leave local poses intact.

Runtime contract v16 adds solve_ik_2d (140,7 args) and queue_sound (141,1 arg).
No new dependency or VM layout change. Rigid cutout skins use one bone influence
with weight 1. Soft multi-influence vertex deformation, external rig formats,
sprite-frame texture crossfades and automatic blend trees are not implemented.
This completes the described cutout/property workflows, not full A07 release
acceptance or the deferred advanced features.

Executed checks:

- Locked workspace: 946 passing Rust tests, zero failed, 103 suites. Six new editor
  integration tests combine cues/blends/IK, silent seek, paused JSON restore,
  preview draw/restore/unwind, rig creation undo and sample regeneration. Six
  runtime tests cover IK math, ownership/rejection, callback fail/debug suspension
  and queued playback through the native decoded audio engine. Existing egui
  pointer-event test additionally clicks preview, blend, cue and IK controls.
- just check, strict default and opt-out WASM Clippy pass. Matching v16 tools/SDK
  built before GUI replays. Python helper suite 49 passing; Node syntax/whitespace
  checks pass. Logs: target/a07-complete-{focused,rig,widgets,legacy,quality,
  workspace,wasm,wasm-optout,tools,web,python}.log.

The rigged_cutscene sample blends two target takes, solves the cutout arm, plays
one chime, emits two markers and a completion signal. Its listener turns the
beacon blue only after the two markers. Export receipts are recorded below.

No physical speaker/device/DPI/performance or full OS authoring/reimport
acceptance is claimed. Renderer submission/pose restoration and headless egui
widgets are tested; full editor widget workflow remains a separate acceptance
exercise. Remaining A03/A04 gates were not retried.

Final export results: rigged cutscene passes native and browser exports with
1024 green tip pixels at [224,104,256,136] and blue beacon 64. Browser Web Audio
instrumentation observes zero starts before consent and exactly one cue start.
A platform-adapter regression found during replay was fixed and has a dedicated
attachment/failed-callback test. The native decoded audio engine is also tested.
Actual prior v15 browser SDK export rejects against v16 tools.

The original authored cutscene remains a native/browser regression, with 8192
green pixels at [192,56,256,184] and blue beacon 64. Final replays run serially
after builds, with relocated exports/hidden authoring sources and native player
PATH excluding developer tools. Receipts/screenshots:
target/a07-complete-{native,browser},
target/a07-complete-legacy-{native,browser}. Stale SDK log:
target/a07-complete-stale.log. Final pre-commit just check and whitespace checks
pass (target/a07-complete-precommit.log). Generated artifacts are not committed.
