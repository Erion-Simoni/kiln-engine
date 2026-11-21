# A04 staged saved-source restart

2026-11-21, branch `fix/a01-ci-baseline`, following `653f5ad`. User explicitly
requested finishing or deferring the current A03 work and moving to the next item.
Remaining A03 acceptance is deferred, still PARTIAL; this is an A04 checkpoint.

Restart previously stopped the game before compiling the next run. ADR-0095
requires a saved/reverted source buffer and prepares the replacement session, world
and camera before stopping. Source read/compile/camera errors preserve the existing
VM/world and pause/error/clock state. A successful preparation invokes existing Stop,
installs the fresh authored runtime and starts an unpaused clock. Audio output is
opened after replacement, retaining the ordinary nonfatal device-error behavior.
Tooltips explain saved-source restart and failure preservation. KILNSCRIPT records
the reload/reset/persistence contract; no implicit source save or live VM patching.

Two headless tests verify an imported function returning 1, invalid saved imported
code rejecting replacement with the module filename, and the original VM continuing
to score 2 despite the invalid disk file. Corrected imported code returns 7 in a fresh
replacement while the old run stays unchanged; paused/error state and authored
project/save-file bytes are checked. Missing source and invalid camera reject
preparation. The fresh session starts with score zero, unpaused and without errors.
An initial fixture put an import and statement on one line; the language requires
imports on their own lines. Corrected fixture passes; diagnostics are retained in
the initial log. No production assertion was relaxed.

Raw logs: `target/a04-restart-{initial,tests,tests-final,workspace-final,quality-final}.log`.
This is headless replacement preparation/VM proof plus compiled UI wiring, not an
actual widget/physical GPU/audio replay. Future runtime errors after replacement
follow normal Play handling; user-authored callbacks may access persistence. Live
source breakpoints, imported frames/locals/watches, and the equivalent script/graph
acceptance game remain open. A04 is PARTIAL; this checkpoint does not close it.

Final local validation: **822 Rust tests passed** and `just check` passed. Hosted
confirmation for this commit remains pending.
