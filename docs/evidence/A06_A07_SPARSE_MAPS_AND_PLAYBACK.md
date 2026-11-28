# A06 sparse maps and A07 playback — 2026-11-27

User explicitly authorized progression through A07, overriding the repository's
single-milestone session restriction. A05 remains PARTIAL; deferred A03/A04 gates
were not retried. The environment execution connection dropped during A05
checkpointing, then resumed with edits intact. Missing commit was verified and
completed as 898ba25. A06 fixes were committed as 54521e7 before A07 work.

## A06: stored-chunk query bounds

ADR-0111 replaces coordinate-span render/collision loops with stored intersecting
chunk traversal. Huge finite rectangles no longer enumerate billions of empty
cells/chunks. Shared f64 chunk bounds reject invalid rectangles, tile scales and
offsets. Wide tile arithmetic avoids multiplication overflow. Chunk draw order is
deterministic; first collision preserves layer order and global tile row priority.
Map extent reporting saturates; oversized atlas image dimensions reject UVs.
Malformed chunk keys and unsupported negative/out-of-range coordinates are ignored.
This is bounded sparse traversal, not streaming, stamps or a spatial index.

Core/runtime tests include full finite-world queries, tile coordinate
2,000,000,000, row precedence across chunks, offset collision, exact edge contact,
bad geometry, map extent and atlas overflow. An initial chunk-edge culling mismatch
was corrected before final tests. Existing culling/collision regressions pass.
`samples/sparse_tilemap` has one visible green tile plus a far tile in another
stored chunk. Native/browser exports both render 4096 green pixels at exact bounds
[128,112,192,176]. Huge query completion is tested at the shared code boundary;
the actual pixel fixture uses a normal camera. Full A06 editor-level acceptance,
reusable stamps, streaming and further tile asset validation remain open.

## A07: ping-pong/speed and consistent clock

ADR-0112 adds defaulted Clip.ping_pong=false and speed=1, validated finite 0..16.
Editor controls participate in existing history. Looping ping-pong reverses through
interior frames without duplicating endpoints; non-looping playback returns to
frame zero before hold/next. f64 duration/sample/event/seek calculations share the
same frame iterator. Runtime and authored speed multiply; transition leftovers
remain real time and use the next clip's speed. Zero dt or either zero speed emits
no initial/transition-entry events. Seek suppresses historical event dispatch.

Tests cover forward/reverse frame boundaries, defaults/JSON roundtrip, invalid
speed, initial pause/zero-dt silence, exact five-event return, hold, seek,
transition between different speeds, logical snapshot/restore and nondefault
editor control undo/redo. Total duration must remain float-representable for
legacy authoring accessors. Existing default clip and runtime tests pass.

`samples/ping_pong` uses one-shot speed 2, with events zero/one/two/one/zero.
Attached script validates event order/count, checks at least four events by tick
15, and turns green only after tick 90 if all checks passed. Ignoring either
ping-pong or speed cannot satisfy the gate. Native/browser exports both render
4096 green pixels at [128,88,192,152]. Controls/data travel through real project
load, export, runtime animation and script callbacks. No new OS widget-drag
acceptance is claimed. State graphs, arbitrary tracks/curves, rigs/IK and full A07
acceptance remain open. Existing 4096-cycle update cap is retained; extreme
cycle/event overload behavior and physical performance are not newly accepted.

## Executed validation

- Full locked workspace 907 Rust PASS (serial), Python helper suite 49 PASS.
- just check, strict default/opt-out WASM Clippy, editor history test, Node syntax,
  Python helper compilation and whitespace PASS.
- Six serial native/browser replays after all builds: sparse map, ping-pong and
  existing five-phase clipped-pointer regression. Exports relocated; authoring
  copies hidden; native developer PATH empty; browser reports no page errors.
- Matching editor/CLI/player/prepared browser rebuilt, contract v12. Actual export
  using previous v11 browser bundle rejects as incompatible. New clip fields are
  defaulted for old project/snapshot data; no dependency, VM-memory or save-layout
  change. Source PNG fixture texels are generated assets, not external art.

Logs: target/a06-sparse-{core-final,runtime-final,quality}.log and
 target/a07-{clips,playback,workspace,quality,wasm,wasm-optout,editor-controls,tools,
web,python,stale,precommit}.log. Receipts/screenshots:
target/a07-{sparse_tilemap,ping_pong}-{native,browser},
target/a07-pointer-{native,browser}. Previous SDK: target/a07-prior-web-runtime.
Linux Xvfb/software graphics and Chromium/SwiftShader only. No physical DPI/device,
production performance, full milestone completion, release or 3D claim.
