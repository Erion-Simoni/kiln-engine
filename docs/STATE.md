# Kiln Session Handoff

**Active focus:** Task 2 of `docs/2D_COMPLETION_PLAN.md`, IN PROGRESS.
**Branch:** `feat/2d-production-workflow`.
**Scope:** Continue the user's ordered 2D completion program. 3D remains paused.

## Done

- Earlier checkpoints cover native/WASM baseline repair, project lifecycle and atomic writes, authored metadata preservation, multi-scene tabs/history, project Save/Discard/Cancel, transform/property commands, component presence commands, and hierarchy entity commands. ADR-0039–0047 record the architecture.
- Hierarchy create/delete/duplicate/reparent are document-first commands with ID validation, cycle prevention, metadata preservation, and undo. Native GUI verified create/selection, duplicate/selection, delete/undo, reparent/save, and saved undo/redo of Parent.
- Inspector edits for Transform position/rotation/uniform-scale and Sprite tint/layer/dimensions update authored and runtime state with one undo step per field gesture. Sprite color/layer/size validation fails atomically.
- Sprite dimensions are authored in world units and default to 32×32 when absent in older project files. Runtime scene loading and editor snapshots retain dimensions. GPU tests verify sprite shader rotation and scaling.

## Validation

- Latest renderer checkpoint: `just check` passed; `just test`: 506 passed; focused GPU renderer test, editor build, WASM target check, and Chromium browser smoke passed.
- Latest Sprite dimensions checkpoint: `just check` passed; `just test`: 507 passed, zero failed/skipped. Legacy 32×32 deserialization, project round-trip, Sprite property transaction, save/reopen, and invalid-value checks passed. Editor build, WASM check, and Chromium browser smoke also passed.
- Native editor GUI has previously verified Inspector transforms/properties and hierarchy edits in Xvfb/software GL/Low tier. Native visual confirmation of the latest transformed sprite render remains outstanding. Physical devices, other render tiers, cross-platform runners, and remote CI are unverified.

## Next

1. Implement parent/local transform propagation consistently in runtime and editor projection, with nested/cycle/undo tests.
2. Add rotation/scale viewport gizmos and multi-selection/framing.
3. Add dirty scene-tab Save/Discard/Cancel and autosave/recovery.
4. Run full two-scene transform/reparent/duplicate/delete/save/restart acceptance before completing Task 2. Tasks 3–9 remain pending.

## Known limits and environment

Shared sprite submissions carry entity rotation and scale across editor, player, browser, mobile, and sample paths. Tilemap cell instances retain identity transforms. Parent links currently serialize and hierarchy edits work, but runtime/editor transform propagation through parents is not implemented. The command model now covers transform properties, Sprite visuals/size, component presence, and hierarchy entities; other panels still have transitional direct-mutation paths. Loader-created default Transforms remain projection-only until authored. Undo is session-only; crash recovery is absent. Native visuals cover software GL/Low tier only.

Save As stays within the asset directory; relocation is rejected. Dialogs accept typed paths. Platformer/Top-down templates remain empty starters; only Empty 2D has been exercised. Other render tiers, audible audio, complete exports, budgets, clean-machine packaging, cross-platform runs, physical devices, and remote CI remain separate gates. Optional audio/WASM warnings remain outside strict default-native lint. Plugin ABI v2 requires creator-matched destruction; loader/dispatch remains future work.

Source `/workspace/.kiln-tools/activate.sh` for cloud Cargo commands.
Use `CHROMIUM_PATH=/usr/bin/chromium just web-smoke` for installed-browser validation.
