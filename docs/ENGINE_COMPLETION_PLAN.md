# Kiln engine completion plan — 2D, 3D and production release

**Authoritative work plan • consolidated 2026-11-13 • baseline master `ae492f1`.**

This is the single source for future scope, ordering, status and release gates.
It supersedes the scheduling and completion claims in the planning files mapped
at the end. Those files retain historical evidence and technical detail; their
old checked boxes do not close items here. Architectural decisions remain in
[DECISIONS.md](DECISIONS.md); measured budgets remain in [BUDGETS.md](BUDGETS.md).

## 1. What finished means

A developer can build, debug, save, reopen and ship original 2D and stylized 3D
games through the editor and supported scripts/graphs, without modifying engine
Rust or hand-repairing project manifests for ordinary tasks. Exported games run
without the repository, editor or Rust toolchain. Documented features survive
project upgrades, error recovery and supported target/device changes.

“Finished” is a supported production release with a bounded feature contract,
not an obligation to reproduce every Unity/Godot/Unreal feature. Advanced research
and optional platform integrations remain explicitly tracked after the core gates.

- **G2D — dependable 2D release:** A01–A15 accepted; Windows/Linux editor and
  native games plus browser games validated. Full 3D implementation stays paused
  until this gate passes. Existing preview visuals/types do not establish 3D readiness.
- **G3D — playable 3D editor:** G2D plus B01–B09 accepted; a complete small 3D
  game can be authored, debugged, saved and exported through the real editor.
- **GPROD — finished core engine:** G3D plus B10–B13 and C01–C06 accepted;
  production asset, team, QA, compatibility, accessibility and release workflows work.
- **GPLATFORM — original full-platform ambition:** GPROD plus D01–D04 accepted;
  macOS, Android, iOS and handheld support have real target evidence. Do not
  advertise this broader completion until those gates pass.
- **Optional extensions:** D05–D08 and R01–R08. Each needs an explicit product
  decision and its own acceptance; they do not silently block the core release.

**Scope reconciliation:** older M18/M26/M61 plans coupled mobile to earlier
release numbers; the newer Task 9 plan explicitly defers mobile breadth. This
plan keeps that newer ordering, retains all mobile work under GPLATFORM, and
uses named gates instead of claiming the old version numbers are already earned.
No required item may be moved to optional merely to obtain a green gate; record
any later scope change and its rationale in an ADR and this file.

**Owner decisions preserved:** no package/release-size target or ceiling; MIT OR
Apache-2.0 remains settled. Memory, startup, frame-time and bounded-input goals
remain active. No new engine dependencies or architecture changes without an ADR.
This document authorizes planning, not publication, paid services or SDK agreements.

## 2. Verified starting point and status rules

The reviewed master includes editor-authored Emberwatch, Skybridge and Switchyard,
shared source editing, Quick Actions, scene history, prefabs, Rapier 2D physics,
cameras, sprite clips/tilemaps/particles, saved UI/audio/settings and actual exports.
Existing local evidence for the identical source tree records 700 Rust tests,
29 Python tests, three real authoring replays, six exports and native/browser play.
See [integration evidence](evidence/task9-games/merge-validation.json).

Latest public observations from the 2026-11-13 review:

| Check | Source | Observed result |
| --- | --- | --- |
| [General CI](https://github.com/Erion-Simoni/kiln-engine/actions/runs/43405013257) | ae492f1 | Failed |
| [Release candidate](https://github.com/Erion-Simoni/kiln-engine/actions/runs/43405013163) | ae492f1 | Failed |
| [Windows candidate](https://github.com/Erion-Simoni/kiln-engine/actions/runs/43405013305) | ae492f1 | Passed |
| [Reference games](https://github.com/Erion-Simoni/kiln-engine/actions/runs/43404644543) | f7c8f9d, merged into master | Failed |

Failure causes were not established: additional job-detail retrieval hit an
automatic-approval service usage limit. Refresh conclusions before implementing
A01. A previously passing candidate does not validate newer source.

Statuses below: **PARTIAL** = useful implementation/evidence exists but listed
acceptance remains open; **PLANNED** = not accepted as an integrated workflow;
**BLOCKED** = a named external prerequisite is missing; **ACCEPTED** requires
recorded evidence. An isolated library, stub, panel or design is not acceptance.
Every unchecked deliverable is work to verify or complete, not a claim its
entire implementation is absent. No release gate is currently accepted.

## 3. Execution and evidence contract

Work on one milestone at a time; split large items into reviewable substeps.
Reuse existing implementations and tests before replacing systems. Dependencies
below are minimum prerequisites; listed phase order is the default work order.
Each milestone gets an owner when started, a branch/source SHA and an evidence row.

For every milestone:

1. State the supported authoring/runtime contract and any migration before coding.
2. Implement editor, serialization, runtime and export paths together where relevant.
3. Add meaningful failure/regression tests; run `just check` before every commit
   and appropriate `just test` cases. Test feature-off and target builds affected.
4. Exercise real widgets and the exported game, including save/reopen and errors.
   Headless model tests do not establish GUI or physical-device acceptance.
5. Record source/tool versions, commands, nonzero test counts, screenshots/replays,
   OS/GPU/driver, resource measurements, limitations and artifact identities.
6. Mark ACCEPTED only when all required targets pass. Retain failed/unrun results;
   do not remove assertions or silently substitute synthetic state for gameplay.

Use Windows/Linux/macOS native runners for claimed native platforms; browser
acceptance names browser/version/backend. Include focus loss, resize, DPI changes,
controller/audio availability, suspension and device loss where applicable.

Performance goals come from current BUDGETS.md: record median and tail frame
latency, CPU/GPU/script/physics cost, startup, RAM/VRAM and allocations against a
specified scene, resolution and tier. Preserve the existing startup and frame
budget goals; set missing workload-specific budgets before optimizing. Treat
Intel HD 520/i7-6600U as a candidate Low reference until measured. Do not promise
that every 3D effect works on it. Lower tiers need explicit supported fallbacks.
Package bytes may be reported, but never used as a completion ceiling.

Evidence ledger template (one row per accepted substep/target):

| Milestone | Source SHA | Target/device | Commands and actual outcome | Evidence | Remaining limitations | Status |
| --- | --- | --- | --- | --- | --- | --- |
| A01–R08 | Unassigned | Unmeasured | Not run for this plan | Link when executed | Explicitly list | Open |

## 4. Phase A — finish the 2D engine

### A01 — Restore trustworthy CI and baseline

**Status:** PARTIAL. **Depends on:** None.

- [ ] Diagnose current CI/release/reference-game failures; repair causes without weakening checks. Run reference-game authoring/export/play acceptance on master and relevant pull requests. Align native, audio, WASM, feature-off and browser gates; detect stale/mismatched runtime bundles.
- [ ] **Acceptance:** A fresh source revision passes required hosted checks, all three game replays and artifact verification; deliberate exporter failures still fail visibly.

**Progress 2026-11-13:** repaired reproducible documentation build failures and
stale saved-UI/native/browser export smoke interactions; promoted reference-game
checks to master/main and pull requests. Local strict checks, 700 Rust tests,
29 Python tests and the three repaired UI smokes pass. See
[A01 repair evidence](evidence/A01_CI_BASELINE.md). Hosted failures, complete
three-game rerun and artifact verification remain open; A01 is not accepted.

**Second batch 2026-11-13:** moved cargo-dist configuration into supported metadata;
validated three desktop plans and experimental-archive scope; repaired upload path,
CI tool/dependency pins and fix-branch triggers; bounded replay cleanup with two
child-process regressions. Fresh matching tools authored/exported all three games;
settled Input Rebinding panel capture repairs the Skybridge replay race. All six
native/browser games pass victories and persistence checks; see
[reference-game record](evidence/A01_REFERENCE_GAMES.json). Full
archive builds, fresh hosted/candidate verification and physical targets remain open.

**Third batch 2026-11-13:** fresh hosted docs/tests/native-UI checks passed on
aeda8dd; archive failure exposed a masked installer HTTP 404 and missing dist
profile. Corrected platform-native install assets/version checks, added the profile,
kept independent archive diagnostics and enabled candidate workflows on fix branches.
Full moved-editor integrity/template/export/relocated-play acceptance passes; see
[portable acceptance](evidence/A01_PORTABLE_ACCEPTANCE.json). Fresh hosted archive
and candidate outcomes remain required before A01 acceptance.

**Fourth batch 2026-11-13:** source 4b7f488 passes full hosted general CI,
including actual Linux/Windows/macOS archive builds and native/browser/portable
checks; Windows candidate and origin attestation also pass. The reference-game
run fails in authoring/export; its exact exception is not yet available. Linux
candidate remains in progress. See [hosted snapshot](evidence/A01_HOSTED_STATUS.json).
Replay now waits for rendered panels and exact saved input postconditions, and
publishes specific, escaped failure annotations/diffs. These synchronization and
diagnostic repairs require a fresh hosted rerun; A01 remains PARTIAL.

**Fifth batch 2026-11-13:** 4b7f488 Linux candidate also passed. Source 4285924
has green general CI, but the reference-game annotation identifies a timeout after
Emberwatch's successful native export, before expected overwrite rejection. Export
buttons now use separated hover/press/release events, matching the passing native
smoke; focused local authoring/export checks pass for all three games. The change
still requires hosted confirmation. The independent A02 recovery substep below is
implemented and tested under ADR-0079; broader gates remain open.

**Sixth batch 2026-11-14:** source 16b0e7b passes general CI and the Linux
candidate. Reference authoring now identifies Skybridge's first input binding;
Windows capture selected a tiny helper before the main client was ready. Held
binding presses and largest-valid-client selection repair those specific paths,
with regression coverage. Fresh matching-source hosted results remain required.

### A02 — Durable projects, scene editing and recovery

**Status:** PARTIAL. **Depends on:** A01.

- [ ] Finish full create/open/save/save-as, scene tabs, hierarchy, multiselect, local/world move/rotate/scale, snapping/alignment, copy/paste, lock/hide and undo workflows.
- [x] Recover unsaved script buffers alongside project drafts; preserve guarded saves and external edits. Versioned sidecars read legacy snapshots, keep newer saved projects, and require explicit discard. See [recovery contract](SCRIPT_RECOVERY.md).
- [ ] **Acceptance:** Author two scenes through widgets, edit/reparent/duplicate/delete, undo/redo, force-close and recover, then restart and compare every supported property/reference. Original files survive failed writes.

**Recovery substep 2026-11-13:** implemented under ADR-0079 while A01 hosted
verification continues. Source drafts restore without writing files; guarded saves
preserve external edits, newer manifests keep their saved scenes, and legacy/corrupt
snapshots are handled explicitly. Real 30-second autosave/force-crash/restart,
Restore/Save, Escape, recovery Discard, normal exit Cancel/Discard and unsupported
snapshot preservation pass. [Evidence](evidence/A02_SCRIPT_RECOVERY.md). The full
two-scene/gizmo/selection/undo acceptance above remains open.

**Authoring batch 2026-11-14:** entity clipboard transactions capture selected
subtrees, retain detached world poses and supported components, remap parent/joint/
prefab-instance IDs, and paste once into scene history. System clipboard output
is now handled. Two-scene widget Copy/Paste, undo/redo and save/restart pass.
Manifest saves detect external edits/deletion/corruption before writing; Save As
refuses another existing manifest. Lock/hide, alignment and the full combined
transform/recovery acceptance remain open. See [batch evidence](evidence/A02_A04_AUTHORING.md).

**Scene-controls batch 2026-11-14:** added saved/inherited editor lock/hide,
command guards, viewport sprite picking and marquee, and one-command world-position
alignment. Real two-scene selection/copy/alignment/undo, hidden-in-Edit/visible-in-
Play, locked drag rejection, forced crash/recovery and restart comparison pass.
Workspace flags survive prefab propagation. Windows candidate now passes on
27fa148, but matching general CI/reference results failed and remain under repair.
A02's broad cross-platform acceptance is still open; see STATE.md for current work.

### A03 — Assets, sprites and prefab production workflow

**Status:** PARTIAL. **Depends on:** A02.

- [ ] Complete stable IDs/dependency tracking, search/filter/previews, import/reimport/file-change detection, rename/move, missing/duplicate diagnostics, atlas packing, slicing/pivots and platform import presets. Verify nested prefabs, variants, override apply/revert and reference-safe propagation.
- [ ] **Acceptance:** Reimport renamed art used by prefabs in multiple scenes; retain overrides and IDs, rebuild an atlas, reopen and export with identical intended visuals and no stale references.

**Asset batch 2026-11-14:** project-file polling refreshes browser listings,
previews and rendered PNGs while retaining the selected preview/filter and project
asset IDs. Recovery/export outputs are excluded. Atlas placement is now shared by
fit validation, pixel copy and UVs, with atomic rejection, image/name validation and
fallible allocation. Pixel/preview tests and actual rendered reimport pass.
Polling uses size/mtime; content-preserving metadata changes, the complete renamed-
prefab/atlas/export workflow, platform presets and nested/variant acceptance remain
open. See [batch evidence](evidence/A02_A04_AUTHORING.md).

**Resource/import batch 2026-11-14:** one typed reference visitor now covers
prefab overrides, clips, tilesets, scripts, fonts, audio and shader/material
textures. Stable IDs move with references. Import defaults and separate target
presets persist through real widget rename/save/restart; background diagnostics
report missing dependents and duplicate IDs/content. Fixed a real GPU preview
lifetime crash and hidden import controls. Nested/variant prefab and integrated
atlas rebuild/export acceptance remain open. See
[checkpoint evidence](evidence/A03_A04_RESOURCE_TOOLS.md).

### A04 — General scripting, graphs and real debugging

**Status:** PARTIAL. **Depends on:** A02.

- [ ] Retain shared host registry, lifecycle/events/signals, functions/modules/collections and runtime entity APIs. Complete for/while loops, dictionaries, enums/structured values, documented global-state rules, timers/delayed execution and bounded coroutine scheduling; extend existing language constructs rather than duplicate them. Add highlighting, completion/docs, project search, formatter and source navigation; source breakpoints, statement stepping, stack/locals/watches. Wire editable graph pins/connections, compile diagnostics and runtime value inspection to the same semantics. Specify safe reload/restart and script budgets.
- [ ] **Acceptance:** Build a new enemy/projectile/trigger game using scripts and an equivalent graph; debug an imported function, stop infinite work with a useful diagnostic, and reproduce source/export behavior without engine edits.

**Scripting batch 2026-11-14:** scoped counted loops and bounded first-class
dictionaries share the existing IR/host registry and typed saves/preferences.
Added lossless syntax colors and a searchable built-in reference. Bytecode steps
now execute; resume, locals, call-depth stepping and per-execution/single-step
budgets have regression tests. The actual live gameplay debugger still needs
source breakpoints, imported function frames and watches. Enums, scheduling,
completion/search/format/navigation and graph authoring/inspection acceptance
remain open. See [language contract](KILNSCRIPT.md) and
[batch evidence](evidence/A02_A04_AUTHORING.md). A02–A04 remain PARTIAL.

**Source/scheduling batch 2026-11-14:** qualified enums, cooperative staged tasks
and one-shot game-time timers use the shared VM and existing typed saves. Added
indent formatting, shared-registry call insertion, bounded saved-project search
and literal function navigation. Browser movement passes using editor-authored
enums/tasks/timers/dictionaries/loops; save/restart and invalid input regressions
pass. Runtime identities now reject stale same-version language/project contracts.
This does not supply automatic stackful continuations, live source breakpoints,
imported frames/watches or complete graph authoring. Those and the new equivalent
script/graph game remain required. See [checkpoint evidence](evidence/A03_A04_RESOURCE_TOOLS.md).

### A05 — Sprite rendering, cameras and large-scene behavior

**Status:** PARTIAL. **Depends on:** A03.

- [ ] Complete sorting/masks/outlines/blend modes, flipping, materials, pixel scale, atlas batching, filtering/compression and transparent order. Add camera shake/dead zones/transitions, parallax, multiple viewports/split-screen with correct input mapping. Implement suitable culling and report sprite/texture capacity overflow rather than silently dropping submissions.
- [ ] **Acceptance:** Pixel/interaction regressions cover transforms, camera resize/DPI, split view input and sorting; dense/offscreen scenes remain correct at and beyond configured capacity with visible diagnostics.

### A06 — Complete tilemap and level authoring

**Status:** PARTIAL. **Depends on:** A03, A05.

- [ ] Verify palette/brush presets/erase/rectangle and flood fill, grid controls, layer visibility/locking, selection/copy/paste, terrain/autotile rules, animated tiles, collision shapes and previews, chunking and import/export. Add reusable stamp/level tools and streamed or bounded large-map editing.
- [ ] **Acceptance:** Create a multilayer scrolling level entirely in the editor, paint terrain/collision, undo, save/restart and play both exports; edits affect the actual collision geometry and references survive reimport.

### A07 — Animation graphs, timelines and 2D rigs

**Status:** PARTIAL. **Depends on:** A04, A05.

- [ ] Extend existing frame clips/events with ping-pong, speed, conditions, state graphs and transitions. Author property/transform/color/camera/audio/event tracks and curves; expose tween controls to scripts/graphs. Integrate the ADR-selected 2D skeletal approach: bones, attachments, weights, skinning and basic IK with real editor authoring.
- [ ] **Acceptance:** Author idle/run/jump plus a cutscene and a rigged example; events fire once under pause/seek/transition, all tracks/rigs survive reimport/reopen and match native/browser playback.

### A08 — Complete 2D physics and character tools

**Status:** PARTIAL. **Depends on:** A04, A06.

- [ ] Retain real queries, contacts, masks, sensors and joints. Add authored capsules/polygons, shape-editing handles, controller tools for slopes/stairs/moving and one-way platforms, coyote time/jump buffering/wall checks. Define fixed-step/CCD and transform ownership; keep debug drawing accurate.
- [ ] **Acceptance:** Use an editor-authored obstacle course at varied render rates; assert grounding, one-way passage, queries, enter/exit events, body removal, joint edits and save/load without simulation spikes.

### A09 — Input, UI, localization and accessibility

**Status:** PARTIAL. **Depends on:** A04, A05.

- [ ] Complete multiple players/gamepads, dead zones, hot-plug, glyphs, rebinding persistence, vibration where supported, touch/virtual controls and focus handling. Expand saved Panel/Label/Button UI to images, sliders/toggles/dropdowns/text entry/progress/lists/tooltips, scrolling/containers/nine-slice, drag/drop and UI animation. Integrate font fallback, shaping/RTL/plurals, localization tables and scalable accessible controls.
- [ ] **Acceptance:** Build inventory/settings/dialogue screens with mouse, keyboard, controller and browser touch; test resizing, non-Latin/RTL text, missing glyphs, remapping and restart with no stuck inputs or incorrect UI capture.

### A10 — Audio, preferences and game persistence

**Status:** PARTIAL. **Depends on:** A04, A09.

- [ ] Verify actual supported codecs, previews, effects/music, stream long tracks, playback position/fades, named buses/ducking/basic DSP and 2D attenuation. Add device selection/hot-plug/sample-rate handling. Complete multiple save slots, previews/timestamps, migrations and graphics/audio/input preferences, with persistent entities and explicit load/unload/additive-scene rules.
- [ ] **Acceptance:** Play overlapping effects/music on real devices; unplug/reconnect audio; migrate an older game save and resume inventory/quests/scenes. Corruption, quota errors and missing resources preserve previous valid state.

### A11 — Authorable 2D lights, shaders and VFX

**Status:** PARTIAL. **Depends on:** A05, A07.

- [ ] Integrate actual 2D point/freeform/directional-style lights, normal maps, layers, occluders/soft shadows and baked/Low fallbacks. Preserve custom sprite shaders; expose validated material graph authoring and node diagnostics. Complete particle shapes/bursts/curves/gradients/subemitters/trails/collision/flipbooks with CPU fallback and GPU path only where supported.
- [ ] **Acceptance:** Author a lit level and effects in editor, reopen/export and compare pixels/behavior by tier. Record light/particle cost and bounded overload behavior; unsupported effects produce explicit fallback or export errors.

### A12 — Gameplay content and 2D navigation

**Status:** PARTIAL. **Depends on:** A04, A08, A10.

- [ ] Integrate grid A*/2D navigation, dynamic obstacles, path overlays, behavior/state graphs, dialogue/quest tools, CSV data-table editing and inventory/stat examples. Make these reusable saved resources accessible to scripts/graphs rather than disconnected libraries or lists.
- [ ] **Acceptance:** Author a data-driven NPC quest with obstacle navigation, branching localized dialogue and inventory rewards; save halfway, reopen and complete it in exported builds without hand-editing manifests.

### A13 — Performance, stress and independent usability

**Status:** PARTIAL. **Depends on:** A02–A12.

- [ ] Add CPU/GPU timelines, script/system timings, memory/asset/audio statistics, draw-call inspection and warnings. Exercise multi-hour editing/play, large imports/maps, cancellation, recovery, missing/corrupt resources and feature-off builds. Rewrite editor-first getting-started/tutorials; have an independent developer follow them unaided.
- [ ] **Acceptance:** Document measured startup/frame/memory budgets on real Low/Medium hardware, successful soak/recovery runs and an independent original-game tutorial report; resolve observed workflow blockers.

### A14 — Reliable desktop/browser delivery

**Status:** PARTIAL. **Depends on:** A01–A13.

- [ ] Finish real installed-editor export/play on Windows and Linux with no build tools, clean userspace/relocation and offline resource verification. Test claimed browser backends, loading/error/retry, storage/audio consent and hosted paths. Finish icons/versions/build profiles, debug symbols, dependency notices/provenance, OS signing/package verification and public support requirements.
- [ ] **Acceptance:** A clean machine authors and exports all reference games; downloaded verified artifacts run elsewhere, including save/restart. CI and release evidence bind the same source/tools; support limits such as Linux GLIBC floor are explicit.

### A15 — G2D release acceptance

**Status:** PLANNED. **Depends on:** A01–A14.

- [ ] Reconcile all old checklists with executed evidence. Extend the three existing reference games to cover new UI/rigging/navigation/lighting/asset paths rather than rebuilding them from scratch. Publish supported contracts and known limitations; keep outstanding platforms explicitly unsupported.
- [ ] **Acceptance:** Every required Phase A target has evidence and no unresolved release-blocking correctness/data-loss issue. All three games are authored, debugged, reopened and exported by documented workflows; record a named G2D sign-off before B01.

## 5. Phase B — integrated 3D, then production scene scale

Existing feature-gated mesh/material types and the requested 3D visual preview
are scaffolding. These milestones require real scene data, rendering, authoring
and exported gameplay. New crate names in older plans are proposals, not a
requirement to add dependencies before an ADR establishes the need.

### B01 — 3D data, conventions and feature isolation

**Status:** PLANNED. **Depends on:** G2D.

- [ ] Specify units, handedness, up axis, quaternion/Euler conversion, transform hierarchy, nonuniform/negative scale, bounds and serialization. Add real 3D scene/component/camera data through existing command history and stable IDs. Extend scripting/graph vector/quaternion APIs and version/migrate 2D projects. Keep 3D dependencies optional.
- [ ] **Acceptance:** Open old 2D projects unchanged; round-trip parented 3D transforms and entity references. Native/WASM checks with 3D on/off pass; 2D does not instantiate 3D systems when disabled.

### B02 — Production mesh and scene import

**Status:** PLANNED. **Depends on:** B01.

- [ ] Complete glTF/GLB import of meshes, hierarchy, submeshes, materials/textures, UVs, normals/tangents, skins/clips/morph targets; meshopt and supported extensions with explicit rejection/fallback for others. Handle unit/axis conversion, external resource bounds and malformed files. Preserve import subasset IDs and user overrides on Blender/DCC reimport.
- [ ] **Acceptance:** Import an original multi-material animated scene, re-export it from a DCC, and retain placed instances/overrides after reimport and packaging. Malformed/unsupported assets produce actionable errors without corrupting the project.

### B03 — Real 3D rendering and backend lifecycle

**Status:** PLANNED. **Depends on:** B01, B02.

- [ ] Implement perspective/orthographic projection, depth testing, near/far precision policy, frustum bounds/culling, opaque/cutout/transparent passes, color-space/HDR/tone mapping and basic antialiasing. Connect the actual render graph and resource lifetime tracking. Handle resize, minimization, device/surface loss and unsupported GPU capabilities.
- [ ] **Acceptance:** Depth/occlusion/transparency/color-space image tests pass; near-plane objects and resized viewports pick/render correctly. Recover from supported surface loss and validate native/browser 3D or report unsupported capability before export.

### B04 — Complete 3D editor and level construction

**Status:** PLANNED. **Depends on:** B03, A02, A03.

- [ ] Provide orbit/fly/pan, focus selection, perspective/orthographic views, orientation cube/grid, accurate ray/ID picking and marquee selection. Add axis/plane move, rotate, scale, local/world and pivot modes, grid/angle/vertex/surface snapping. Author primitives, meshes, cameras, prefabs, light/physics/audio components with undo and multiple scene views.
- [ ] **Acceptance:** Build a small level entirely with real widgets, duplicate/reparent scaled objects, undo/redo and restart. Saved component values and camera picking agree with runtime; visual preview mode cannot masquerade as saved 3D content.

### B05 — PBR/stylized materials, lights and shadows

**Status:** PLANNED. **Depends on:** B03, B04, A11.

- [ ] Add metallic-roughness PBR, normal/emissive/occlusion maps, texture samplers, IBL/reflection probes and stylized/unlit variants. Implement directional/point/spot lights, measured tiled/clustered lighting, cascaded/directional and local-light shadows with bias/filter controls and Low fallback. Reuse material graph validation and preserve last valid shader after errors.
- [ ] **Acceptance:** Author lit indoor/outdoor scenes and materials, inspect shadow artifacts and transparent objects, then export. Visual and performance checks pass per tier; disabling expensive effects leaves a usable scene.

### B06 — 3D physics and robust character gameplay

**Status:** PLANNED. **Depends on:** B01, B04, A04.

- [ ] Integrate rigid bodies, primitive/convex/static-mesh colliders, layers/masks, triggers, queries, joints, CCD and fixed-step interpolation. Handle collider cooking/reimport, scaling, sleeping/removal and scene unload. Add character slopes/stairs/moving platforms, cameras with collision, interaction rays and a ragdoll example.
- [ ] **Acceptance:** Complete a scripted physics obstacle course at multiple frame rates; assert stable contacts, stairs, queries, character/camera collision and serialization. Editor debug shapes match actual physics; load/unload does not leave ghost bodies.

### B07 — Skeletal animation, morphs and animation debugging

**Status:** PLANNED. **Depends on:** B02, B06, A07.

- [ ] Import skeletons/clips and morph targets; implement GPU skinning with supported fallback, state/blend trees, masks/layers, events, root motion, basic IK and retargeting conventions. Provide skeleton/weight/clip/root-motion inspection and bake/compression settings. Define animation/physics/script update order.
- [ ] **Acceptance:** A playable character blends idle/walk/run/jump, uses an IK or look-at target and root-motion interaction; retargeted/reimported animation keeps stable references and matches exported behavior.

### B08 — 3D audio, navigation and runtime scene lifecycle

**Status:** PLANNED. **Depends on:** B06, B07, A10, A12.

- [ ] Integrate listeners/emitters, attenuation, panning, spatial bus routing and basic occlusion/reverb zones. Add navmesh bake/visualization, dynamic obstacles, off-mesh links, avoidance and scripted agents. Extend camera/input/save/UI APIs for 3D, including world-space UI with ray interaction, distance scaling and occlusion; define additive loading, persistent entities and async cancellation.
- [ ] **Acceptance:** NPCs navigate doors/links and moving obstacles with audible spatial cues; save during a scene transition, restart and restore entities, audio, animation and navigation state without duplicate objects.

### B09 — G3D playable reference game

**Status:** PLANNED. **Depends on:** B01–B08.

- [ ] Build an original third-person adventure or arena through editor/DCC assets and supported scripts/graphs: two levels, animated player/enemies, interactions, UI/settings/audio and save/load. Add a small first-person interaction scene to exercise alternate cameras/controller behavior. Export supported native/browser profiles.
- [ ] **Acceptance:** An independent developer completes and exports the game without engine edits. Fresh-process play, victory/restart/save restoration, source-unavailable relocation and current-source CI pass; record G3D sign-off.

### B10 — Baked lighting and environment workflow

**Status:** PLANNED. **Depends on:** B05, B09.

- [ ] Add lightmap UV validation/generation, cancellable bake jobs, cache invalidation, probes and reflection capture authoring, density/seam/debug views and reproducible packaged bake outputs. Support a skybox/environment and inexpensive fog; bake advanced effects for Low where appropriate.
- [ ] **Acceptance:** Bake and re-bake a changed level, cancel safely and relocate the exported game without source/baker. Compare baked and dynamic appearance/cost and verify probes on moving objects.

### B11 — Scene scale, LOD and resource streaming

**Status:** PLANNED. **Depends on:** B09, B10.

- [ ] Integrate scene-used job scheduling/background import with measured fallback; mesh LOD, instancing, occlusion culling, async chunk/cell loading and mip streaming. Track RAM/VRAM ownership, unloading, texture formats and residency; expose counts/limits. Define hitch and memory budgets before choosing GPU-driven techniques.
- [ ] **Acceptance:** Stress an authored world and repeatedly traverse/unload/reload cells; record frame-tail/hitch and memory bounds. Compare culling/LOD results against unculled reference scenes; no missing objects, dangling references or unbounded growth.

### B12 — Terrain, foliage and larger worlds

**Status:** PLANNED. **Depends on:** B11, B08.

- [ ] Add heightfield sculpt/paint/layers, LOD/seams, terrain colliders, foliage placement/instancing/wind/fade and navigation rebuild regions. Choose origin rebasing or higher-precision world coordinates by ADR, with camera-relative rendering and stable save/streaming IDs. Support streamed navigation tiles and avoidance.
- [ ] **Acceptance:** Edit and stream a terrain world, save far from origin and reload without visible jitter or physics/nav failures. Retain old 100 km precision and 200-agent scenarios as measured stress targets with hardware/workload specified, not current guarantees.

### B13 — 3D cinematics, VFX and scalability polish

**Status:** PLANNED. **Depends on:** B07, B10–B12.

- [ ] Extend timeline to 3D cameras, animation, audio and events, with deterministic seek and image/video capture. Add 3D billboards/meshes/trails/decals/particle collisions and budgeted GPU fallback. Expose quality profiles and opt-in post effects with per-pass cost; preserve readable Low rendering.
- [ ] **Acceptance:** Author a cutscene and effects-heavy gameplay sequence, seek/pause/export it, and compare runtime with capture. Tier switching releases obsolete resources and meets declared quality/performance contracts.

## 6. Phase C — shared production and maintenance

### C01 — Versioned formats and safe upgrades

**Status:** PARTIAL. **Depends on:** A15; extend through B13.

- [ ] Version project/scene/prefab/graph/asset/save schemas and scripting/host/plugin ABI. Add forward-error behavior, backup/rollback, migration previews and fixtures from released versions; stable IDs must survive moves/imports and branch merges.
- [ ] **Acceptance:** Upgrade old 2D and 3D projects and saved games, deliberately fail a migration, recover originals and verify equivalent gameplay. Unsupported future versions never overwrite data.

### C02 — Team workflow and asset build reliability

**Status:** PARTIAL. **Depends on:** A03, B02, C01.

- [ ] Use stable merge-friendly text and deterministic import outputs, dependency graph/background builds, cache invalidation and per-platform profiles. Add scene/graph diff, Git/LFS guidance, validation CLI and reproducible package identities. Integrate external editors/DCC without losing authored overrides; provide a language-server protocol implementation for completion, diagnostics and source debugging tools.
- [ ] **Acceptance:** Two developers independently edit scenes/prefabs/assets in separate clones, merge, validate and build matching intended output. Broken references/cache entries are diagnosed; cancellations preserve valid imports.

### C03 — Production debugging and automated QA

**Status:** PARTIAL. **Depends on:** A13, B09, C01.

- [ ] Add frame/draw/resource debugger, CPU/GPU/physics/script/audio profiling, optional remote exported-game script and graph debugging, deterministic replay where supported, screenshot tolerance policy and automated playtests. Symbolicate crash logs; collect reports only with consent. Fuzz bounded parsers/importers/archives and test malformed projects and device failures.
- [ ] **Acceptance:** Replay 2D/3D bugs from recorded inputs with reproducible seeds/timing policy; explain unsupported nondeterminism. Diagnose an exported crash and a memory leak; CI catches deliberate visual, asset and gameplay regressions.

### C04 — Production UX, docs and accessibility

**Status:** PARTIAL. **Depends on:** A09, B04, C03.

- [ ] Persist sensible docking/workspaces, commands/shortcuts, search, contextual help, property reset/copy, accessibility scaling and diagnostics linked to source/assets. Keep editor-first tutorials, API/graph docs, supported-format matrices and examples current; test multiple DPI/monitors and keyboard-only authoring.
- [ ] **Acceptance:** New developers build original 2D and 3D levels unaided, locate/debug errors and export; record task failures and fix them. Tutorials do not require hand-authoring manifests for ordinary editor operations.

### C05 — Release, licensing and security maintenance

**Status:** PARTIAL. **Depends on:** A14, C01–C04.

- [ ] Define SemVer/project compatibility, LTS/support windows, regression triage and patch policy. Maintain dependency/license/source notices and SBOM/provenance, verify downloaded signatures/archives, protect release credentials and document vulnerability reporting. Verify signed distributions and update/rollback procedures on supported desktop systems.
- [ ] **Acceptance:** Produce a source-bound release candidate with all required gates, independently verify delivered bytes/signatures/notices, install/uninstall/upgrade and recover a failed upgrade. Public releases require the owner’s publication instruction.

### C06 — GPROD core release sign-off

**Status:** PLANNED. **Depends on:** G2D, G3D, B10–B13, C01–C05.

- [ ] Complete reference games and long-session/large-project/device matrices on declared desktop/browser profiles. Audit visible panels and advertised features against real workflows; hide or label unsupported tools. Publish limitations and practical tutorials, support/contribution policies and migration examples.
- [ ] **Acceptance:** Every required milestone is accepted with exact-source evidence, no unresolved release-blocking issue, and independent 2D/3D authoring/export acceptance. Optional extensions and unvalidated platforms remain visibly separate.

## 7. Phase D — platforms and ecosystem (retain original long-term goals)

### D01 — macOS native editor and games

**Status:** PLANNED. **Depends on:** A14, C05; 3D profile also G3D.

- [ ] Build/validate supported Apple architectures, Metal capabilities, DPI/input/filesystem behavior, .app packaging, signing/notarization and clean-machine migration. Obtain required Mac/Apple developer access.
- [ ] **Acceptance:** A real Mac authors and exports 2D/3D profiles and runs relocated signed/notarized artifacts, including sleep/wake, save paths and peripheral changes.

### D02 — Android player and export workflow

**Status:** PLANNED. **Depends on:** A09, A14, C01; 3D profile also G3D.

- [ ] Wire genuine editor/CLI APK/AAB builds with SDK/NDK validation, signing, architecture/device profiles, permissions, orientation/safe areas and touch/virtual controls. Handle surface loss, pause/resume, interruptions and thermal/memory constraints.
- [ ] **Acceptance:** A physical Low and recent Android device install/play/save/resume exported games; signed AAB passes current upload validation. Desktop scaffolding or simulator-only runs do not close this item.

### D03 — iOS player and export workflow

**Status:** PLANNED. **Depends on:** D02 shared mobile contract, D01 tooling.

- [ ] Integrate Xcode/device builds, signing/provisioning, safe areas/orientation, touch, audio interruptions, background restoration and storage. Obtain Mac, Xcode and Apple developer/device access; verify current privacy/store requirements.
- [ ] **Acceptance:** Run native builds on actual supported iOS devices and TestFlight/store validation, restore suspended gameplay and saved progress, and package supported 2D/3D profiles from documented tools.

### D04 — GPLATFORM cross-platform and handheld gate

**Status:** PLANNED. **Depends on:** GPROD, D01–D03.

- [ ] Run Windows/Linux/macOS/web/Android/iOS profiles and Steam Deck handheld checks. Include controller glyphs, 800p UI, sleep/resume, battery/thermal behavior and cloud-save conflicts where claimed. Add an editor-authored mobile casual flagship beside the 2D and 3D samples.
- [ ] **Acceptance:** The same supported game contracts pass real target matrices, all three flagship samples ship with tutorials, and minimum OS/GPU/browser/device requirements are explicit. No console or XR claim is implied.

### D05 — Stores, platform services and safe updates

**Status:** PLANNED. **Depends on:** C05; each platform first validated.

- [ ] Finish itch distribution, a real Steam/platform-service backend for achievements/overlay/cloud saves/leaderboards, and optional IAP/ad adapters with offline fallbacks. Complete signed/hash-verified update download/staging/rollback and content dependency validation; store-managed builds disable competing updaters.
- [ ] **Acceptance:** Test a private distribution/update cycle, interrupted download, tampering, rollback, service outage and cloud conflicts. Refresh fees/SDK terms at implementation; no purchase, account agreement or publication is implicit.

### D06 — Sandboxed plugins, packages, mods and DLC

**Status:** PLANNED. **Depends on:** C01, C02, C05.

- [ ] Implement actual versioned plugin loading, permission/resource limits, isolation and unload for custom nodes/importers/panels; validate any render-pass extension surface. Add package pin/install/update and compatibility errors. Support data mods/load order/conflicts, sandboxed code mods and DLC enable/disable with save compatibility; Workshop adapter if selected.
- [ ] **Acceptance:** Install an example importer/panel plugin and a data/content mod; reject incompatible or over-budget extensions safely, update/rollback and load a save with optional content missing.

### D07 — Multiplayer and dedicated/headless server

**Status:** PLANNED. **Depends on:** B09, C01, C03.

- [ ] Choose transports and native/browser compatibility by ADR; add stable replication IDs, authority/ownership, serialization/version limits, RPC validation, prediction/reconciliation and optional rollback. Implement dedicated server builds, replication visibility, reconnect and editor latency/loss simulation.
- [ ] **Acceptance:** A two-client sample survives simulated latency/loss/reconnect and invalid client messages with authoritative results; test server packaging, bandwidth/performance and compatibility. Single-player release does not require this extension.

### D08 — Console and XR readiness

**Status:** PLANNED. **Depends on:** GPROD; external SDK/device access.

- [ ] Audit platform traits and licensed console SDK integration without promising unavailable targets. OpenXR remains feature-gated with stereo, tracking/controllers, comfort/accessibility settings and explicit performance budgets. Track console and XR as separate subprojects.
- [ ] **Acceptance:** Console acceptance requires authorized SDK/devkit and platform certification evidence; XR requires real headset frame-time/input tests. Missing access is BLOCKED, never a completed portability checkbox.

## 8. Optional advanced engineering and rendering research — never a hidden core-release dependency

Each experiment needs a workload, hardware/backend matrix, measured improvement,
maintenance estimate, ADR go/no-go decision and a working baseline fallback.
A no-go outcome is valid; remove unsupported marketing claims rather than leaving
an unfinished mandatory feature. Existing post effects should be reused.

| ID | Retained scope | Prerequisites | Acceptance to adopt |
| --- | --- | --- | --- |
| R01 | Advanced post: exposure, GTAO, SSR, DOF, motion blur, TAA/ghosting controls; dynamic resolution and spatial/temporal upscaling | B05, B11, C03 | Measured image quality/frame-time gain; stable motion/disocclusion; per-effect costs and Low fallback |
| R02 | Compute culling/indirect draws, GPU-driven 100k-instance workload | B11 | Compare CPU/GPU costs and correctness; CPU fallback passes unsupported-device tests |
| R03 | Virtual geometry/mesh clustering and virtual texturing | B11, R02 | Separate go/no-go studies show residency/streaming benefit within measured memory; baseline paths remain usable |
| R04 | Dynamic GI/probe/SDF/screen-space approaches; optional hardware ray tracing only after API/backend support review | B10, C03 | Temporal stability and cost measured; baked GI fallback; supported targets explicit |
| R05 | Atmosphere, volumetrics, water, time-of-day/weather and wet surfaces | B05, B12 | Author/export a weather sequence; bounded cost, correct depth/transparency and cheap sky/fog/water fallbacks |
| R06 | Advanced character hair/cards, cloth, skin/subsurface and facial tooling | B07, C03 | Animated character test measures deformation/physics/render cost; cheaper material/rig variants remain valid |
| R07 | Script async operations and optional AOT/JIT/IR-to-WASM acceleration; explicit Rust extension FFI contracts | A04, C01, C03, D06 for plugins | Benchmark against VM, preserve diagnostics/semantics and sandbox limits; ABI ownership/version tests; keep VM fallback |
| R08 | Spline-based 2D terrain, roads/rivers and decorative geometry authoring | A06, A08 | Edit control points, undo/reopen, regenerate render/collision geometry and export without broken references |

## 9. Explicit additions closing gaps in the older plans

These are required additions/refinements, not declarations that no related code exists:

- Recover unsaved source and validate error/cancel/crash behavior (A02).
- Replace disconnected UI/animation/AI libraries with integrated authoring contracts
  and source-level debugging (A04, A07, A09, A12).
- Detect capacity overflow and measure actual CPU, memory and tail latency (A05, A13).
- Define 3D coordinate/scale/transform ownership, scene migration and script APIs (B01).
- Preserve imported subasset identity, overrides and skeletal/morph references on DCC reimport (B02).
- Validate color space, transparency, picking, surface/device loss and backend capability gates (B03–B05).
- Add practical collider cooking, character/camera collision, root-motion ordering,
  retargeting, spatial audio and scene-transition persistence (B06–B08).
- Test bake-cache invalidation, navigation/terrain updates and full resource unloading (B10–B12).
- Establish explicit project/save/ABI compatibility, migration rollback, hostile-input
  bounds, independent signing checks and long-term support (C01–C05).
- Add dedicated-server authority/security/reconnection acceptance if multiplayer is selected (D07).

## 10. Source inventory and old-to-new milestone crosswalk

Planning documents were inventoried across root and docs, including names that do
not contain “plan”. Historical evidence/readiness pages informed statuses; specialist
integration/platform documents supply contracts, not separate priority queues.
All future milestone status changes belong here, with evidence linked back.

| Source | Scope retained here |
| --- | --- |
| [ROADMAP.md](ROADMAP.md), M0–M13 and R0 foundations | A01–A14, C03; preserve runtime/render/VM baseline rather than redo it |
| [ROADMAP_FULL_M14_PLUS.md](../ROADMAP_FULL_M14_PLUS.md), M14–M18 | A14, D01–D02, D05–D06; updates/stores/mobile retained |
| Same, M19–M26 including M22a/M22b | A03–A12, D03; rigs, debugger, sprite editor, AI/content and input/save work retained |
| Same, M27–M28 | A13, B03, B11, C03; scheduler/render graph must serve real workloads |
| Same, M29–M34 | B01–B07; import/render/editor/PBR/shadows/material graph/animation/physics |
| Same, M35–M39 | B08–B12; bake/probes, scale, large worlds, navigation, terrain/foliage |
| Same, M40–M47 | B11, B13, R01–R06; high-end research stays optional |
| Same, M48–M52 | A07/A10/A13, B08/B13, C01–C04; sequencer/audio/profiling/assets/team tools |
| Same, M53–M58 | A04/A09, C03–C05, D05–D07; networking/plugins/localization/QA/scripting/mods |
| Same, M59–M61 | C06, D01–D04, D08; full platform, ecosystem, support and flagship gates |
| [2D_COMPLETION_PLAN.md](2D_COMPLETION_PLAN.md), Tasks 1–3 | A01–A03; implementation exists, complete acceptance and remaining breadth |
| Same, Tasks 4–7 | A04–A12; do not mistake limited supported contracts for full parity |
| Same, Tasks 8–9 | A13–A15, C05; retain three existing games and executed export evidence |
| [KILN_2D_V1_ROADMAP.md](KILN_2D_V1_ROADMAP.md), Stages 0–3 | A01–A04 |
| Same, Stages 4–8 | A03–A08, A11; sprite/tilemap/script/animation/physics requirements |
| Same, Stages 9–14 | A03, A09–A11; input/UI/audio/lighting/particles/prefabs/persistence |
| Same, Stages 15–16 and 17-item gate | A12–A15, C03–C06, D01–D04 for expanded platform scope |
| [2D_V1_ROADMAP.md](2D_V1_ROADMAP.md), V1-1…V1-10 | A01–A15; old “only hardware remains” claim superseded; former size gate removed |
| [EDITOR_ROADMAP.md](EDITOR_ROADMAP.md), [FIX_EDITOR_PLAN.md](FIX_EDITOR_PLAN.md) | A01–A13, B04, C02–C04, D06; old editor-specific M30–M60 numbering does not replace 3D milestones |
| [TODO_LIST.md](TODO_LIST.md), [PROGRESS_F6_NEXT.md](PROGRESS_F6_NEXT.md), [fixes m1-29.md](fixes%20m1-29.md) | A01/A05/A09/A13–A15; historical fixes and unresolved claims, not fresh acceptance |
| [2D_READINESS_AUDIT.md](2D_READINESS_AUDIT.md), [2D_GATE_VERIFICATION.md](2D_GATE_VERIFICATION.md) | Baseline history and A15/C06 reconciliation; old G-1…G-6 does not unlock full 3D |
| [M29_RENDERER_PLAN.md](M29_RENDERER_PLAN.md), [M29_EDITOR_3D_PLAN.md](M29_EDITOR_3D_PLAN.md) | B01–B04; design options need implementation and evidence |
| [M29_FEATURE_GATE_VERIFICATION.md](M29_FEATURE_GATE_VERIFICATION.md), [M29_BASELINE_AND_HARDWARE.md](M29_BASELINE_AND_HARDWARE.md) | B01/A13/B03; dependency isolation and real hardware retained, size thresholds superseded |
| [M2_WEB_GAME_LOOP.md](M2_WEB_GAME_LOOP.md) | A01/A05/A09–A11/A14; browser runtime/effect/input parity |
| [ANIMATION_INTEGRATION.md](ANIMATION_INTEGRATION.md), [PHYSICS_INTEGRATION.md](PHYSICS_INTEGRATION.md), [AUDIO_INTEGRATION.md](AUDIO_INTEGRATION.md), [UI_INTEGRATION.md](UI_INTEGRATION.md), [VFX_INTEGRATION.md](VFX_INTEGRATION.md) | A07–A11 and corresponding 3D extensions; historical integration designs |
| [EDITOR_DESIGN.md](EDITOR_DESIGN.md), [ARCHITECTURE.md](ARCHITECTURE.md), [PROJECT_FORMAT.md](PROJECT_FORMAT.md), [DECISIONS.md](DECISIONS.md) | A02/A04, B01/B04, C01; design/format/ADR references remain maintained |
| [PHYSICS_2D.md](PHYSICS_2D.md), [VISUAL_AUTHORING_2D.md](VISUAL_AUTHORING_2D.md), [TILEMAPS.md](TILEMAPS.md), [SPRITE_MATERIALS_2D.md](SPRITE_MATERIALS_2D.md) | A05–A08/A11 and existing tested contracts |
| [AUDIO_AUTHORING_2D.md](AUDIO_AUTHORING_2D.md), [UI_AUTHORING_2D.md](UI_AUTHORING_2D.md), [GAME_SAVING_2D.md](GAME_SAVING_2D.md), [SDF_TEXT.md](SDF_TEXT.md) | A09–A10/C01; distinguish font library capability from saved UI integration |
| [NATIVE_EXPORT_2D.md](NATIVE_EXPORT_2D.md), [BROWSER_EXPORT_2D.md](BROWSER_EXPORT_2D.md), [BROWSER_PHYSICS_2D.md](BROWSER_PHYSICS_2D.md) | A08/A14 and actual runtime capabilities |
| [PORTABLE_EDITOR.md](PORTABLE_EDITOR.md), [WINDOWS_PORTABLE.md](WINDOWS_PORTABLE.md), [BUILD_PROVENANCE.md](BUILD_PROVENANCE.md), [PACKAGING.md](PACKAGING.md) | A14/C05, D01/D05; packaging instructions do not establish signatures or support |
| [ANDROID.md](ANDROID.md), [IOS.md](IOS.md) | D02–D03; instructions/scaffolding do not establish device acceptance |
| [CLEAN_MACHINE_TEST.md](CLEAN_MACHINE_TEST.md), [PHYSICAL_DEVICE_TEST.md](PHYSICAL_DEVICE_TEST.md), [WEB_EXPORT_TESTING.md](WEB_EXPORT_TESTING.md), [BUDGETS.md](BUDGETS.md) | A13–A15, B09, C03–C06, D04; actual target evidence required |
| [GETTING_STARTED.md](GETTING_STARTED.md), [TUTORIAL.md](TUTORIAL.md), [API_REFERENCE.md](API_REFERENCE.md), [KILNSCRIPT.md](KILNSCRIPT.md), [ENGINE_OVERVIEW.md](ENGINE_OVERVIEW.md), [SUMMARY.md](SUMMARY.md), [STATE.md](STATE.md) | A04/A13, C04/C06; reconcile docs/navigation/handoff with current supported behavior |

## 11. Immediate next session

Start **A01**: retrieve the failed current-source job details, reproduce the first
failure, fix it without weakening checks, and extend reference-game workflow
coverage to master/PRs. Record source-pinned results, then proceed to A02.
This consolidation itself implements no new engine feature and closes no gate.
