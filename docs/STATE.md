# Kiln Session Handoff

Branch fix/a01-ci-baseline. Current checkpoint: A04 bounded live expression
watches (2026-11-24), following 74db617. User directs at most three focused attempts
per blocked step/check, retain failure evidence and continue independent feasible
work. Skipped acceptance stays incomplete. A03 remains deferred. A02/A03/A04 are
PARTIAL; A04 active. A01 accepted baseline 6de428b unchanged. Full 3D behind G2D.
MIT OR Apache-2.0.

## Done

- ADR-0102: actual live watch field accepts expr: arithmetic, comparisons and pure
  container reads over allocated locals, explicit slot(index), local(name),
  instance state(key) and shared game(key), including distinct entity owners.
- Parse/validate a single expression; rewrite bare names to local reads; compile
  and execute with normal compiler/VM semantics in an isolated read-only host.
  Pure array_length/array_get/dict_get/dict_has are shared with gameplay. No actual
  gameplay host callback can be invoked by this evaluator.
- Mutations, statements, user functions, imports, loops and I/O reject, even in
  skipped branches. Legitimate missing reads stay lazy under short-circuiting.
  Bounds: selector 512 bytes, AST 128 nodes/depth 32, 4096 evaluation instructions,
  copied values 4096 nodes/depth 16/64 KiB including keys/overhead. Existing direct
  data selectors remain available for bounded previews of larger values.
- Watch UI retains existing fields and session-only watch storage. Added help and
  optional expression mode to the existing imported-frame/Restart widget replay.
  Rebuilt editor. Runtime ABI remains v6; CLI/player/prepared browser bundle remain
  the prior 74db617 checkpoint. No new dependency or save-format change.
- Prior A04 work includes script/graph projectile range and actual exports,
  same-tick despawn lifecycle fix, bounded graph generation/v1 migration, shared
  callable bodies, imported frames/locals, shared tick budgets and safe Restart.
  See docs/PLAN_PROGRESS.md and evidence/A04_EXPRESSION_WATCHES.md.

## Executed validation

- Full locked Rust workspace: 859 tests PASS. just check PASS. Default and opt-out
  WASM strict Clippy PASS. Python helper suite 49 PASS; py_compile/whitespace PASS.
- Precision, arithmetic, comparison, nested state, shared game data, owner/slot
  selection, ambiguity, lazy missing reads, mutation/statement/type/arity/overflow
  rejection and AST/value limits PASS. Gameplay snapshots, VM instruction position
  and shared gameplay budget usage remain exactly unchanged after evaluation.
- Actual Linux editor widgets PASS on first run: imported failure/shared budget,
  computed watch I64(19), actual outer/inner frames, corrected source Restart,
  Pause, computed watch I64(15), manifest unchanged. Screenshots inspected.
- Raw logs target/a04-expression-{focused,final-focused,workspace,quality,wasm,
  wasm-optout,editor-build,widgets,python,precommit}.log. Actual widget receipt,
  OCR and screenshots: target/a04-expression-widgets/. Hosted CI and new browser
  pixel replay for this editor-only feature are not claimed.

## Blocked / skipped

- Exact physics continuation remains blocked; do NOT retry to delay plan progress.
  Prior pre-impact restore diverged at tick 46 (x=140.00006 versus 138.33339).
  Evidence: target/a04-range-exact-physics-continuation-blocked.log. Restored final
  victory outcomes pass, with explicit narrower scope. No general physics-save
  determinism fix is claimed. Existing script/graph exports passed in 74db617.
- External cross-platform/hardware/device certification is outside this request.

## Next

- Continue feasible A04 source breakpoints and scheduler statement continuation;
  apply the user's bounded-attempt skip policy. Read-only expression watches now
  exist, but names still refer to allocated slots and reject ambiguity. Active
  lexical scopes remain open. Imported actual function frames already work.
- Equivalent enemy/projectile/trigger game exists; full editor-authoring/debugger
  production acceptance remains open. Advance past genuinely blocked steps with
  evidence, keeping incomplete milestone acceptance marked PARTIAL. A05 has not
  started in this checkpoint.
- A03 deferred: nested/variant identities, arbitrary component overrides, general
  multi-file crash transactions and combined production acceptance. A02 combined
  every-inspector-property widget acceptance remains open.
