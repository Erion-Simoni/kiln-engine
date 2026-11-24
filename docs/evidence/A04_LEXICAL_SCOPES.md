# A04 active lexical local inspection

2026-11-24, branch fix/a01-ci-baseline, following 026a19d. A04 remains PARTIAL;
A03 remains deferred. The blocked exact-physics-continuation check was not retried.

ADR-0103 adds per-slot initialized lexical instruction ranges to compiler metadata.
Parameter/local initialization opens a range after SetLocal; block/loop/function
exit closes it. Current function ownership excludes stale caller/other-function
values. Control-flow jumps naturally leave ranges; completed VMs have no active
bindings. Instruction layout, slot allocation, VM-memory signatures and runtime ABI
remain unchanged. No dependency was added.

The live debugger's Active lexical locals only checkbox filters before the 128-local
preview bound. active:NAME and expr:scope("NAME") resolve strict current-frame
bindings; loop shadowing chooses the innermost binding. Allocated inspection and
ordinary expression names remain available for retained post-completion values.
This is current-frame scope inspection, not caller-frame selection or source stepping.

Executed checks:

- Full locked Rust workspace: 863 tests PASS. just check PASS. Default/opt-out WASM
  strict Clippy PASS. Python helper suite 49 PASS; py_compile and whitespace checks.
- Current function parameters/locals exclude caller, expired-block and uninitialized
  declarations. Loop shadowing, continue-to-increment scope exit, completed scopes
  and explicit allocated compatibility PASS. Active slots beyond 160 inactive
  allocations still appear because filtering precedes the preview limit.
- Actual Linux editor widgets PASS on the first run: imported function exhausts
  the shared 1000 limit, strict scope expression reads I64(19), active-only view
  shows inner::value and inner::debug_counter, actual caller/current frames remain
  visible. Corrected saved-source Restart/Pause shows allocated expression I64(15)
  while the strict scope expression reports no active lexical binding. Manifest
  bytes remain unchanged. Both screenshots inspected.
- Optional replay: KILN_DEBUG_ACTIVE_SCOPES=1 KILN_DEBUG_FUNCTION_FRAMES=1 plus
  tests/native/shared_script_budget_smoke.py. Existing modes remain available.

Raw logs: target/a04-scopes-{initial,focused,workspace,quality,wasm,wasm-optout,
editor-build,widgets,python,precommit}.log. Actual receipt, OCR, screenshots and
editor log: target/a04-scopes-widgets/.

Editor rebuilt. CLI/player/prepared browser remain the prior v6 runtime checkpoint;
no new browser pixel replay or hosted CI result is claimed for this debugger view.
Source breakpoints and scheduler statement continuation remain open, along with
full equivalent-game production/debugger acceptance and previously skipped physics
continuation. Follow the user's bounded-attempt policy for future blocked steps.
