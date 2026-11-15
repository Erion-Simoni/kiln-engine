# A02 stable floating-panel replay — 2026-11-15

Branch `fix/a01-ci-baseline`, following `276524c`. A02 remains PARTIAL.

## Reproduced failure and repair

Reference-game run [43711940851](https://github.com/Erion-Simoni/kiln-engine/actions/runs/43711940851)
on `276524c` failed with `skybridge authoring: KeyError: 'run'`, matching prior
`c9958d7` run 43709956415. The same replay failed locally before this repair.
The failure screenshot shows the animation panel at the top-left while replay
clicks still targeted the expected widget origin. Recorded actions show a title
segment at x=354 was used as a drag origin, moving the panel away from its expected
position before clip creation. A diagnostic capture with a settled title starts
at x=33. The first painted title segment is insufficient evidence of settled
floating-window geometry after closing the enlarged source panel.

The shared screen observer now requires three consecutive matching title origins
before deciding whether to drag. After a drag, it waits for the expected rendered
origin before targeting widgets. It retains the existing bounded wait and fails
with a specific panel-position diagnostic. No application hook, longer arbitrary
sleep, swallowed failure, project-data injection or weakened comparison was added.
The complete saved project must still equal the checked-in game.

## Validation

- Local Skybridge authoring FAILED before the repair at the same missing `run`.
- After repair, Skybridge real-widget authoring PASSED with exact full-project
  equality, exact source bytes and unchanged original template. Inspected the
  animation screenshot: `run`, two frames, second X=16, sheet `art/hero.png`.
- Two observer regressions cover a transient first title segment and verifying
  the destination after a drag sequence. Both FAIL against the original observer
  and PASS after the repair. Python suite: 40 PASSED.
- `just check` PASSED formatting, strict Clippy and workspace/audio checks.

Raw evidence: `target/a02-animation-authoring-before/`,
`target/a02-animation-authoring-after/`, `target/a02-animation-layout/`,
`target/a02-animation-authoring-logs/`.
An initial full replay used the unpackaged debug directory and correctly failed
export-notice verification because that directory had no installation license
tree. This is not counted as export acceptance. The subsequent packaged replay PASSED all three games.

## Packaged reference replay

Built the native editor/CLI/player and prepared a verified portable installation
with the existing prebuilt browser runtime and locked native/WASM dependency
metadata. `task9_editor_games.py --author` PASSED Emberwatch, Switchyard and
Skybridge: all three full projects equal their checked-in references, pasted
source bytes match, and original templates remain unchanged. All six native/web
exports preserve the complete license inventory and bind the actual runtime
bytes in their receipts. Repeating each export rejects overwrite and preserves
the successful receipt. Child PATH contains no external authoring tools.

Report: `target/a02-panel-packaged-replay/result.json`. Package verification:
`target/a02-animation-authoring-logs/package.log`. This step validates authoring
and exports; standalone/browser gameplay was not rerun and physical-device
acceptance is not inferred. The repair remains in the shared screen observer;
no game or application implementation was changed.

## Hosted source boundary

On `276524c`, [CI 43711940922](https://github.com/Erion-Simoni/kiln-engine/actions/runs/43711940922),
[Linux candidate 43711940942](https://github.com/Erion-Simoni/kiln-engine/actions/runs/43711940942)
and [Windows candidate 43711940872](https://github.com/Erion-Simoni/kiln-engine/actions/runs/43711940872)
PASSED. Reference-game replay failed as described above. These results precede
this harness repair; matching hosted results remain required. A01 acceptance
remains pinned to its documented baseline; full A02/cross-platform/device and
combined all-property recovery acceptance remain open.
