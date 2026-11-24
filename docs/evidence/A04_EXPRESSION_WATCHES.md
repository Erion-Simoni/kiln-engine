# A04 bounded read-only expression watches

2026-11-24, branch fix/a01-ci-baseline, following 74db617. A04 stays PARTIAL and
A03 deferred. The previously blocked exact-physics-continuation check was not
retried. This checkpoint implements the next independent debugger step.

ADR-0102 adds expr: selectors to the actual live watch field. A single parsed
return expression is validated before compiling. Bare identifiers become named
allocated-local reads; explicit slot(index), local(name), state(key), game(key)
read the selected gameplay VM or shared game state. Read-only array_length,
array_get, dict_get and dict_has share their gameplay implementations. The normal
compiler/VM supplies arithmetic, comparison, integer precision and short-circuit
semantics. The isolated host never calls the gameplay host.

Mutations, user functions, statements, imports, loops and I/O reject, including
forbidden calls in skipped branches. Missing read-only values in skipped branches
remain skipped. Bounds: full selector 512 bytes, AST 128 nodes/depth 32, evaluation
4096 instructions; cumulative copied values 4096 nodes/depth 16/64 KiB including
keys and overhead. Direct selectors still provide bounded previews for larger
values. Allocated locals are not active lexical scopes; ambiguous names reject.

Executed validation:

- Full locked Rust workspace: 859 tests PASS. just check PASS. Default and opt-out
  WASM strict Clippy PASS. Python helper suite 49 PASS; py_compile and diff checks.
- Tests exercise arithmetic/precision, comparisons, nested instance data, shared
  game data from distinct entity owners, explicit slots, ambiguity, lazy reads,
  mutation/statement/arity/type/overflow rejection, AST bounds and deep/broad/large
  value copy guards. Gameplay snapshots, VM instruction position and shared
  gameplay instruction usage remain exactly unchanged after watch evaluation.
- Actual Linux editor widget replay PASS on its first run: imported inner function
  exhausts the shared 1000 budget; expr:debug_counter*2+1 shows I64(19), actual
  outer/inner frames are inspected, corrected saved source restarts and pauses,
  and the same watch shows I64(15). Manifest bytes remain unchanged. Screenshots
  inspected. Optional replay: KILN_DEBUG_EXPRESSION_WATCHES=1 together with
  KILN_DEBUG_FUNCTION_FRAMES=1 and shared_script_budget_smoke.py.

Raw evidence: target/a04-expression-{focused,final-focused,workspace,quality,wasm,
wasm-optout,editor-build,widgets,python,precommit}.log and
 target/a04-expression-widgets/ (receipt, OCR, screenshots, editor log).

No new dependency, language/runtime ABI or save-format change. Editor rebuilt;
CLI/player/prepared browser bundle remain the prior v6 checkpoint. Browser pixel
replay for this editor-only watch surface is not claimed. Source breakpoints,
scheduler statement continuation and active lexical scopes remain open. Existing
script/graph projectile range and export evidence remain in their own checkpoint.
