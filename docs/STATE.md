# Kiln Session Handoff

Owner scope: A02–A04 on `fix/a01-ci-baseline`.
Current focus: A03 saved atlas recipes / loaded asset identities (2026-11-18),
following 836a2d8. A02–A04 remain PARTIAL. A01 accepted baseline stays 6de428b;
full 3D behind G2D. MIT OR Apache-2.0 settled; no package-size ceiling.

## Done

- Optional saved atlas recipes bake deterministic PNGs and named static clips.
  Sprite Editor's Atlas Builder loads/builds/rebuilds recipes. Source asset moves
  update recipe paths while retaining labels, clip IDs, regions and output identity.
  Rebuild explicitly refreshes scene textures, including unchanged clip geometry.
- Guarded PNG replacement and generated-clip ownership preserve external edits,
  unrelated files/clips, used clip references and prior output on rejected builds.
  Export rebuilds only its staging snapshot from current art; source files remain.
- Fixed a reproduced pre-existing loaded-ID collision: ordinary imports and atlas
  output allocation now reserve persisted identities and orphaned import presets
  through one batched allocator. Existing authored duplicates remain diagnostic data.
- ADR-0089, project-format/budget docs and CI cover the new recipe workflow.
  Evidence: docs/evidence/A03_SAVED_ATLAS_RECIPES.md.
- 836a2d8 source-module moves preserve incoming/outgoing imports, manifest paths,
  stable IDs and open source buffers; dirty source blocks moves. 6ddcc44 strengthens
  independent whole-project/two-scene recovery and all-12-component headless oracles.

## Executed validation

- Full cargo test --workspace --locked: 797 PASS, zero failures including docs.
  just check PASS. Default/feature-opt-out WASM strict Clippy PASS. Python 49 PASS;
  py_compile, Node syntax and diff checks PASS.
- Saved-ID regression fails with extracted blind-counter behavior, passes now;
  defaults/platform-only orphan presets and repeated input identities are preserved.
- Current-source actual widgets PASS: build, assign clip, red/blue/green scene pixels,
  source move, stable source/output IDs and regions, reopen/load/rebuild, complete
  project equality and unchanged scripts. Selection gizmos are hidden for pixels;
  painted-field waits and held clicks prevent missed focus. No relaxed assertions.
- Current-source relocated native and actual Chromium playback both PASS with
  1024 expected green pixels. Authoring copies hidden; native child PATH has no
  development tools. Screenshots inspected. Xvfb/SwiftShader/software graphics.
- Portable installed editor also passed actual native/browser Build / Export,
  bundled notice verification and overwrite refusal. Separate CLI native/KAR and
  browser staging regression rebuilds changed source art while preserving authoring
  manifest/atlas bytes. Its handshake/browser-module fixtures are transaction proof.
- Raw: target/a03-atlas-workspace-final.log, target/a03-atlas-quality-final.log,
  target/a03-atlas-identity-{before,after}.log, target/a03-atlas-current-*,
  target/a03-atlas-recipe{,-renamed}, target/a03-atlas-editor-export,
  target/a03-atlas-player and target/a03-atlas-browser. Setup diagnostics retained
  separately in a03-atlas-widget-first, a03-atlas-focus-before, a03-atlas-gizmo-before.

## Next / known issues

Obtain matching hosted checks for the new source. 7f7c60e hosted reference
44370197155, CI 44370197413 and Linux candidate 44370197855 all passed; these
accept the earlier panel repair, not later production changes.

A03: full nested prefabs/variants and combined renamed-art/prefab/override/multiple-
scene production acceptance remain open. Recipes generate whole-image static clips;
independent clip animation edits are separate assets. PNG replacement and manifest
Save are separate: a crash between them can leave a newer output than the saved
recipe hash, requiring a new output path or restoration before rebuild. General
multi-file asset/source crash atomicity remains open. Bounds are in PROJECT_FORMAT
and BUDGETS; no physical GPU/FPS/memory or cross-platform visual certification.

A02: every-inspector-property combined widgets and cross-platform GUI remain open.
Scoped/repeated nested identities need a deliberate runtime/script contract;
ambiguous links are refused and authored data preserved. Joint form requires Apply;
inspector edits remain primary-entity scoped. Save As rescue needs source resources
and explicit Save/Revert of dirty scripts. Source can consume Ctrl+S; explicit
File / Save Project is available. Source moves require clean saved buffers and
bounded portable .ks paths; all-file crash atomicity remains separate.

A04: actual live source breakpoints, statement stepping, imported function frames/
locals/watches, safe reload specification and the new enemy/projectile/trigger game
with equivalent graph and complete exports remain open. Cooperative task payloads
are explicit state machines, not automatic suspended function stacks/yield syntax.
The separate bytecode debugger is not the live gameplay source debugger.

User requested finishing A02–A04; continue in separately validated checkpoints.

## Tools / boundaries

Source /workspace/.kiln-tools/activate.sh (Rust 1.99.1, rootless libraries/software
Vulkan, CHROMIUM_PATH Debian Chromium 156). Run timed UI/gameplay checks serially;
concurrent build/export load previously exposed graph/physics timing issues.
Generated target artifacts stay out of Git. Do not kill unrelated editor/Xvfb.
just check before every commit. No physical-GPU/audio/controller claims.
