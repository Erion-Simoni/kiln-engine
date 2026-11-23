# Kiln Session Handoff

Branch fix/a01-ci-baseline. Current checkpoint: A04 shared callable bodies and real
imported frames (2026-11-23), following de8c42d. User requests fixes/checks executable
by the agent; do not block on external hardware/platform certification. A03 remains
explicitly deferred. A02/A03/A04 stay PARTIAL; A04 is active. A01 accepted baseline
6de428b unchanged; full 3D behind G2D. MIT OR Apache-2.0; no package-size ceiling.

## Done

- ADR-0099 closes function body/local multiplication: each reachable function
  compiles once using IR Call/Return. Nested/repeated calls, left-to-right arguments,
  lazy branches and default/early returns preserve semantics. Recursion rejects;
  cached transitive call depth and raw VM call stack both cap at 32.
- Actual live frames show entry/call-site/current-function names and imported
  locations. Allocated locals show function ownership. Missing Call source mapping
  found by a new test was fixed. Metadata stores one name per body, not per opcode.
- Runtime contract v6. Function-using VM memory adds shared-calls-v1 ABI signature;
  incompatible older function layouts reject before load commit. Straight-line
  signatures remain compatible; files are preserved. No dependency added.
- Matching editor/CLI/player and prepared browser runtime rebuilt. Default
  target/debug/web-runtime updated; prior copy retained in target/a04-web-runtime-prior-*.
- Source-only samples/function_calls plus native/browser CI verify real repeated,
  transitive and nested function results through relocated exports. Shared pixel
  observers accept explicit scenarios without an atlas fixture, retaining default
  behavior. Optional widget replay verifies actual imported frames/watch/Restart.
- Cumulative feature/history report is docs/PLAN_PROGRESS.md. Prior work includes
  shared tick budgets, actual VM locals/data watches, safe Restart, source/graph
  tooling; A03 atlas/identity/move/prefab work remains recorded and deferred.

## Executed validation

- Full locked Rust workspace: 847 PASS, zero failures including docs. just check
  PASS. Default/opt-out WASM strict Clippy PASS. Python helper suite 49 PASS;
  py_compile, Node syntax, diff checks PASS.
- Before controls fail for per-call parameter duplication and valid reuse-heavy
  compilation. Compiler/VM/runtime/save tests pass for real calls, cached depth,
  recursion, preserved semantics, imported frames/locals and ABI compatibility.
  Existing loop/graph/save/watch/scheduler/shared-budget coverage also passes.
- Actual Linux editor widgets PASS: imported function budget error, real call
  frames, live watch 9, corrected saved-source Restart/Pause, watch 7, exact manifest.
- Actual relocated native and Chromium exports PASS: 4096 green pixels each from
  imported/nested results; source copies hidden. Native player PATH has no developer
  tools; browser has no page errors. Screenshots inspected.
- Raw target/a04-functions-{before,compiler-final,runtime-final,workspace-complete,
  quality-complete,wasm-complete,wasm-optout-complete,python}.log;
  target/a04-function-frames-widgets-final/; target/a04-functions-{native,browser}/.
  Detailed evidence: docs/evidence/A04_SHARED_CALLS_AND_FRAMES.md.
- Hosted checks for new source remain pending; CI includes new native/browser
  replay steps. Existing local actual tool/runtime builds are not release bundles.

## Next / scope

- Continue feasible A04 software work: source breakpoints, statement-level scheduler
  continuation, lexical-block scope/expression watches, and the required equivalent
  enemy/projectile/trigger script/graph game with full export/debug acceptance.
  Actual imported call frames now exist. Allocated slots still outlive lexical
  blocks; functions cannot recurse or capture caller locals. Tasks are explicit
  state machines, not suspended function stacks.
- A03 deferred: scoped nested/variant identities, arbitrary component overrides,
  general multi-file crash transactions and combined production acceptance.
- A02 combined every-inspector-property widget coverage remains open. External
  cross-platform/device certification is outside the user's current request.
- Script budgets count bytecode, not host/render/physics wall time, and do not
  roll back prior completed callbacks. Current default is 1M shared instructions
  per tick with 1M per invocation; explicit Stop gets bounded cleanup allowance.

## Tools

Source /workspace/.kiln-tools/activate.sh (Rust 1.99.1/rootless libraries/Chromium).
Run timed GUI/player/browser replays serially without build/export CPU contention.
Optional imported-frame widgets require existing tesseract and normal native tools.
Generated target artifacts stay out of Git; do not kill unrelated editor/Xvfb.
just check before every commit. Primary plan: docs/ENGINE_COMPLETION_PLAN.md.
