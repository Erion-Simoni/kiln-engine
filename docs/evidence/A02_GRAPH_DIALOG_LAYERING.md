# A02 authoring-dialog reliability followup — 2026-11-15

Branch `fix/a01-ci-baseline`, following `4270bd6`. A02–A04 remain PARTIAL.
No gameplay/compiler/debugger feature or new dependency is introduced.

## Reproduced defect and fix

Graph parameter editors were ordinary Middle-order egui windows opened while
rendering the floating graph panel. Raising the containing panel could obscure
an already-open parameter editor. A deterministic regression renders the real
graph widgets with an active constant editor, raises the containing graph window,
then hit-tests the parameter region. Before the fix, the parent layer intercepted
that region even though the parameter-edit state remained active. The regression
FAILED on the original code and PASSED after placing parameter windows in egui's
foreground order. The check asserts the actual topmost parameter identity rather
than prescribing an internal layer implementation.

This is a reproduced application defect and a sufficient cause of an invisible
parameter editor. Public hosted timeout annotations alone do not prove the cause
of every prior hosted failure. Matching hosted confirmation is still required.

The external widget replay also now waits for three visible states when selecting
a host function: the measured/recentered dialog, its open registry menu and its
closed picker before OK. It uses the existing captured pixels and narrow sRGB
color tolerance; it adds no application-state hook, blind click retries, longer
timeout or replacement success condition. Constant-dialog observation and saved
`add_score` source/gameplay/history/save-restoration assertions remain active.

## Executed validation

- Deterministic parent-window occlusion regression: FAILED before, PASSED after.
  Logs: `target/a02-dialog-logs/before.log`, `after.log`.
- `just check`: PASSED formatting, strict workspace/all-target Clippy, workspace
  check and standalone audio sample check.
- Real graph/text gameplay and inspection save/restore integration: 1 PASSED.
- Python regressions: 38 PASSED.
- Two separate fresh project/profile runs of
  `tests/native/graph_authoring_smoke.py --web-runtime target/debug/web-runtime`
  PASSED: actual host argument selection, Cancel preserving parameters, Undo/Redo,
  complete multi-frame drag undo, scene assignment, native execution and F5 save,
  saved node values, exact restart, browser execution and real F9/F5 restoration.
  First run: native score 261, browser score 360. Repeat: native 252, browser 342.
  Both browsers rendered 392 hero pixels and saved three graph node values; native
  saved state includes actual Integer(9) and scores are multiples of that argument.
- Raw receipts/screenshots: `target/a02-graph-dialog/` and
  `target/a02-graph-dialog-repeat/`; quality/GUI logs: `target/a02-dialog-logs/`.

No new Windows/macOS GUI or physical-device acceptance is claimed. The existing
native and browser CI jobs already run this full graph workflow and retain evidence;
the new Rust regression runs with the workspace tests. The foreground change keeps
the existing disabled-during-Play/project-guard behavior and parameter transactions.

## Hosted observation at start

On `4270bd6`, Windows candidate 43705134977 PASSED. General CI 43705135020,
reference games 43705134990 and Linux candidate 43705134979 were IN_PROGRESS at
observation. These results predate this dialog fix. Earlier `bdab5e4` general CI
failed on the graph constant-dialog wait, as recorded in A02_HIERARCHY_TREE.md.
Do not promote prior-source results or local passes to matching hosted acceptance.
