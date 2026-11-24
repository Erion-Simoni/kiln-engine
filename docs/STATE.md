# Kiln Session Handoff

Branch fix/a01-ci-baseline. Current checkpoint: A04 active lexical inspection
(2026-11-24), following 026a19d. User directs at most three focused attempts per
blocked step/check, retain failure evidence and continue independent feasible work.
Skipped acceptance stays incomplete. A03 remains deferred. A02/A03/A04 PARTIAL;
A04 active. A01 accepted baseline 6de428b unchanged. Full 3D behind G2D.
MIT OR Apache-2.0.

## Done

- ADR-0103: compiler metadata records initialized per-slot lexical instruction
  ranges and function ownership. Bindings start after parameter/local stores,
  close on block/loop/function exit, and apply only to the current function frame.
  Break/continue/return leave the relevant ranges; completed VMs have no active
  bindings. Bytecode, local slot layout and VM-memory ABI remain unchanged.
- Active lexical locals only checkbox in actual live debugger. Filter before the
  128-local preview bound, retaining later active slots. Snapshot locals expose
  in_scope. active:NAME selectors and scope(name) expression reads resolve strict
  current-frame names, choosing the innermost shadowing loop binding.
- Allocated locals/default expression names remain compatible for post-completion
  and post-error retained values. Strict scope reads reject expired, uninitialized
  and non-current-frame bindings. Caller frames stay visible, but there is no
  caller-frame scope selection in this checkpoint.
- Editor rebuilt. Runtime contract v6 and save signatures unchanged; CLI/player/
  prepared browser bundle remain prior 74db617 runtime checkpoint. No dependency.
- Prior A04 work includes isolated read-only expressions, script/graph projectile
  range and exports, same-tick despawn fix, bounded graphs/v1 migration, shared
  callable bodies, imported frames, shared tick budgets and safe saved Restart.
  See docs/PLAN_PROGRESS.md and evidence/A04_LEXICAL_SCOPES.md.

## Executed validation

- Full locked Rust workspace: 863 tests PASS. just check PASS. Default/opt-out WASM
  strict Clippy PASS. Python helper suite 49 PASS; py_compile/whitespace PASS.
- Current-frame/parameter lifetimes, expired blocks/callers, uninitialized locals,
  loop shadowing/continue, completed scopes, allocated compatibility and active
  slots beyond 160 inactive allocations PASS. Existing watches/saves/budgets and
  graph/runtime tests remain passing.
- Actual editor widget replay PASS on first run: imported failure/shared budget,
  scoped expression I64(19), active-only inner::value and inner::debug_counter,
  actual outer/inner frames; corrected-source Restart/Pause, allocated expression
  I64(15), strict scope reports no active binding after completion. Manifest bytes
  unchanged. Screenshots inspected.
- Raw target/a04-scopes-{initial,focused,workspace,quality,wasm,wasm-optout,
  editor-build,widgets,python,precommit}.log. Actual receipt, OCR, screenshots and
  editor log: target/a04-scopes-widgets/. No new hosted/browser pixel claim.

## Blocked / skipped

- Exact physics continuation remains blocked; do NOT retry to delay plan progress.
  Prior pre-impact restore diverged at tick 46 (x=140.00006 versus 138.33339).
  Evidence: target/a04-range-exact-physics-continuation-blocked.log. Restored final
  victory outcomes pass with explicit narrower scope. Existing script/graph native
  and browser exports passed in 74db617. No general physics-save fix is claimed.
- External cross-platform/hardware/device certification is outside this request.

## Next

- Continue A04 source breakpoints and scheduler statement continuation using the
  user's bounded-attempt skip policy. Current-frame lexical inspection and strict
  lexical expression reads now exist. Whole-game frame stepping remains separate.
- Equivalent enemy/projectile/trigger game exists; full editor-authoring/debugger
  production acceptance remains open. Advance past genuinely blocked steps with
  evidence, keeping skipped milestone acceptance PARTIAL. A05 has not started.
- A03 deferred: nested/variant identities, arbitrary component overrides, general
  multi-file crash transactions and combined production acceptance. A02 combined
  every-inspector-property widget acceptance remains open.
