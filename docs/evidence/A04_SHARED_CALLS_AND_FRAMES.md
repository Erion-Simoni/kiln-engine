# A04 shared callable bodies and actual imported frames

2026-11-23, fix/a01-ci-baseline, following de8c42d. Scope is agent-executable
software fixes/checks, as requested; A03 remains deferred and A04 active.

## Fix

ADR-0099 replaces function inlining at every call with one reachable body and the
existing IR Call/Return instructions. The before controls fail: two calls allocate
two copies of one parameter, and a valid 400-call reuse program hits the compiler's
expansion guard. The fixed compiler allocates 101 total slots for the 100-local
helper and compiles the entire program below 3000 instructions, then executes it.
Repeated/nested calls, left-to-right side effects, early/default returns, skipped
branches and caller-loop isolation retain their behavior. Recursion remains rejected;
transitive depth includes cached callees, and raw bytecode cannot exceed 32 frames.

Function instruction ranges and slot ownership are metadata, with one function name
per body rather than per instruction. Actual live snapshots show caller/current
frames and imported source locations, plus function-owned allocated locals. An
initial nested-frame test found missing call-site source mapping; the Call opcode
now records its expression span explicitly. Block-local lifetime/statement-level
continuation is not claimed. Function-call results still use the ordinary compiler,
host registry and VM on native and web.

Runtime identity advances to kiln-authoring-v6. Function-using saved VM memory has
a shared-calls-v1 signature; incompatible prior function slot layouts reject before
commit even when slot counts match. Straight-line programs keep their legacy
signatures. Save files remain untouched. Matching CLI/player/editor/browser tools
were rebuilt, and the default development web runtime updated with its previous
copy preserved under target/a04-web-runtime-prior-*.

## Executed checks

- Locked workspace: 847 Rust tests passed. just check and default/opt-out WASM
  strict Clippy pass. Python helpers 49 pass; py_compile, Node syntax and diff
  checks pass.
- Before controls: target/a04-functions-before.log. Compiler tests cover compact
  repeated bodies, a 400-call program, nested reuse, argument side effects, lazy
  branches, default/early returns and cached transitive depth. Raw IR recursion
  stops at 32; imported failure exposes entry → outer → inner with correct source
  positions and owned parameter/local values. VM-memory tests reject old function
  ABI while preserving straight-line signatures. Existing loop, save, graph,
  scheduler, budget and watch suites pass. The ambiguity fixture now uses two
  distinct functions with the same parameter name, matching the shared-body model.
- Actual editor widgets pass: imported nested functions exhaust the shared 1000
  limit, display real imported frames, watch I64(9), correct saved source, Restart,
  Pause and watch I64(7), with unchanged manifest bytes. Screenshots inspected.
  Optional replay: KILN_DEBUG_FUNCTION_FRAMES=1 plus shared_script_budget_smoke.py.
- samples/function_calls proves repeated/transitive/nested call results in a plain
  project: evaluate(2)=7 and bump(bump(0))=2 turn a red square green. Actual relocated
  native export passes with 4096 green pixels, source copy hidden and player PATH
  without developer tools. Actual relocated Chromium export also passes with 4096
  green pixels under its nested deployment path, source copy hidden and no page
  errors. Both screenshots inspected. Existing green-pixel observers now accept an
  explicit project/scenario without requiring a prior atlas receipt; default atlas
  behavior/receipt fields stay unchanged.
- Native/browser CI now runs that checked-in function sample and uploads focused
  logs/screenshots/receipts. Hosted results for this new commit are not claimed.

Raw: target/a04-functions-{compiler-final,runtime-final,workspace-complete,
quality-complete,wasm-complete,wasm-optout-complete,python}.log;
target/a04-function-frames-widgets-final/ and sibling log;
target/a04-functions-native/ and target/a04-functions-browser/ with sibling play-final logs;
target/a04-functions-*-build*.log and web-prepare logs. These are Linux software
rendering and Chromium checks available in this environment. No external device
approval or certification is required for the completed work.
