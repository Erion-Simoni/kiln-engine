# Kiln Session Handoff

Branch fix/a01-ci-baseline. Current checkpoint: A04 projectile range and despawn
lifecycle (2026-11-24), following 7d5c861. User directs at most three focused
attempts per blocked step/check, record the failure and continue to independent
feasible work. Skipped acceptance stays incomplete. A03 remains deferred.
A02/A03/A04 remain PARTIAL; A04 is active. A01 accepted baseline 6de428b unchanged.
Full 3D remains gated behind G2D. MIT OR Apache-2.0.

## Done

- ADR-0101: source-only script and editable graph projectile range samples. Moving
  enemy, stop/boost input, projectile sensor hit, enemy removal, shared hits data,
  downstream victory sensor and actual green indicator. Root HUD mirrors shared
  hit count. Text motion uses imported functions; graph versions use existing
  host calls, explicit trigger chains, branch routing and the same frontend/VM.
- Deterministic kiln-graph example regenerates guarded v2 graph documents with
  stable node/pin/connection identities. Graph comparisons accept ordinary values
  matching text/boolean/numeric equality; arithmetic remains numeric. Refresh
  preserves compatible prior pin identities. No new dependency or runtime ABI.
- Real gameplay exposed same-tick update after collision-time self-despawn.
  AttachedScript checks liveness before every non-stop callback. Stop still runs
  once through started guards; other instances continue. Focused before control
  fails; corrected regression passes.
- Rebuilt matching editor/CLI/player and default-feature browser runtime. Contract
  remains v6. Prior browser bundle retained in target/a04-range-prior-web-runtime.
- CI exercises both samples through relocated native/browser green-pixel observers
  and retains screenshots/receipts. Hosted execution is not claimed locally.
- Prior work includes bounded shared graph lowering/v1 migration, shared callable
  bodies, imported frames/locals, nested watches, shared tick budgets and safe
  staged Restart. See docs/PLAN_PROGRESS.md and evidence/A04_PROJECTILE_RANGE.md.

## Executed validation

- Locked Rust workspace: 855 tests PASS. just check PASS. Default and opt-out WASM
  strict Clippy PASS. Python helper suite 49 PASS. Focused lifecycle/parity checks
  rerun after adding equivalent HUD resources. Whitespace checks PASS.
- Text/graph actual physics comparison on 240 ticks passes: stop/boost/release,
  positions, sensor events, removal, shared hit data, trigger state and indicator.
  No collision events are injected into this game test.
- Logical pre-impact snapshots restore both games and reach the same victory,
  enemy removal, hit data and green indicator. Exact physics continuation does
  NOT pass; it remains explicitly blocked below, not silently weakened.
- Both source and graph games pass actual relocated native and Chromium exports,
  with 3932 green pixels each, source copies hidden and no developer tools in
  native player PATH. Final HUD replay logs/receipts are under
  target/a04-range-{native,browser}-final-*. Screenshots inspected.
- Raw before failure: target/a04-range-despawn-before.log. Corrected tests:
  target/a04-range-lifecycle-parity.log. Other logs: a04-range-{workspace,quality,
  wasm-check,wasm-optout,python,tools-build,web-build,precommit}.log.

## Blocked / skipped checks

- Exact resumed physics poses diverge immediately after pre-impact load:
  projectile x=140.00006 versus uninterrupted x=138.33339 at tick 46. Logical
  saves do not preserve kinematic warm-start/history. Failure retained in
  target/a04-range-exact-physics-continuation-blocked.log. Stop retrying this
  acceptance check; continue other plan work. Restored final outcomes pass and
  are explicitly narrower than exact continuation. No general physics-save
  determinism fix is claimed in this A04 checkpoint.
- External hardware/device/cross-platform certification is outside this request.

## Next

- Continue feasible A04 source breakpoints, statement-level continuation and
  active lexical scopes/expression watches, applying the user's bounded-attempt
  skip policy. Imported frames already work. Equivalent game implementation now
  exists, but full editor-authoring/debugger/production acceptance remains open.
- Advance past genuinely blocked steps with recorded evidence; keep milestone
  status PARTIAL when acceptance was skipped. Do not rerun a blocked check merely
  to delay moving forward. A05 has not been started in this checkpoint.
- A03 deferred: nested/variant identities, arbitrary component overrides, general
  multi-file crash transactions and combined production acceptance. A02 combined
  every-inspector-property widget acceptance remains open.
