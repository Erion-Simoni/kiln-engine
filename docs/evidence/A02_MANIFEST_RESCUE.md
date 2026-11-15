# A02 draft rescue after external manifest changes — 2026-11-15

Branch `fix/a01-ci-baseline`, following `ac0a86c`. A02 remains PARTIAL.

## Reproduced defect and fix

Normal Save correctly refuses to overwrite an externally changed/corrupt manifest
or recreate a deleted one, and directs users to Save As. Cross-directory Save As
nevertheless canonicalized the old manifest solely to exclude it during resource
copying, making rescue fail if that file was gone. A new disk-backed regression
FAILED before the fix with `deleted: No such file or directory`.

Resource copying now excludes the loaded manifest's filename under the already
canonical source root. The destination manifest is generated from the in-memory
project snapshot; no readable/existing source manifest is needed. Other files,
including unrelated TOML resources, retain their existing copy behavior. A replaced
manifest link is excluded without importing its target; ordinary resource links
remain rejected. Destination no-merge, nested-path and staging/publication guards
are unchanged. No new dependency, format or command architecture.

Project-operation errors use the existing error color and can be dismissed with
Escape without discarding the draft. Save As's own inline error and explicit
Save/Revert requirement for dirty script buffers remain unchanged.

## Executed validation

- Missing-manifest rescue regression: FAILED before, PASSED after.
- `just check`: PASSED formatting, strict workspace/all-target Clippy, workspace
  and standalone audio sample checks.
- Project roundtrip/lifecycle/two-scene integration: 22 PASSED. New coverage checks
  deletion, corruption and valid external edits; the guard refuses normal save,
  copied metadata matches the complete in-memory draft, original disk state and
  unrelated resources remain exact, and staging is removed.
- Unix coverage checks a replaced manifest symlink is ignored without altering its
  outside target, while adding a linked resource still refuses publication.
- Python regressions: 38 PASSED.
- `tests/native/manifest_rescue_smoke.py`: PASSED three separate real widget cases:
  - Make actor X=88 unsaved, then delete, corrupt or externally edit the original
    manifest. Normal Ctrl+S produces a visible error; the original state/resources
    remain untouched. Escape dismisses the error and retains the scene draft.
  - Save As into a fresh directory succeeds from memory with X=88, the original
    project name (not the external version), exact resources and no staging leak.
    The deleted manifest stays absent; corrupt/changed original bytes remain exact.
  - Select through the rescued hierarchy, edit Y=72 and save only the copy. Restart
    compares the complete rescued project. Source file state and resources remain
    unchanged through subsequent edits/restart in all three cases.
- Existing `save_as_failure_smoke.py` PASSED again: occupied destination/staged
  resource-link failures, retry/cancel, original preservation, clean-copy editing,
  dirty-source rejection, explicit source save and exact restart.
- Inspected missing-manifest error and editable rescued-copy screenshots.

CI runs the new three-case widget workflow and retains receipts/screenshots/logs.
Raw evidence: `target/a02-manifest-rescue/`, `target/a02-manifest-rescue-logs/`;
regression evidence: `target/a02-save-as-failure/`.
These are Linux Xvfb/software graphics results. Source directory/resources must
remain available; recreating an entirely deleted project/resource tree is not
implemented. Dirty script buffers still require explicit Save/Revert. Full A02,
cross-platform GUI and physical-device acceptance remain open.

## Prior-source hosted observation

On `ac0a86c`, Linux candidate 43708624752 FAILED the artifact-service preflight:
its probe download reported artifact `kiln-artifact-probe-43708624752-1` not found.
The application build/acceptance job did not run. This is a reported artifact
roundtrip failure, not an established application failure or diagnosed service root
cause; the preflight check was not weakened.

At observation Windows candidate 43708624748, general CI 43708624755 and reference
games 43708624764 were IN_PROGRESS. These precede the manifest-rescue change.
Matching hosted results and successful candidate preflight remain required.
