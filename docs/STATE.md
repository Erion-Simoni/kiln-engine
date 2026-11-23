# Kiln Session Handoff

Branch fix/a01-ci-baseline. Current checkpoint: A04 bounded shared graph
compilation (2026-11-23), following 96989fa. User requests substantial fixes/checks
that the agent can execute; skip unavailable external certification. A03 remains
explicitly deferred. A02/A03/A04 stay PARTIAL; A04 is active. A01 accepted baseline
6de428b unchanged. Full 3D remains behind G2D. MIT OR Apache-2.0.

## Done

- ADR-0100 replaces exponential graph source duplication with one lazy case per
  reachable node and balanced dispatch through ordinary KilnScript. Per-invocation
  computed/value/event locals retain shared-input memoization, host call order,
  false-trigger and conditional-branch laziness, and live inspection-state writes.
- Static DAG validation memoizes heights and retains depth 64, cycle, signature,
  pin and connection checks. Existing 256-node/2048-connection, 256 KiB source and
  1 MiB document bounds remain. No recursive calls or new dependency are needed.
- Visual document v2 uses compact generated bodies. Frozen unchanged v1 lowering
  verifies legacy bodies; opening preserves source, identity and positions.
  Explicit visual save upgrades. Edited bodies reject visual overwrite. Legacy
  invalid drafts, including previously oversized shared DAGs, remain recoverable.
- Actual editor replay exposed a v1-only scene-assignment check; shared supported
  header recognition now accepts both versions. Editor rebuilt against the fix.
- Prior checkpoints retain shared callable bodies, actual imported frames/locals,
  nested watches, shared tick budgets and staged safe Restart. Runtime contract
  stays v6: the new graph backend uses existing language/VM constructs.
- Cumulative implemented and open work: docs/PLAN_PROGRESS.md. Details and raw
  evidence paths: docs/evidence/A04_SHARED_GRAPH_DEPENDENCIES.md.

## Executed validation

- Locked full Rust workspace: 852 tests PASS. just check PASS. Default and opt-out
  WASM strict Clippy PASS. Python helper suite: 49 PASS. Diff whitespace PASS.
- Before control: valid 22-node shared-input graph exceeds old source bound.
  Fixed regression compiles below 32 KiB/16000 instructions, executes twice to
  1048576, and writes each node exactly once per invocation. Depth 64 executes;
  depth 65 rejects. Legacy migration and edited-source guards PASS.
- Existing host order, lazy branch/trigger, cycle, invalid-pin, value/string,
  save/load, debugger, scheduler and aggregate-budget tests PASS.
- Actual graph widget/browser replay PASS with rebuilt editor and matching v6
  browser runtime: connections, Cancel/history, Play/node state, normal close,
  exact save/reopen and Chromium save restoration. Passing receipt is under
  target/a04-graph-dag-widgets-replay/. Earlier close-timeout evidence retained.
- Local evidence lives under target/a04-graph-dag-*; generated artifacts remain
  untracked. Hosted CI for this source and release certification are not claimed.

## Next / known issues

- Continue A04 source breakpoints, statement-level continuation, active lexical
  block scopes/expression watches and the required equivalent script/graph
  enemy/projectile/trigger game with export/debug acceptance. Real imported
  function frames already exist. Tasks remain explicit staged state machines.
- A03 deferred: nested/variant identities, arbitrary component overrides,
  general multi-file crash transactions and combined production acceptance.
- A02 combined every-inspector-property widget acceptance remains open.
- Script instruction budgets bound bytecode rather than host/render/physics wall
  time and do not roll back completed callbacks. Inactive graph inspection values
  remain last-executed results. Legacy verification intentionally retains the old
  bounded emitter solely to guard old document bodies.
- External cross-platform/hardware/device checks are outside this request.
