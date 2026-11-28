# A05 camera shake — 2026-11-25

ADR-0107 adds `camera_shake(x,y,seconds,hertz,seed)` and `camera_shake_stop()`.
The callback queue is shared and capped at 32, returning false on overflow.
Validation precedes enqueue; effects commit only with successful callbacks.
Debugger-suspended ticks cannot drain effects or advance the adapter. The mixer
holds 32 seeded oscillators with quadratic decay, replacing the oldest on active
overflow. It samples newly delivered requests at time zero. UI pause holds time.
Offsets apply to the unshaken follow origin before pixel snapping/final bounds;
failed camera validation cannot advance the clock. Reset clears transient state;
logical save files do not serialize shake history.

## Executed checks

- Full locked workspace: 885 Rust tests PASS with serial test execution. Initial
  concurrent test execution hit an atlas publication lock conflict; serial run
  passed. An incorrect namespace in the added pause test was corrected before this
  final run. 49 Python helper tests PASS.
- `just check`, strict default/opt-out WASM Clippy, Node syntax, Python bytecode compilation and
  whitespace checks PASS. Matching native tools/browser runtime rebuilt, contract
  v8. An actual export with the previous v7 browser runtime rejects as incompatible.
- Integration coverage: scene/40 attached scripts share one request cap; pending
  callback requests cap at 32; invalid arguments queue nothing; source suspension
  and failed callbacks deliver no uncommitted effects; shared adapter applies
  shake, holds time under authored UI pause, and delivers script cancellation.
  Unit coverage includes deterministic seeds, decay/expiry, active replacement,
  amplitude bounds, follow isolation and failed-update clock recovery.

## Actual pixel replays

Source-only `samples/camera_shake`: green 64×64 marker, 24×16 amplitude,
2-second shake, seed 17. The beacon turns blue at tick 150. Observers require at
least two distinct marker bounds, then exact rest bounds `[128,88,192,152]` and a
blue beacon. An ignored shake or initial resting frame cannot satisfy the gate.
Cancellation variant requests 10 seconds, cancels at tick 90 and enables the
beacon at tick 120. Replays ran serially after builds.

| Scenario | Native distinct bounds | Browser distinct bounds | Final green / blue pixels |
| --- | ---: | ---: | ---: |
| Expiry | 12 | 13 | 4096 / 64 |
| Cancellation | 14 | 17 | 4096 / 64 |

Native runs relocate exports, hide the source and remove developer tools from the
player PATH. Browser runs relocate under a nested URL and report no page errors.
Existing no-shake dead-zone sample also passes native/browser exact bounds and
4096 green pixels. Default observer flags remain compatible. CI includes the
expiry motion/rest fixture. Counts of sampled bounds depend on capture timing.

Raw receipts/screenshots: `target/a05-shake-{native,browser}/`,
`target/a05-shake-cancel-{native,browser}/`, and
`target/a05-shake-default-{native,browser}/`. Logs:
`target/a05-shake-{workspace-final,quality,wasm,python,tools,web,stale,precommit}.log`.
Linux Xvfb/software graphics and Chromium/SwiftShader only; physical DPI/device,
performance acceptance, transitions, parallax and split-view routing remain open.
A03/A04 deferred gates remain deferred. A05 remains PARTIAL.
