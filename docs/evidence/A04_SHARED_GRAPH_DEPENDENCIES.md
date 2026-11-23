# A04 shared graph dependencies

2026-11-23, branch fix/a01-ci-baseline, following 96989fa. A04 remains active;
A03 remains deferred. Scope is locally executable software fixes and checks.

ADR-0100 replaces recursive dependency source expansion with one generated case
per reachable node, selected through a balanced dispatch tree. Ordinary source
locals memoize computed values/events per invocation. Demand returns to Output
after each completed node; false triggers and unselected branches remain lazy.
No recursive VM calls, new dependencies or runtime contract change are needed.
Existing node, connection, depth, source and document bounds remain enforced.

The before control fails for a valid 22-node graph with 20 doubled shared inputs:
its generated source exceeds 256 KiB. The fixed regression requires source below
32 KiB and IR below 16000 instructions, executes twice with result 1048576, and
counts exactly one inspection-state write per node per invocation. A depth-64
shared graph executes, while depth 65 rejects. Existing host fanout, side-effect
ordering, branch laziness, string, cycle and invalid-pin tests remain passing.

New visual documents use v2. Frozen v1 lowering is unchanged from the previous
commit and verifies canonical old bodies before exposing the graph model. Opening
does not rewrite source; explicit visual save upgrades to v2. Tests preserve model
identity/positions, recover invalid drafts, reject edited bodies in both versions,
and migrate the previously oversized shared-input draft into executable source.
The first actual editor replay caught a hardcoded v1-only scene-assignment check;
the editor now uses the shared supported-header predicate.

Validation and raw evidence:

- Full locked Rust workspace: 852 tests PASS. just check and default/opt-out
  WASM strict Clippy PASS.
- Python helper suite: 49 passed.
- Actual Linux graph widgets and Chromium graph save/restore replay PASS, using the
  rebuilt editor and existing matching v6 browser runtime. Covers connections,
  real add_score arguments, Cancel, Undo/Redo, grouped drag, assignment, Play,
  live reserved node values, save/reopen equality and browser save restoration.
  Native saved score 261 (multiple of 9); browser restored score 387 and three
  actual node-state values. Reopened graph screenshot inspected.
- Logs: target/a04-graph-dag-{before,final-focused,workspace-final,quality-final,
  wasm,wasm-optout,python,editor-build-final,widgets-final}.log.
  Final passing widget screenshots and receipt: target/a04-graph-dag-widgets-replay/
  and target/a04-graph-dag-widgets-replay.log. The prior corrected run reached
  gameplay but timed out during normal editor close; retained under widgets-final.
  Its isolated unchanged replay passed, including normal close and reopen.

This improves graph compilation and migration; it does not complete source
breakpoints, statement continuation, lexical-block expression watches or the
required equivalent enemy/projectile/trigger game. Hosted CI and external hardware
certification are not claimed.
