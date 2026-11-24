# A04 projectile range, graph comparisons and despawn lifecycle

2026-11-24, branch fix/a01-ci-baseline, following 7d5c861. A04 remains PARTIAL;
A03 remains deferred. User-directed policy is to stop repeating blocked checks,
retain their evidence and proceed to independent feasible work.

## Implementation

Added samples/projectile_range and samples/projectile_range_graph: a cyan
projectile moves automatically, Left/A stops it, Right/D doubles speed. An orange
enemy moves toward it. A real filtered sensor collision despawns the enemy and
sets shared game data hits=1. The projectile continues to a purple victory sensor,
which activates a large green indicator only after the hit. Text motion calls an
imported function. The equivalent visual resources contain guarded editable v2
metadata and ordinary generated source, with explicit triggers and branch routing.
No new runtime host API or dependency is needed.

The deterministic kiln-graph projectile_range example regenerates all four graph
files with stable node/pin/connection identities. Regeneration of all four resources is
byte-for-byte reproducible. Comparison pins now accept normal
Value inputs, matching VM equality for text/boolean/numeric values. Arithmetic
pins remain numeric. Tests cover string/boolean comparisons and preserve prior
numeric pin identities during refresh.

The game found a genuine lifecycle defect: collision-time self-despawn was followed
by update in the same tick, allowing reads of an entity already removed from ECS.
The focused before control fails; AttachedScript now checks liveness before every
non-stop callback. Existing started guards still issue stop once. Other instances
continue normally. The regression checks skipped update and exactly one stop.

## Checks and boundaries

- Full locked Rust workspace: 855 tests PASS. just check and default/opt-out WASM
  strict Clippy PASS. Focused lifecycle/parity tests rerun with final HUD scripts.
- Python helper suite: 49 PASS.
- Exact text/graph parity on each of 240 fixed ticks: input stop/boost/release,
  projectile positions, real sensor collisions, enemy removal, shared hit data,
  trigger activation and actual sprite color. No collision events are injected.
- Pre-hit logical snapshots restore both variants and reach the same victory,
  removal, shared data and green indicator. **Exact physics pose continuation is
  blocked:** resumed projectile x=140.00006 versus uninterrupted x=138.33339 at
  tick 46. Kinematic warm-start/history is not represented by the logical save.
  This check was stopped, not relabeled as passing; final-outcome checks have a
  narrower explicit scope. Raw failure is retained below. Investigating full
  physics continuation is deferred to its plan step.
- Matching editor/CLI/player and prepared default-feature browser runtime rebuilt.
  Runtime contract stays v6; source/VM memory ABI is unchanged. Previous prepared
  browser runtime retained under target/a04-range-prior-web-runtime.
- Actual relocated native and Chromium exports PASS for both projects: each
  observes 3932 green pixels after real sensor gameplay, with source copies hidden
  and native player PATH without developer tools. Final receipts live under
  target/a04-range-{native,browser}-final-*. Screenshots inspected.
- Native/browser CI runs both projects through relocated export pixel observers
  and retains receipts/screenshots. Hosted execution is not claimed locally.

Raw logs: target/a04-range-{generate,graph,despawn-before,lifecycle-parity,
exact-physics-continuation-blocked,workspace,quality,wasm-check,wasm-optout,python,
tools-build,web-build,precommit-final,regenerate-check}.log. Export receipts live under target/a04-range-native-*
and target/a04-range-browser-* with sibling play logs.

This advances the equivalent-game requirement; it does not complete its full
production/debugger acceptance. Source breakpoints, statement continuation and
lexical expression watches remain open. No external hardware/device certification
is claimed or needed for the completed software checks.
