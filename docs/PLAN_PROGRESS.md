# A02–A04 implementation and acceptance progress

2026-11-23, branch `fix/a01-ci-baseline`. Authoritative gates remain in
ENGINE_COMPLETION_PLAN.md. This summarizes implemented work; it does not convert
local feature checks into milestone acceptance. A01 accepted baseline is 6de428b.

| Milestone | Implemented and checked | Remaining acceptance / decision |
| --- | --- | --- |
| A02 authoring | Scene/hierarchy composition, reference guards, copy/paste/duplicate/history, property-preserving projection, project persistence/Save As rescue, whole-project/two-scene crash recovery and component coverage | Every-inspector-property combined widget acceptance and cross-platform GUI remain open. A02 PARTIAL. |
| A03 assets/prefabs | Stable loaded IDs/presets, dependency-aware asset/source moves, source-import rewriting, art refresh, saved deterministic atlases/static clips, staging-only native/browser rebuilds, interrupted atlas recovery, prefab color/texture/geometry/material/clip overrides, reference-safe joint propagation | Nested/variant prefab identity/runtime contract, arbitrary component overrides, general multi-file crash transactions and combined prefab/multi-scene/atlas/export visual acceptance remain open. Deferred by user 2026-11-21. |
| A04 scripting/graphs/debugging | Shared host registry; functions/modules, loops/dictionaries/enums, explicit tasks/timers; source tools; editable connected gameplay graph lowering through the same frontend; staged saved-source restart; actual VM named allocated-local inspection and read-only nested data and bounded read-only expression watches; aggregate tick budgets across scene/entity/event/UI work; shared callable bodies and actual imported-function frames; bounded shared-dependency graph generation with guarded v1 migration; text/graph projectile range and same-tick despawn lifecycle correction | Live source breakpoints, statement-level continuation, active lexical-block scopes remain open. Allocated-local read-only expression watches are implemented. Equivalent enemy/projectile/trigger game is implemented; full export/debug production acceptance and exact physics-save continuation remain open. A04 PARTIAL and active. |

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
