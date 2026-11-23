# Kiln Session Handoff

Branch: fix/a01-ci-baseline. Current checkpoint: A04 shared gameplay instruction
budget, following bb337cb (2026-11-23). User requested larger fixes and a cumulative
plan report. A03 acceptance remains explicitly deferred by the user (2026-11-21).
A02/A03/A04 remain PARTIAL; A04 is active. A01 accepted baseline stays 6de428b.
Full 3D stays behind G2D. MIT OR Apache-2.0 settled; no package-size ceiling.

## Done

- ADR-0098 closes multiplication of per-invocation instruction allowances across
  scene/entity/lifecycle/saved-UI callbacks. One shared pool resets per public tick;
  explicit Stop gets a fresh bounded cleanup allowance. Existing individual VM
  one-million limit remains. Reserve/refund is per execution, not per opcode.
- Project script_instruction_budget defaults to 1,000,000 (omitted on save), accepts
  1..10,000,000 and validates before source reads. TOML/JSON persistence covered.
  Debugger shows used/limit and source/entity/phase errors identify exhaustion.
- Runtime contract is kiln-authoring-v5; stale v4 tools reject before export.
  Rebuild matching player/browser tools. No new crate or application dependency.
- Actual editor replay exercises failure/Pause, a real VM local watch and corrected
  saved-source Restart/Pause. The optional replay uses existing tesseract plus
  Xvfb/xdotool/ImageMagick; it is not wired as a new default CI dependency.
- Cumulative implemented work, checkpoint history and open gates are consolidated
  in docs/PLAN_PROGRESS.md. Earlier A04 work includes safe Restart, real allocated
  VM locals/data watches, shared scripting semantics and editable gameplay graphs.
  Earlier A03 includes atlas build/export/crash recovery, loaded IDs/source moves,
  prefab sprite overrides and reference-safe joint propagation. A02 authoring/
  recovery/property/reference work remains in its dated evidence and the plan.

## Executed validation

- Locked Rust workspace: 837 PASS, zero failures including docs. just check PASS.
  Default/opt-out WASM strict Clippy PASS. Python helper suite: 49 PASS; new native
  replay py_compile and git diff checks PASS.
- Controlled old-behavior test fails: four finite scripts exceed the intended tick
  allowance without stopping. Six final runtime cases PASS across root/entities,
  signal fan-out, collision callbacks, paused UI, reset over 20 ticks, bounded Stop,
  configurable work, persistence and rejection before source reads. Two VM tests
  verify exact zero/partial/error accounting and retained per-call limits.
- Actual Linux Xvfb/software editor replay PASS: 1000/1000 error/Pause, live watch
  I64(9), corrected saved-source Restart/Pause, I64(7) and usage 5/1000. Manifest
  bytes unchanged; screenshots inspected. Two initial OCR target observer failures
  were fixed using observed normalized field positions, unchanged value assertions.
- Raw: target/a04-shared-budget-{before,after,vm,integration,workspace,quality-final,
  wasm,wasm-optout,python,editor-build}.log and
  target/a04-shared-budget-widgets-verified/{result.json,ocr-latest.txt,editor/}.
  Evidence: docs/evidence/A04_SHARED_SCRIPT_BUDGET.md.
- Hosted checks for new source remain pending. Earlier hosted references accept
  earlier source only; do not substitute those for current acceptance.

## Next / known issues

- Continue A04: actual source breakpoints, statement-level continuation, active
  imported frames/scopes and expression watches; required enemy/projectile/trigger
  game with equivalent graph and full native/browser/export/debug acceptance.
  Current locals are allocated slots, not active lexical frames. Watches are
  bounded data selectors, not arbitrary expression evaluation. Cooperative tasks
  are explicit state machines, not suspended function stacks.
- Shared work is bytecode counting, not host-call/render/physics wall time. Prior
  completed callbacks may already have changed world/state; a failed tick is not
  rolled back. Physical frame-time/GPU/device certification remains separate.
- A03 stays deferred: nested/variant prefab identity/runtime contracts, arbitrary
  component overrides, general multi-file source/asset crash transactions, and
  combined prefab/multi-scene/atlas/export visual production acceptance remain open.
  Atlas recovery regenerates from current art, not historical art backups. Old
  snapshots that already lost an explicit texture clear cannot recover that intent.
- A02: combined every-inspector-property widget and cross-platform GUI acceptance
  remain open. Scoped/repeated identities require deliberate contracts; ambiguous
  links are refused while preserving data. Save As needs resources and saved/reverted
  script drafts. Explicit File / Save Project avoids source-editor Ctrl+S focus.

## Tools / boundaries

Source /workspace/.kiln-tools/activate.sh for Rust 1.99.1, rootless libraries/software
Vulkan and Debian Chromium. Run timed UI/gameplay checks serially, without concurrent
build/export load. Optional actual budget replay:
python3 tests/native/shared_script_budget_smoke.py.
Generated target artifacts stay out of Git. Do not kill unrelated editor/Xvfb.
just check before every commit. No physical GPU/audio/controller claims.
