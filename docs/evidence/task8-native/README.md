# Task 8 native export checkpoint — 2026-11-06

Actual Linux x86_64 editor/player captures with isolated Xvfb/software graphics
and user-data profiles. The editor process used an empty tools PATH, with its
prebuilt exporter/player beside it. No Cargo ran during export. The complete
artifact was moved outside the repository and then launched from /tmp with no
archive argument; Cargo and rustc were absent from its PATH.

- [Verified export](export-verified.png): Save & Export persisted an unsaved
  inspector X=88 edit and published a validated standalone game/receipt/archive.
- [Failed rebuild](export-failed-preserved.png): actual nonzero exit for an
  existing destination, with the prior archive bytes unchanged.
- [Active export](export-active.png): responsive editor with streaming diagnostics
  and Cancel during a real runtime handshake.
- [Cancelled export](export-cancelled.png): UI cancellation exited the CLI and left
  no staging/output for that job.
- [Relocated game resume](relocated-resumed.png): fresh-process Continue preserved
  health 66, brass_key inventory and attached-script start count 1. Settings and
  corrupt-slot handling passed as well.

`editor-result.json` records output sizes/assertions. `relocated-result.json`
records gameplay/storage assertions after moving the artifact. Full evidence is
created by `tests/native/export_smoke.py` and `tests/native/relocated_export.py`
and uploaded in native CI artifacts. Unit/integration suites also test invalid
media/scripts/resources, missing outputs despite zero exit, both diagnostic
streams, cancellation fallback and process reaping.

This is **not a clean-machine OS/dependency or release-size test**. The runner
already has graphics/audio/window-system libraries. ALSA null output does not
certify audible devices. Physical GPU/controller, Windows/macOS, minimum ABI,
portable editor packages, browser exports, signing and remote CI results remain
separate acceptance gates. Task 8 is still in progress; see NATIVE_EXPORT_2D.md.
