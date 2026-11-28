# A03 interrupted atlas publication recovery

2026-11-19, `fix/a01-ci-baseline`, following `5bc6e0a`. A03 stays PARTIAL.
ADR-0090 closes the saved-atlas checkpoint's interrupted PNG/manifest gap for
known-generated atlas outputs. Arbitrary source-file transactions remain open.

Before replacing a generated PNG, persist bounded project/recipe/output/hash
provenance. Explicit project Open reconstructs the saved recipe, while recovery
Restore reconstructs the selected draft. Both use current source art and preserve
manifest/source bytes. Independently edited outputs remain protected. Successful
manifest Save compacts only matching committed provenance. Multiple generations
and an older autosave can be selected after newer unsaved builds.

Save As copies validated provenance and normalizes only its isolated copy. Native
and browser export rebuild the saved recipe in staging, preserve source output and
provenance bytes, and strip private metadata before publication. Private ledger,
lock and interrupted temporary files are hidden from asset browsing/copying.
Standard-library advisory locks serialize cooperating publishers and release after
process exit; orphan temporary names are skipped without deleting their contents.
The ledger is limited to 512 KiB/4096 entries. No dependency was added.

## Executed evidence

- Locked Rust workspace: **804 passed**, zero failures. `just check` passed.
  Default/opt-out WASM strict Clippy passed. Python native helper suite:
  **49 passed**; smoke script compilation passed.
- Atlas unit tests cover saved/older JSON recovery selection, unknown output edits,
  foreign project ownership, invalid provenance, active lock and orphan temp names.
- Editor integration covers exact saved reconstruction, unchanged source/manifest,
  Save As preserving its pending draft without mutating source files/provenance,
  and read-only disk-conflict checks retaining pending output through Save.
- The prior loader behavior failed the saved-output reconstruction assertion.
  Actual widgets then exposed Save's disk-conflict check accidentally triggering
  Open recovery. Separating read-only load from explicit recovery fixed this;
  the dedicated regression and complete forced-crash workflow pass.
- Real Linux widgets publish a second source that shifts hero's clip rectangle,
  crash before the real autosave, reopen and verify exact saved PNG/manifest plus
  green hero pixels. They publish again, wait for the actual timer snapshot,
  crash, Restore and Save, compare the entire project to that snapshot, compare
  exact restored PNG bytes, verify source art/scripts unchanged and provenance
  compacted, then restart and verify rendering/persistence again.
- Snapshot comparison normalizes only typed frame-duration f32 representation
  between JSON's shortest decimals and TOML's f64 spelling; every other authored
  property compares exactly. No production assertion was relaxed.
- Native KAR and browser staging regression exports the saved 16px recipe after
  an unsaved 32px build, verifies current blue source pixels and source preservation,
  and verifies private provenance is absent from the distributed project/archive.

Raw local artifacts: `target/a03-atlas-recovery-{workspace,python,integration}.log`,
`target/a03-atlas-recovery-{quality-final,export-final,private}.log`,
`target/a03-atlas-recovery-final-widget.log` and
`target/a03-atlas-recovery-final/{result.json,*-crash.json,*-restore.json,editor,reopened}`.
Before/failure diagnostics remain separately in `target/a03-atlas-recovery-before.log`
and `target/a03-atlas-recovery-serialized`. Native CI runs the new workflow after
creating its saved-atlas fixture and uploads focused receipts/screenshots/logs.
Screenshots were inspected. Visual proof is Linux Xvfb/software graphics only;
physical GPU, other OSes and hosted checks for this new commit remain separate.
