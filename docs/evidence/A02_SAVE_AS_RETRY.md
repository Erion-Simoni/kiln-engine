# A02 Save As failure/retry and source protection — 2026-11-15

Branch `fix/a01-ci-baseline`, following `d012e01`. A02 remains PARTIAL.

## Fixed behavior

Save As previously closed its form as soon as Save was clicked, including when
copying failed. The form now closes only after successful I/O or explicit Cancel /
Escape. Errors appear in the form; destination and draft remain available for
retry. Cancel clears this error without reopening a generic failure popup.

Successful Save As previously cleared the hierarchy view without rebuilding it.
It now rebuilds the active scene and clears stale selection, so the copied project
can immediately be edited through its hierarchy and Inspector.

The save path also used to call script.save() before validating/copying a project,
which could write an unsaved source buffer even if the project copy later failed.
Save As now reuses the existing clean-script guard instead, restoring the documented
Save/Revert requirement. It does not silently save dirty source. Explicit Save
Script remains available after cancelling the copy; source guard messaging now
applies to all operations that reuse that guard.

No new dependency, serialization format or command architecture is introduced.
Same-directory manifest copies still share their resource directory; cross-directory
copies retain the existing no-merge, symlink-rejection and atomic staging behavior.

## Executed validation

- `just check`: PASSED formatting, strict workspace/all-target Clippy, workspace
  and standalone audio sample checks.
- Source guard tests: 7 PASSED.
- Project-lifecycle/roundtrip/two-scene integration: 20 PASSED.
- Native Python regressions: 38 PASSED.
- `tests/native/save_as_failure_smoke.py`: PASSED real Xvfb widgets:
  - Edit actor X=88 without saving. Save As into another occupied project fails
    visibly in the retained form. Source manifest/resources and foreign manifest /
    sentinel remain byte-identical.
  - Escape cancels the failed form while retaining the scene draft. Reopen, fail
    again, then correct the destination without closing the form.
  - A controlled resource symlink causes failure during staged copying. The changed
    error is observed in the same form; no partial destination or private staging
    remains. Remove only that test-created symlink and retry successfully.
  - The resulting copy includes X=88 and exact resource bytes. Its rebuilt hierarchy
    is selectable; editing Y=72 and saving changes only the copied manifest.
  - Load a script from the copy and paste a valid unsaved marker comment. Save As
    refuses before writing source or publishing a destination. The original source
    project/resources and foreign project remain unchanged.
  - Escape preserves the source buffer. Explicit Save Script writes the exact draft
    to the copy, then normal project save and complete restart preserve the manifest
    and script. The original source project's files remain unchanged throughout.
- Existing entity clipboard widget smoke PASSED same/cross-scene identity/history
  and complete restart after the hierarchy rebuild change.
- Inspected retained-copy-error, copied-project and dirty-source guard screenshots.

Raw receipts/captures: `target/a02-save-as-failure/`; logs:
`target/a02-save-as-logs/`. CI runs the new workflow and retains evidence. This is
Linux Xvfb/software graphics validation, not fresh Windows/macOS GUI or device
certification. A02's broader component/multiselect/cross-platform coverage remains
open; no complete transactional dirty-buffer overlay copying is claimed (users
must explicitly Save/Revert first).

## Prior-source hosted observation

On `d012e01`, Windows candidate 43706643993 PASSED. General CI 43706643952,
reference games 43706643985 and Linux candidate 43706644024 were IN_PROGRESS at
observation. That source contains the graph-dialog layering fix but predates this
Save As change. Public workflow-list evidence only; matching confirmation for
both this change and the graph-dialog regression remains required.
