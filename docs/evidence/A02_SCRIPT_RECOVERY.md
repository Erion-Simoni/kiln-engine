# A02 unsaved source recovery — 2026-11-13

Implements the independent recovery substep while A01 hosted checks continue,
authorized by the owner's request to proceed through all tasks. This is not
acceptance of the whole A02 milestone or full 2D release. Design: ADR-0079.

Implemented contracts:

- Autosave includes the loaded script path, source buffer and original disk
  baseline in versioned project recovery. The editable filename field is not
  mistaken for the loaded file. Original project-only snapshots remain readable,
  and the public project-only recovery API remains compatible.
- Restore populates the shared editor buffer without writing source. Existing
  path/link/1 MiB rules apply. Invalid/unfinished code is recoverable; validation
  happens on Save. External modifications still block Save and retain both copies.
  Already-persisted identical source is recognized as saved.
- A newer saved manifest wins over old recovered scenes without losing the script
  draft. Recovery remains until successful Save or explicit Discard; Escape keeps
  it. Unknown/unreadable snapshots remain in a visible pending recovery state.
- Unique temporary sidecars are atomically replaced, ignored by Git and excluded
  from Save As project copies. Normal exit/project Discard clears the sidecar
  before dropping the buffer; cancellation keeps both.

Validation:

- `just check` passed (format, strict all-target clippy, workspace/audio checks).
- `cargo nextest run --workspace --locked`: 705 passed, zero skipped. Five new Rust
  regressions cover new files, external conflicts/failed saves, newer manifests,
  legacy/unsupported sidecars and invalid/oversized recovery. Existing Save As
  regression additionally checks interrupted recovery temporary-file exclusion.
- Python helper tests: 33 passed; all workflows pass actionlint; mdBook build passes.
- `tests/native/script_recovery_smoke.py` uses the actual 30-second interval,
  clipboard source editing, OS kill/restart and actual Restore/Save/Discard widgets.
  Restore, Save, Escape, explicit recovery Discard and unsupported-snapshot
  preservation passed. Normal exit Cancel preserved the draft/sidecar, and normal
  exit Discard removed recovery without modifying the saved source. All GUI
  scenarios passed; the unsupported sidecar is a deliberately invalid fixture.

Raw evidence is under target/a02-* and stays outside Git. Native CI runs this GUI
regression and retains its screenshots/logs/results. Xvfb/software graphics/null
audio provide no physical-device acceptance.
