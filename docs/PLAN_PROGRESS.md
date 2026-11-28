# A02–A05 implementation and acceptance progress

2026-11-25, branch `fix/a01-ci-baseline`. Authoritative gates remain in
ENGINE_COMPLETION_PLAN.md. This summarizes implemented work; it does not convert
local feature checks into milestone acceptance. A01 accepted baseline is 6de428b.

| Milestone | Implemented and checked | Remaining acceptance / decision |
| --- | --- | --- |
| A02 authoring | Scene/hierarchy composition, reference guards, copy/paste/duplicate/history, property-preserving projection, project persistence/Save As rescue, whole-project/two-scene crash recovery and component coverage | Every-inspector-property combined widget acceptance and cross-platform GUI remain open. A02 PARTIAL. |
| A03 assets/prefabs | Stable loaded IDs/presets, dependency-aware asset/source moves, source-import rewriting, art refresh, saved deterministic atlases/static clips, staging-only native/browser rebuilds, interrupted atlas recovery, prefab color/texture/geometry/material/clip overrides, reference-safe joint propagation | Nested/variant prefab identity/runtime contract, arbitrary component overrides, general multi-file crash transactions and combined prefab/multi-scene/atlas/export visual acceptance remain open. Deferred by user 2026-11-21. |
| A04 scripting/graphs/debugging | Shared host registry; functions/modules, loops/dictionaries/enums, explicit tasks/timers; source tools; editable connected gameplay graph lowering through the same frontend; staged saved-source restart; actual VM named allocated-local inspection and read-only nested data and bounded read-only expression watches; aggregate tick budgets across scene/entity/event/UI work; shared callable bodies and actual imported-function frames; bounded shared-dependency graph generation with guarded v1 migration; text/graph projectile range and same-tick despawn lifecycle correction | Scene/attachment source breakpoints and resumable statement stepping are implemented; saved-screen UI callbacks remain atomic. Current-frame lexical inspection is implemented. Allocated and strict lexical read-only expression watches are implemented. Equivalent enemy/projectile/trigger game is implemented; full export/debug production acceptance and exact physics-save continuation remain open. A04 PARTIAL; remaining gates deferred under the bounded-attempt policy. |
| A05 rendering/cameras | Shared transformed-quad culling before sprite submission; per-frame queue/drop/missing-texture diagnostics; visible editor/player/browser warnings; real GPU queue and dense export checks; authored dead zones, recoverable finite camera updates and coordinate conversions | Camera shake/transitions/parallax, split view/input, remaining sorting/mask/outline/blend/material/capacity acceptance remain open. A05 PARTIAL and active; A03/A04 gates remain deferred. |

Recent checkpoint history:

- 5bc6e0a: saved atlas recipes and persisted asset identity allocation.
- c58b5ef: interrupted generated-atlas recovery, including actual crash/restore widgets.
- 0431883: explicit texture-clear persistence through TOML/JSON.
- cc3c971: staged joint propagation, owner remapping and atomic reference rejection.
- cebb055: per-instance sprite geometry overrides.
- 653f5ad: material/clip overrides and generated-clip reference protection.
- b186177: safe saved-source Restart preserves old run on preparation failure.
- 465328d: named allocated locals from real executing gameplay VMs.
- bb337cb: read-only watches on actual scene/entity data.
- de8c42d: shared instruction work across a gameplay tick, configuration,
  diagnostics, usage display and callback/reset/cleanup coverage.
- Current ADR-0099 checkpoint: shared function bodies, real imported call frames,
  depth/recursion guards, save-layout compatibility, and matching native/browser
  exports of the imported-function sample.

Evidence files under docs/evidence describe validation boundaries for each step.
Earlier atlas work includes actual Linux widgets and relocated native/Chromium
pixels. The shared-budget checkpoint also has actual Linux editor budget failure,
live watch and corrected-source Restart proof with unchanged manifest. Other
recent debugger paths retain headless/compiled-UI scope. No physical GPU or new
browser-runtime certification is claimed for the budget checkpoint.
A03 stays deferred. Continue A04 against the open acceptance gates above.

Current locally executable checks also pass for actual imported frame/watch/Restart
widgets and relocated native/Chromium function-call exports (4096 green pixels in
each). New native/browser CI steps retain these receipts. External hardware/device
certification is outside the user's current request; it does not block continuing
A04 software work.

Current ADR-0100 checkpoint replaces exponential graph source expansion with one
lazy case per reachable node. Depth/cycle/bounds validation and inactive branch
semantics remain guarded. Canonical v1 documents and previously oversized drafts
load without rewriting; explicit visual save upgrades to v2. See
[evidence](evidence/A04_SHARED_GRAPH_DEPENDENCIES.md).

2026-11-24 ADR-0101 checkpoint: script and editable graph projectile range, real
sensor/input parity and logical-save victory outcomes; text/boolean graph equality
pins; prevent non-stop callbacks after same-tick self-despawn. Exact resumed physics
poses diverge and remain blocked rather than retried. User policy: maximum three
focused attempts per blocked step, retain evidence and continue independent work.
See [range evidence](evidence/A04_PROJECTILE_RANGE.md).

2026-11-24 ADR-0102 checkpoint: isolated bounded expression watches over allocated
locals, explicit slots, instance state and shared game data. Normal compiler/VM
operator semantics and shared pure container reads; mutation/statement rejection,
copy/AST/instruction limits, and unchanged gameplay state/allowance regressions.
Active lexical scopes and scheduler/source stepping remain open. See
[expression evidence](evidence/A04_EXPRESSION_WATCHES.md).

2026-11-24 ADR-0103 checkpoint: compiler lexical binding ranges, current-frame
active local inspection, strict active:NAME/scope(name) watches and shadowing
resolution. Preserve allocated views; filter before preview bounds. Initialized
parameter/local lifetimes and loop exits/continue/completion are checked. See
[scope evidence](evidence/A04_LEXICAL_SCOPES.md).

2026-11-24 ADR-0104 checkpoint: actual source-line breakpoints, imported statement
stops and resumable scene/attachment ticks with frozen events/input/time and
unchanged remaining budgets. Stop abandons pending commands; unfinished saves
reject. Widget functional checks were observed, but the whole replay gate remains
incomplete after three attempts (setup-baseline assertion). Per user policy stop
retrying that gate and continue to A05; A04 remains PARTIAL with UI-debug/production
and saved-physics continuation limits. See [source evidence](evidence/A04_SOURCE_CONTINUATION.md).

2026-11-25 ADR-0105 checkpoint: offscreen sprites no longer starve visible queue
slots; conservative actual-quad bounds include pivot, scale and rotation. Invalid
views reject before world traversal. Renderer counters and world-capacity/fallback
warnings are exposed in all three hosts. Stress fixtures are generated, not checked
in as large TOMLs. See [A05 evidence](evidence/A05_CULLING_AND_CAPACITY.md).

2026-11-25 ADR-0106 checkpoint: authored full-width/height world dead zones,
minimal follow outside the box, validation before camera-state commit, finite
projection/mapping arithmetic and v7 tool compatibility. Native/browser exact
inside/outside pixel bounds plus default-game replays pass. See
[camera evidence](evidence/A05_CAMERA_DEAD_ZONES.md).

2026-11-25 ADR-0107 checkpoint: script-triggered deterministic bounded camera
shake, cancellation and shared editor/native/browser delivery, without follow
feedback. Runtime v8 tools rebuilt; v7 export rejected. Native/browser motion and
exact-rest replays pass for expiry and cancellation. A05 remains PARTIAL; next
transitions/parallax/split views. See [shake evidence](evidence/A05_CAMERA_SHAKE.md).

2026-11-27 ADR-0108 checkpoint: interruptible smooth scripted camera pan/zoom,
endpoint holding, cuts and cancellation to current authored follow. Shared effect
queue/adapter provides editor/native/browser parity, pause and callback atomicity.
Contract v9 tools rebuilt. See [transition evidence](evidence/A05_CAMERA_TRANSITIONS.md).
A05 stays PARTIAL; parallax/multiple views and deferred A03/A04 gates remain open.

2026-11-27 ADR-0109 checkpoint: authored per-sprite parallax factors/anchors,
validated editor controls/undo, logical-save preservation and shared pre-cull
geometry adjustment. Authoring stays in world coordinates; Play previews camera
scrolling. Contract v10 tools rebuilt; old runtime rejected. A05 remains PARTIAL;
tile/particle/repeated backgrounds and split views remain separate work. See
[parallax evidence](evidence/A05_SPRITE_PARALLAX.md).

2026-11-27 ADR-0110 checkpoint: independent prepared sprite-view uploads, stale/
foreign resource rejection and pending-queue cleanup on texture replacement.
Bounded viewport routing and clipped gameplay pointer validity reach all current
hosts; source-debug/UI/attached callbacks retain captured validity. Real two-view
GPU pixels and native/browser pointer phases pass. Contract v11 tools rebuilt.
Authored split-screen scheduling/configuration remains open; A05 stays PARTIAL.
See [view/input evidence](evidence/A05_VIEW_UPLOADS_AND_POINTERS.md).

2026-11-27 A05 checkpoint: reject gameplay pointer mapping for collapsed/nonfinite
projections and clear stale draw-call diagnostics on texture replacement. User
now authorizes advancing sequentially through A07; A05 remains PARTIAL with
remaining split-screen/rendering acceptance open.

2026-11-27 A06 ADR-0111: sparse render/collision queries visit stored chunks,
retain deterministic order and reject invalid geometry. Saturating map extents
and checked atlas dimensions remove overflow paths. Focused core/runtime tests
and just check PASS. A06 remains PARTIAL; advancing to authorized A07 work.

2026-11-27 A07 ADR-0112: authored ping-pong and speed, canonical f64 frame/event/
seek timeline, silent pause and speed-correct transition leftovers. Nondefault
controls survive snapshots and editor undo/redo. Full workspace 907 Rust /49
Python, quality/WASM and native/browser sparse-map/playback/pointer fixtures PASS.
A05/A06/A07 stay PARTIAL; requested progression stops at A07. See
[A06/A07 evidence](evidence/A06_A07_SPARSE_MAPS_AND_PLAYBACK.md).

2026-11-27 ADR-0113: character animation state machines with ordered conditions,
finite numeric/bool parameters, wildcard interrupts and one-shot finish edges.
Validated inspector drafts, component undo/redo, script callback atomicity and
logical saves are integrated. Native/browser real-keyboard movement/action/idle
replays pass. v13 tools rebuilt. A07 stays PARTIAL; blend/timeline/rig acceptance
remains open. See [character-state evidence](evidence/A07_CHARACTER_STATES.md).

2026-11-27 A07 ADR-0114: saved script cutscene movement/tint tracks with four
curves, pause/seek/cancel and shared bounded ownership. Camera-follow rig demo
passes native/browser exports. 924 Rust tests, 49 Python helpers, quality and
strict WASM checks pass; v14 rejects stale SDKs. A07 remains PARTIAL; graphical
timeline, general/audio/event tracks, blending and skeletal rigs/IK remain open.
See [cutscene evidence](evidence/A07_CUTSCENE_TWEENS.md).

2026-11-28 A07 ADR-0115: local rotation and XY scale join saved cutscene tracks.
Multi-turn spins and positive squash/stretch share pause/seek/cancel and capacity.
Fixed script Transform/Sprite removal using single-component ECS removal.
Contract v15; A07 remains PARTIAL. See
[transform evidence](evidence/A07_TRANSFORM_TWEENS.md).

2026-11-28 A07 ADR-0116: graphical source-backed cutscene keyframes, easing,
value scrubbing and undo/redo in Animation Editor. Embedded authoring data reloads
from canonical generated scripts; guarded source staging/saving preserves manual
and external edits. 934 Rust tests and quality checks pass. Existing v15 runtime
contract retained. A07 stays PARTIAL; scene preview, broad/audio/event tracks,
blending and skeletal rigs/IK remain open. See
[composer evidence](evidence/A07_CUTSCENE_COMPOSER.md).

2026-11-28 A07 ADR-0117: Scene View draw-only preview/restoration, saved once-only
sound/signal cues, two-take property blending and cutout skeletal creation plus
weighted FK/IK. Runtime v16; 946 Rust tests, quality and strict WASM checks pass.
The supported skeleton uses rigid single-bone sprite skins. Soft weighted vertex
meshes, atlas crossfades, automatic blend trees and full OS/performance acceptance
remain open. See [workflow evidence](evidence/A07_ANIMATION_WORKFLOWS.md).

2026-11-28 A07 ADR-0118: actual soft multi-influence vertex skins, dual-texture
crossfades and automatic parameter-driven 1D clip trees. Bind/weight/threshold/fade
controls persist through project transactions and logical saves. 953 Rust tests,
49 Python helpers, quality/WASM and native/browser intermediate-pixel checks pass.
Runtime v17; requested features implemented with documented bounds. Full OS and
hardware acceptance remains open. See [evidence](evidence/A07_ADVANCED_ANIMATION.md).
