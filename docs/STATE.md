# Kiln Session Handoff

**Active focus:** Owner-requested consolidation of all planning Markdown into docs/ENGINE_COMPLETION_PLAN.md, covering remaining 2D, integrated 3D, production release, platforms and optional extensions. Planning only; no engine feature started.
**Branch:** docs/unified-engine-completion-plan, based on master ae492f1. The latest master and prior local source have identical engine trees. Prior review observations are retained below.
**Owner preferences:** no release-size goal; performance/resource bounds remain. MIT OR Apache-2.0 is settled. Full 3D implementation stays gated on G2D.

## Consolidated plan

- One authoritative plan now owns milestone IDs, dependencies, statuses, acceptance criteria, evidence rules and G2D/G3D/GPROD/GPLATFORM gates.
- Preserved M0–M61, nine 2D tasks, seventeen 2D stages, editor plans, M29 designs and specialist/platform requirements through an explicit source crosswalk. Older planning documents carry historical-reference notices; README, documentation navigation and agent entry points point to the unified plan.
- Added gaps for draft recovery, 3D conventions/import identity/migrations, device loss, collision/camera/animation ownership, world-space UI, spatial audio, resource unloading, QA/security/compatibility and release maintenance.
- Mobile/macOS/handheld work remains a full-platform gate; newer Task 9 deferral ordering takes priority over older version-number coupling. Console/XR, multiplayer, plugins/mods/stores and advanced rendering research are explicitly retained as extensions with acceptance, not silently discarded.
- Next implementation: A01, diagnose hosted failures and promote reference-game acceptance to master/PRs. New planning checkboxes do not claim implementation or acceptance.

## Validation for this documentation change

- `just check` passed: formatting, strict workspace/all-target clippy, workspace compilation and the standalone audio sample check.
- Documentation links, milestone coverage and whitespace were checked before committing. No gameplay tests were rerun for this documentation-only change; earlier gameplay evidence remains separately identified below.

## Done / current strengths

- Latest master includes the three editor-authored reference games, the reconciled shared Script Editor, Quick Actions, editor polish, movement starter and new KilnScript control flow.
- Supported scene/hierarchy commands, isolated undo, tabs, prefab/asset workflows, Rapier physics/cameras, sprite-frame animation, tilemaps, particles, audio, saved UI, general saves/settings and host-native/browser exports are implemented to their documented contracts.
- Existing local evidence for the identical source tree records just check, 700 passing Rust tests (zero skipped), 29 Python tests, native/WASM builds, three actual editor authoring replays, six exports, and native/browser victories and restoration. This assessment reused that evidence; it did not rerun those tests.
- Read-only parallel audits checked editor/runtime/shipping code, beyond historical completion checkboxes. A useful small-game engine is present. General production readiness and Unity/Godot feature breadth are not yet established.

## Fresh hosted observations

Public GitHub pages fetched in this review identify master ae492f1:

- Windows portable candidate 43405013305: SUCCESS.
- General CI 43405013257: FAILURE.
- Release candidate verification 43405013163: FAILURE.
- Reference-game workflow 43404644543 for merged source f7c8f9d: FAILURE.

These supersede the old pending checkpoints. Exact failed steps/causes were not retrieved. Further job-detail commands were blocked because automatic approval review hit its usage limit; this was not an unsafe-action determination. Existing fetched pages establish overall conclusions only. No bypass or further blocked network retries were attempted.

## Next: practical 2D release blockers

1. Diagnose and fix the failed hosted gates, preserving assertions. Promote reference-games.yml from its feature-branch-only push trigger to master/pull-request coverage. Do not equate local passes with current hosted acceptance.
2. Complete remaining actual GUI acceptance across hierarchy/gizmos, prefab overrides/propagation, asset rename/reimport, multilayer terrain/colliders, particles, recovery and save/restart. Current reference games cover selected paths; old no-Xvfb statements need reconciliation against newer evidence.
3. Recover unsaved gameplay source after crashes: recovery.rs serializes Project, while the Script Editor buffer remains separate. Test forced close, failed save, undo and reload on representative projects.
4. Measure CPU/script/physics/GPU time, memory and startup on real target hardware, including i7-6600U/Intel HD 520 if that is the minimum. Stress long sessions/large projects. SpriteRenderer currently silently returns at its 16,384 queued-quad limit; expose overflow and validate scaling/culling.
5. Finish full Windows GUI author/save/restart/export/play acceptance, physical audio/controller/GPU tests, other claimed browsers and clean target systems. Windows candidate success is narrower. Current Linux packages require GLIBC 2.39/system libraries.
6. Have an independent developer complete the tutorial unaided, then reconcile the public support matrix and release documentation. Finish reviewed distribution/signing/provenance requirements. Export currently supports the host OS and web; cross-OS/mobile export is explicitly rejected. macOS/other platforms require their own validation before being advertised.

## Feature breadth beyond the current small-game contract

- Source editing/debugging: completion/search/navigation and source breakpoints, statement stepping, stack/locals/watch inspection. Current live controls really pause/step/restart gameplay frames and inspect instance state.
- Animation: visual state graphs, property tracks/timeline, transitions/blending; skeletal animation later if needed. Current authored clips are spritesheet frames/events/next-clip playback.
- Saved game UI: sliders, toggles, text entry, progress bars, scrolling/containers and nine-slice styles; richer text/shaping as needed. Current UiKind is Panel/Label/Button.
- Authorable polygon/capsule collider shapes and editing; current saved collider types are rectangle/circle.
- Integrated authored 2D lights, shadows/occluders and normal-map workflow. Custom sprite materials already work; an isolated lighting module is not proof of this workflow.
- Plugins/ecosystem, navigation tools, mobile breadth and advanced 3D are later scope, not prerequisites for every small 2D game. Unity/Godot are stronger pure-2D benchmarks; Unreal offers mature general tooling and Paper2D but is more oriented toward 3D/2.5D.

## Boundaries

Tasks 8 and 9 remain open. Earlier implementation-complete task labels do not imply mature-engine parity or blanket GUI acceptance. No quantitative parity percentage, performance advantage or physical-device pass is claimed. Xvfb/SwiftShader/ALSA-null/Chromium evidence remains software evidence. This session changes documentation only on docs/unified-engine-completion-plan. Engine code and remote branches were not modified. The handoff was rewritten to make the unified plan authoritative; A01 is the next implementation milestone.
