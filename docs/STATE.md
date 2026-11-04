# Kiln Session Handoff

**Active focus:** Task 4 implementation complete; native graphical acceptance pending.
**Branch:** `feat/2d-production-workflow`. 3D remains paused. Task 5 has not started.

## Delivered

- Authored `Component::Script` stores a project-relative resource and numeric exposed properties. Inspector attach/edit/remove commands are validated, undoable, and preserve metadata through projection/save/reopen.
- Editor/player use `SliceSession::with_scene_sources`, including nested/attachment-only scenes. Each stable entity ID receives an isolated VM state store and `self_entity()` reference. Lifecycle phases cover start/update, deferred signals, backend collision enter/exit, and stop. World systems run once per tick; callbacks reuse tick/frame context.
- KilnScript supports named non-recursive functions, relative module imports, bounded arrays, per-instance state APIs, and scene-transition requests. Existing scene programs retain per-frame top-level execution. See KILNSCRIPT.md for syntax, limits, and reserved-name migration.
- Program source offsets and expanded-module origins produce compiler/runtime file/line/column errors. Live Gameplay Debugger controls actual pause/resume/frame-step/restart and inspects entity state. The previous detached instruction debugger is no longer presented as live gameplay control.
- Prior checkpoint supplies current-scene/project-start Play, shared fixed timing, focus/text-input handling, and explicit undoable runtime-transform application. Player restart/transition rebuilds the world and resets timing; stop callbacks run before teardown.

## Validation

- `just check`: passed. Full `just test`: 560 passed, zero failed/skipped. Native editor/player builds passed.
- Shared runtime acceptance compares source resources against a deserialized player project/byte assets: two enemies with isolated state/properties/arrays, projectile motion, injected backend collision, deferred trigger signal, scene request, stop callbacks, and authoring isolation.
- Focused physics-enabled acceptance and script component history/disk/projection tests passed. Import cycles, recursion, and imported-file compile/runtime errors are covered.
- No display server/Xvfb is available, so native GUI interaction remains unverified.

## Supported limits / next

- Exposed properties are numeric. Runtime state persists between ticks; serializing arbitrary script state into game saves belongs to the later save workflow.
- Live debugging is at frame granularity. Source/instruction breakpoints and arbitrary watches are not exposed by this implementation.
- Collision delivery is implemented; geometric event production depends on the physics backend and is Task 5 work. The acceptance injects backend events.
- Record native graphical acceptance before claiming visual verification. Do not mark Task 5 complete based on these scripting changes.

Source `/workspace/.kiln-tools/activate.sh` for Cargo. Git pushes require the supported `require_escalated` network permission; the prior remote tip was `68491af`.
