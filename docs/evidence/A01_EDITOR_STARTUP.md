# A01 editor startup synchronization — 2026-11-14

Resumed `fix/a01-ci-baseline` at `3efb687475569a3a3df0bb9f13f6e28e3bf49ddc`
in a fresh environment. Existing implementation and handoff were recovered from
that branch, including the graph checkpoint. Work in this checkpoint stays on A01.

## Hosted observation

General CI [43603619952](https://github.com/Erion-Simoni/kiln-engine/actions/runs/43603619952)
failed in the browser structured-scripting authoring smoke before source entry:
`Render floating panel from menu 118, row 68 timed out`. The prior color-tolerance
repair did not resolve that hosted failure. Windows candidate 43603619881 and
reference games 43603619867 passed on the same source. Linux candidate
43603620009 was still in progress when recorded. Public summary/list HTML is the
evidence source; authenticated step logs and artifact contents were not available.
See `A01_HOSTED_STATUS.json` for the observation time and earlier results.

## Reproduction and repair

The helper previously treated X11 window allocation plus 0.7 seconds as render
readiness. Suspending only the owned editor immediately after finding its window,
then resuming after 2.5 seconds, reproduced the exact floating-panel timeout.
The captured screen retained both default Script Editor and Animation Editor tabs:
the startup close clicks had been sent before rendering. The Project menu correctly
selected the existing docked Script Editor, leaving no floating title to observe.
This reproduces a sufficient cause locally; it does not establish that every
previous hosted timeout had this cause.

Startup now observes the actual painted menu (theme background plus visible glyphs)
before closing tabs. Tab closing and panel-menu interactions use separate held
press/release events. The floating-title, saved-source, movement and missing-canvas
assertions remain active. No application state hook or renderer change was added.

`tests/native/editor_startup_smoke.py` reproduces delayed readiness with SIGSTOP /
SIGCONT on its owned process, then requires the real menu to reopen a floating
source editor. Native CI now runs it and retains screenshots, logs and receipts.
Unit coverage rejects unpainted/solid/truncated frames and accepts dithered text.

## Local validation

- Default editor/player/CLI build and strict workspace/all-target Clippy passed.
- Delayed first-render reproduction: failed before the fix; passed after it.
- New committed delayed-start GUI regression passed; its screenshot visibly shows
  Script Editor floating with both original source/animation tabs closed.
- External PNG refresh plus editor-authored enum/task/timer/dictionary/loop source
  smoke passed. Source formatting/navigation/completion/save/restart smoke passed.
- `just check` passed and Python regressions: 38 passed. Forced-crash recovery,
  Restore/Save, explicit discard, exit Cancel/Discard and unreadable-sidecar
  preservation passed. No full Rust test or browser rerun is claimed here.

Raw local evidence is excluded from Git: `target/a01-delayed-start/{before,after}`,
`target/a01-editor-startup`, `target/a03-a04-workflow`, `target/a04-source-tools`.
This is Linux Xvfb/software graphics evidence. A matching hosted run remains
required; A01–A04 remain PARTIAL.
