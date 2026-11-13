# Unsaved script recovery

Dirty projects autosave every 30 seconds while editing. The recovery sidecar now
contains scene/project changes and the shared Script Editor's unsaved source.
Script-only changes also trigger autosave. Recovery is separate from Save: it does
not modify the source file or project manifest.

After an interrupted editor session, reopen the project and choose Restore.
Recovered source returns to the Script Editor; Save Script or Save Project writes
it using normal syntax/import validation and atomic file replacement. Unfinished
source can be recovered even if it does not compile yet. A failed Save keeps the
draft available.

The snapshot retains the original source-file baseline. If another editor changed
that file, Save reports the conflict and preserves both the external file and the
recovered buffer. Copy/merge the draft or use Revert before proceeding. If the
current file already equals the recovered text, it is treated as saved.

A newer saved project manifest takes priority over older recovered scenes, while
its independent script draft remains recoverable. Recovery paths use the same
project-relative `.ks`, link and 1 MiB source limits as ordinary source editing.

Escape leaves recovery pending. Discard Recovery explicitly removes that snapshot
without changing saved files. Discarding edits in the normal project/exit prompt
also removes that project's autosave. Successful Save clears the snapshot only
after writing the document. Restore alone retains recovery until that decision.
Unreadable or future-version snapshots remain pending rather than being silently
deleted or overwritten by autosave.

The sidecar is `.<manifest-name>.recovery.json`. Format version 1 wraps a project
and optional script draft; old project-only JSON snapshots remain readable. Atomic
writes use unique `.*.recovery.tmp` files. These generated files are excluded from
Git and project Save As copies. No project, game-save or export format changed.

Validation: `cargo nextest run --workspace --locked`, `just check` and
`python3 tests/native/script_recovery_smoke.py`. The GUI test uses actual 30-second
autosave, OS process kill/restart, clipboard edits and visible recovery controls;
it does not inject a successful snapshot. Its unsupported-version case separately
uses an invalid sidecar fixture to check data preservation. See ADR-0079 and
[the completion plan](ENGINE_COMPLETION_PLAN.md) for remaining authoring work.
