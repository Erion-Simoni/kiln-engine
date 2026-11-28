# A04 real source breakpoints and resumable gameplay ticks

2026-11-24, fix/a01-ci-baseline, following 0c73b3d. ADR-0104 implements the next
A04 software step. A04 remains PARTIAL; final acceptance gates are not promoted.

## Implemented

Statement-boundary metadata maps compiled instructions to exact imported source
lines without changing bytecode or VM-memory ABI. Up to 128 unconditional lines
apply across real scene/attached VMs. Source stops retain operand/local/call stacks,
queued commands and remaining instruction allowance. Resume skips the stopped
boundary once; Step Statement enters calls and stops at the next executed statement.

A pending tick retains input, time, collisions, animation/signal payloads and its
attachment callback cursor. Completed callbacks and systems are not replayed.
Callback queues reference the captured payload pool rather than eagerly cloning
all signal values. Commands commit once on callback completion. Direct state writes
already executed are visible; this is not rollback. Unconfigured execution retains
the existing fast VM loop.

The live editor displays source stops, supplies line controls and statement steps,
and rebinds breakpoints during staged saved-source Restart. Missing new lines are
logged/omitted rather than breaking a valid Restart. Logical saves reject unfinished
ticks. Stop abandons pending commands then performs bounded cleanup with probes
disabled. Saved-screen input/start/UI-event callbacks remain atomic. Definitions
are session-local; scene transitions/logical loads may require reinstalling them.
Step-over/out and caller-frame scope selection are not added. No dependency or
runtime contract/save-format change; contract stays v6.

## Checks

- Locked workspace: 870 Rust tests PASS. just check PASS. Default/opt-out WASM
  strict Clippy PASS. Python helpers 49 PASS; py_compile and whitespace checks.
- Focused source tests cover imported stops, real return/caller continuation,
  repeated calls with pending arithmetic operands, exact callback/event ordering,
  shared signal payloads delivered once, frozen input/time, one physics advance,
  unfinished-save rejection, queued-command abandonment and probe-free cleanup.
  Repeated stops cannot replenish the shared 60-instruction limit.
- Final review found attached frame_count was reset when re-entering a suspended
  callback. The new before control fails with I64(0) instead of I64(1); guarded
  initialization fixes it. All seven source tests plus existing entity/shared
  budget tests pass after that correction. just check rerun. Matching editor,
  CLI/player and prepared browser runtime rebuilt; previous bundle preserved.
- Both script and graph projectile games pass relocated native and Chromium
  exports: 3932 green pixels each, source copies hidden and native player PATH
  without developer tools. This verifies normal gameplay after the queue refactor.

## Widget gate: incomplete, stopped after three attempts

Attempt 1 stopped on ambiguous OCR Pause labels. Attempt 2 stopped on punctuation
before OCR Add. Attempt 3 observed imported breakpoint installation, Restart
rebinding, real scope value 4, statement Return into main line 3, caller result 8,
and completion of the pending frame with no active scope. Screenshots inspected.
It then failed manifest byte comparison: the baseline was captured before the
setup's normal Save normalized the original TOML (defaults such as audio_assets
and mobile settings were added). The harness now captures baseline after that
Save, but this correction was NOT replayed a fourth time under the user's policy.
No overall widget PASS or manifest-preservation PASS is claimed. Keep this gate
incomplete and move to independent A05 work; A04 stays PARTIAL.

Raw logs: target/a04-source-{initial,focused,final-focused,workspace-final,
quality-final,wasm-final,wasm-optout-final,frame-before,python,precommit,tools-final,
web-final}.log. Widget attempts: target/a04-source-widgets{,-replay,-final}/ and
sibling logs; final partial-result.json records the incomplete gate. Export logs,
receipts/screenshots: target/a04-source-{native,browser}-projectile_range{,_graph}/.

Previously blocked exact saved-physics continuation was not retried. Saved-screen
UI debugging, full combined production acceptance and external certification stay
open. Proceed to A05 per the user's bounded-attempt/skip direction, without marking
A03 or A04 complete.
