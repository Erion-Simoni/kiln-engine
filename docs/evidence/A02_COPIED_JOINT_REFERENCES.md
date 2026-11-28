# A02 copied joint references and recovery — 2026-11-17

Branch `fix/a01-ci-baseline`, following `50b74d1`. A02 remains PARTIAL.

## Reproduced defects and repair

Joint Add/Set resolved the edited composition, but clipboard paste independently
checked only root entities. It rejected valid unique nested endpoints and accepted
an ambiguous endpoint when a root entity happened to share that ID. Duplicate
could publish a joint pointing to a missing entity. Prefab placement/Revert copied
joint self-references from the definition without replacing the template owner ID.
Four new Rust regressions FAILED before the fixes.

Entity commands now validate newly published/changed joints on the fully staged
scene before assigning the document or recording history. Clipboard remaps internal
references first and leaves external IDs for that shared composition-aware check.
A valid nested endpoint is retained; missing/ambiguous endpoints refuse the complete
operation without clearing redo. Unchanged legacy joints do not block unrelated
edits. Previous entities are indexed once for changed-joint comparison.

A shared owner-reference remapper preserves the existing duplicate semantics and
also applies them to prefab placement and Revert. Only endpoint references equal
to the copied template/owner ID change; external references and joint properties
remain intact. No manifest migration, dependency, ID namespace or history stack.
This does not implement nested prefabs/variants or general repeated scene instances.

## Executed validation

- Before/after Rust regressions: valid nested paste, ambiguous nested paste with
  redo preservation, missing-reference Duplicate, and prefab placement/Revert owner
  identity. All four FAILED before and PASSED after.
- Prefab integration also saves/reopens the complete project and checks the joint
  on the actual spawned runtime instance; anchors/properties and endpoint IDs agree.
- `just check` PASSED formatting, strict workspace/all-target Clippy and
  workspace/standalone audio checks.
- Full `cargo test --workspace --locked`: 781 PASSED, zero failures including docs.
  Python regressions: 44 PASSED. Script compilation/diff checks PASSED.
- `copy_joint_reference_smoke.py` PASSED actual widgets/clipboard:
  - Attach the controller's joint to Player and copy its joint/script data.
  - Paste into an independent empty scene: visible rejection; save confirms the
    complete project was not partially changed.
  - Nest the source scene through the picker. The same paste now resolves Player
    uniquely; the copied owner receives a new ID, the target stays `player`, and
    script path, joint fields and original source scene remain exact.
  - One Undo removes the whole paste; Redo restores the same ID and properties.
  - A second unsaved paste reaches the real autosave timer without writing the
    manifest. Kill the editor, Restore/Save, compare the entire expected project
    and joint/script data, then ordinary restart compares the complete project.
- Existing `entity_clipboard_smoke.py` PASSED same/cross-scene identity, paste
  history and exact restart. Existing `entity_reference_smoke.py` PASSED invalid
  joint draft correction, Cancel/redo, camera/joint deletion guards, reference
  resolution and exact restart. Screenshots inspected.

CI runs the new workflow and retains receipts, screenshots and editor logs.
Raw evidence: `target/a02-copy-joint/`, `target/a02-copy-reference-logs/`,
`target/a02-copy-reference-guards/`, `target/a02-entity-clipboard/`.
The before-fix claims refer to the Rust regression logs; the preliminary GUI
receipt was generated after integration-test builds had already relinked the editor.
Linux Xvfb/software graphics only; no physical joint simulation or cross-platform
GUI acceptance is inferred from these authoring/data-preservation tests.

## Source-pinned hosted boundary

On prior source `50b74d1`, [CI 44078732105](https://github.com/Erion-Simoni/kiln-engine/actions/runs/44078732105)
and [reference games 44078732134](https://github.com/Erion-Simoni/kiln-engine/actions/runs/44078732134)
PASSED, confirming the centered-error replay repair remotely. Linux candidate
44078732106 was IN_PROGRESS at observation; Windows was not triggered for that
test/docs-only change. Prior production `7796386` Windows 43725520747 and Linux
43725520736 passed. These do not accept the new copy-command production changes.
Matching hosted results, full every-property combined acceptance and scoped repeated
nested identities remain open; A02–A04 stay PARTIAL.
