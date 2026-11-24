# Kiln Session Handoff

Branch fix/a01-ci-baseline. Current checkpoint: A04 source statement suspension
and continuation (2026-11-24), following 0c73b3d. User directs at most three focused
attempts per blocked step/check, retain failures and move to the next feasible plan
step. A03 remains deferred. A04 remains PARTIAL; its remaining acceptance is deferred
under this policy. NEXT WORK: independent A05 rendering/camera/large-scene fixes.
Do not retry blocked A04 gates to delay A05. A01 baseline 6de428b unchanged;
A02/A03 remain PARTIAL. Full 3D behind G2D. MIT OR Apache-2.0.

## Done

- ADR-0104: statement metadata mapped to exact imported source lines. Up to 128
  unconditional breakpoints affect the actual scene/attachment VMs. Source stops
  retain operand/local/call stacks, queued commands and remaining instruction
  allowance. Resume skips the stopped boundary once. Step Statement enters calls
  and crosses callback boundaries without a new tick or refreshed allowance.
- Resumable tick captures input/time/events plus attachment callback cursor.
  Systems advance once; completed callbacks are not replayed. Signal/animation
  callbacks reference a captured payload pool. Commands commit on callback finish;
  previously executed direct state writes remain visible, without rollback.
- Live editor source controls/status/statement stepping; staged Restart rebinds
  lines against saved sources and logs stale/unbound lines. Saved-screen UI
  input/start/event dispatch stays atomic. Breakpoints are session-only and may
  need reinstalling after scene transitions/logical loads. No step-over/out or
  caller-frame scope selection is added.
- Unfinished logical saves reject. Stop abandons pending commands and runs bounded
  cleanup with probes disabled. Final review caught/repaired resumed attachment
  frame_count reset; failing before control retained.
- Rebuilt matching editor/CLI/player/prepared browser runtime. Runtime ABI stays
  v6; previous browser bundle is target/a04-source-prior-web-runtime. No dependency.
- Prior A04 features: active lexical inspection/scoped and isolated expressions,
  shared function bodies/real imported frames, budgets, Restart, bounded graphs,
  script/graph projectile game and source/export parity. PLAN_PROGRESS.md and
  evidence/A04_SOURCE_CONTINUATION.md contain details.

## Executed validation

- Locked full Rust workspace: 870 PASS. just check PASS. Default/opt-out WASM strict
  Clippy PASS. Python helper suite 49 PASS; py_compile/whitespace PASS.
- Seven focused source tests PASS, including final frame-count guard: imported
  return/caller continuation, operand-stack reuse, callback/signal/event ordering,
  frozen input/time/frame, one physics step, no budget refill, incomplete saves,
  abandoned commands and cleanup. Existing entity/shared-budget tests also pass.
- Script and graph games each PASS relocated native and Chromium exports: 3932
  green pixels, source copy hidden, native PATH without developer tools. Receipts
  under target/a04-source-{native,browser}-projectile_range{,_graph}/.
- Raw target/a04-source-{workspace-final,quality-final,wasm-final,wasm-optout-final,
  final-focused,frame-before,python,precommit,tools-final,web-final}.log.

## Blocked / skipped gates — do not loop on these

- Source widget replay overall INCOMPLETE after three attempts. First two had OCR
  selection failures. Third observed real imported breakpoint, Restart rebinding,
  scoped value 4, caller statement/result 8 and completed pending tick; screenshots
  inspected. It failed manifest comparison because setup Save normalized TOML
  after the baseline capture. Baseline placement is corrected for future use but
  no fourth replay was performed. Do not claim overall widget/manifest PASS.
  Evidence: target/a04-source-widgets{,-replay,-final}/ and sibling logs; final
  partial-result.json records scope. Continue to A05 under the user's skip policy.
- Exact saved-physics continuation remains blocked from the prior checkpoint:
  tick 46 x=140.00006 versus 138.33339. Raw
  target/a04-range-exact-physics-continuation-blocked.log. Restored final victory
  outcomes pass with narrower scope; no general physics-save fix is claimed.
- Saved-screen UI source suspension and full combined A04 production acceptance
  remain open/deferred. External cross-platform/device certification is outside
  the request. A04 is not accepted/complete.

## Next

- Start A05: sprite rendering, cameras and large-scene behavior. Inspect its plan
  and choose substantial agent-executable fixes/checks (sorting/culling/capacity
  diagnostics/camera mapping). A03 dependencies remain deferred; choose independent
  software work and record any genuinely blocked dependency. Apply the same maximum
  three focused attempts per blocked step, retaining incomplete acceptance.
- A03: nested/variant identities, arbitrary overrides, multi-file crash transactions
  and combined acceptance deferred. A02 every-inspector-property combined widget
  acceptance remains open. Preserve these statuses; no release/3D approval implied.
