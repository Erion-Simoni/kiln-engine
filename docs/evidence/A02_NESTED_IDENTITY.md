# A02 nested identity protection — 2026-11-15

Branch `fix/a01-ci-baseline`, following `9bc2ace`. A02 remains PARTIAL.

## Reproduced defects and repair

The runtime uses SourceId without a scene namespace. Nesting duplicated scenes
therefore accepted ambiguous IDs, while editor maps/snapshots arbitrarily chose
one runtime entity. Three before-fix regressions FAILED: direct collision,
ancestor/transitive collision, and preserving an authored root whose ID is reused
by a nested entity. The real-widget duplicated-scene nesting regression also
FAILED before the guard: the invalid link was saved.

Nesting now validates all affected ancestor compositions before publishing a link.
It rejects shared entity IDs, repeated transitive instances and missing scene
references. Traversal is iterative and bounded by visited scene identities; the
cycle reachability check is iterative too. Failure leaves documents/history
unchanged and gives an inline, dismissible error plus console/log explanation.

Loaded legacy/external ambiguous compositions are protected too: snapshots keep
the original root entity document rather than baking arbitrary runtime values,
and ambiguous handles are omitted from inspector/gizmo selection mapping. No
on-disk identity migration, automatic script rewriting or namespace is invented.
Separate scenes may still share IDs; only composing them creates the conflict.
This is data-preservation protection, not full scoped/repeated scene instances.

## Executed validation

- Before/after unit regressions: both topology tests FAILED before/PASSED after.
  Root snapshot/disk-preservation regression FAILED before/PASSED after.
- Real widgets: duplicate the scene, confirm original IDs retained, attempt to
  nest main into the copy, verify complete project/history unchanged and a specific
  diagnostic. Save/restart remains exact.
- External legacy fixture adds a colliding composition, opens its scene, selects
  the ambiguous row and attempts movement. No wrong target is edited; saving and
  restart preserve the complete imported project. Screenshot retained.
- Existing scene-settings workflow reruns valid distinct-ID composition, offsets,
  removal/Undo and rendered bounds, camera cancellation/recovery and exact restart.
- `just check` PASSED. Full workspace tests: 773 PASSED, zero failures, including
  doc tests. Python: 40 PASSED. Both final widget workflows PASSED; screenshots inspected.

CI runs the identity test and keeps normal/legacy screenshots and editor logs.
Raw evidence: `target/a02-nested-identity/`, `target/a02-nested-identity-before/`,
`target/a02-scene-settings-logs/identity-*`. Linux Xvfb/software graphics only.
Matching hosted checks, every-property combined acceptance, scoped repeated nested
instances and cross-platform GUI acceptance remain open. A03/A04 remain PARTIAL.
