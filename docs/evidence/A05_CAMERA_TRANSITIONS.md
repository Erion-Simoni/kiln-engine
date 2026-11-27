# A05 camera transitions — 2026-11-27

ADR-0108 adds `camera_transition(x,y,zoom,seconds)` and
`camera_transition_stop()` through the same bounded callback queue and shared
editor/native/browser adapter as shake. Targets are world-space origins; zoom is
1/64..64, duration 0..60 seconds, and positions must be finite. Invalid arguments
fail without enqueueing. Queue overflow returns false. Smoothstep pan/zoom samples
fresh requests at time zero, holds its exact endpoint, supports instant cuts, and
can be interrupted from the previous successfully constrained unshaken view.

Authored follow/dead-zone history advances independently underneath transitions.
Stop returns to that current base. UI pause holds time; suspended callbacks cannot
deliver pending commands. Failed projection validation commits no follow/effect
clock state. Transitioned zoom drives final bounds, then shake, pixel snapping and
final clamping apply. Pixel-perfect zoom rounds to integers. Reset clears effects;
logical save files do not serialize transition history. No dependencies or new
render passes. Contract v9 tools rebuilt; an actual v8-runtime export rejects.

## Validation

- Full locked workspace: 891 Rust tests PASS (serial execution); 49 Python helper
  tests PASS. just check and strict default/opt-out WASM Clippy PASS. Node syntax
  and whitespace checks PASS.
- Unit tests cover fresh sampling, pause, exact endpoint/hold, cuts, interruption,
  invalid replacement, large finite interpolation, 60/120 Hz elapsed equivalence,
  follow isolation/cancel, zoom-aware bounds/pixel rounding, shake isolation and
  projection failure without clock advancement.
- Integration tests cover bounded transition commands, invalid arguments,
  failed/debug-suspended callback delivery, UI pause and script cancellation.
- `samples/camera_transition` pans to [80,40], zoom 2 over two seconds. A blue
  beacon at tick 150 prevents initial-frame acceptance; observers require multiple
  distinct green bounds and the expected final box. Gameplay Y grows upward.
  Initial native/browser checks used a y-down expected box and failed; corrected
  y-up checks passed on their second attempts. Original failure logs are retained.
- Cancellation variant stops at tick 90; interruption variant replaces the request
  at tick 60 with origin [32,16], zoom 1, duration 1. Both use the delayed beacon.
  Existing shake sample also passes with unchanged observer requirements.

| Scenario | Native / browser distinct bounds | Final green bounds | Green / blue pixels |
| --- | ---: | --- | ---: |
| Pan/zoom endpoint | 16 / 22 | [96,16,224,144] | 15869 / 256 |
| Cancellation | 12 / 15 | [128,88,192,152] | 4096 / 64 |
| Interruption | 16 / 22 | [96,104,160,168] | 4096 / 64 |

Distinct captured bounds depend on screenshot timing; thresholded green counts
include the renderer's filtering at scaled edges. Exact final bounds and delayed
blue beacon are required in every check.

Actual pixel receipts/screenshots live under target/a05-transition-{native,browser},
target/a05-transition-cancel-{native,browser},
target/a05-transition-interrupt-{native,browser} and
target/a05-transition-shake-{native,browser}. Replays run serially after builds,
with exports relocated, authoring copies hidden, native developer PATH removed,
and no browser page errors. Linux Xvfb/software graphics and Chromium/SwiftShader;
no physical device/DPI, production performance, split-input or release acceptance.

Raw logs target/a05-transition-{workspace,quality,wasm,wasm-optout,python,tools,web,
stale,precommit}.log. Previous v8 bundle: target/a05-transition-prior-web-runtime.
A05 remains PARTIAL; parallax, multiple views and remaining rendering/capacity
acceptance remain open. A03/A04 deferred gates stay deferred.
