# A03 saved atlas recipes and stable asset allocation

2026-11-18, branch `fix/a01-ci-baseline`, following `836a2d8`.
A03 remains PARTIAL; this completes a focused atlas build/rebuild/export checkpoint.

## Behavior

ADR-0089 adds optional saved `Project.atlases` recipes: output PNG, size/padding,
stable source labels/paths and generated hash/regions. Atlas Builder is available
beside the Sprite Editor heading, with saved-recipe loading and bounded scrolling.
A recipe generates ordinary PNG pixels and named single-frame clips, using the
existing renderer/animation player on native and browser runtimes. Authors assign
those clips explicitly; existing scenes and prefab references are preserved.

Packing follows labels, so moving art through Assets updates the recipe's source
path while retaining generated clip identities and regions. Rebuild updates pixels
and forces scene-texture refresh even when clip geometry is unchanged. The output
identity and import presets persist. Inputs, output ownership and generated-clip
ownership are checked before replacement. External edits, unrelated collisions,
unsafe/symlink paths, invalid/oversized images, failed fit and removal of referenced
clips preserve the prior project/output. Export rebuilds only its staging snapshot
from current source images and updates generated clips before validation/packing.
The authoring project, saved atlas and source art stay unchanged during export.

Review also exposed a pre-existing allocator defect: numeric asset IDs could be
reused after loading a project because the process counter restarted independently
of persisted IDs. A regression with the extracted previous allocation behavior
fails. The shared batched allocator now reserves existing IDs and orphaned default/
platform import presets, preserves input order and returns the same ID for repeated
paths. Both ordinary editor imports and atlas outputs use it. Existing duplicate
IDs are retained for diagnostics rather than silently changing authored identities.

## Executed validation

- Full locked Rust workspace: **797 passed**, zero failures including docs.
  `just check` passes format, strict workspace Clippy and workspace/audio checks.
  Both default and feature-opt-out WASM Clippy pass with warnings denied. Python:
  49 tests pass; Python compilation, Node syntax and whitespace checks pass.
- Five atlas-project regressions cover deterministic pixels/regions, renamed source
  references and IDs, complete TOML roundtrip, external/clip-edit preservation,
  referenced-clip removal, collision/fit/path rejection, extreme PNG dimensions and
  symlink rejection. Output pixels are checked against independently chosen colors.
- The saved-ID regression fails with the previous blind-counter allocation and
  passes with namespace reservation. Raw controls: `target/a03-atlas-identity-before.log`
  and `target/a03-atlas-identity-after.log`.
- Real native widgets build a recipe, assign its generated clip to a sprite, show
  red pixels, rebuild blue art, move the source through Assets, retain its ID and
  label, rebuild green art, then reopen/rebuild and compare the complete project
  and unchanged script bytes. The atlas/output ID and generated regions remain exact.
- A prior harness attempt resized an already enlarged panel; its screenshot exposed
  the setup error. Another caught a missed first-field focus; the saved-property
  assertion failed while only the animation field changed. The final test observes
  painted reference fields and uses held clicks. Pixel classification hides selection
  gizmos: another setup assertion had counted a blue scale handle while the atlas
  itself already rendered green. Assertions and wait budgets remain unchanged.
- A verified portable installation creates native and browser exports through the
  actual Build / Export widgets, verifies bundled notices and refuses overwrites.
  Both relocated artifacts display **1,024 green pixels** from the generated clip
  in the native player and actual Chromium. Screenshots were inspected. Authoring
  copies are hidden; development tools are absent from the native player's PATH.
- A separate CLI regression changes source pixels after saving the atlas, verifies
  both native/KAR and browser export staging contain rebuilt blue pixels and updated
  recipe hashes/regions, and proves source manifest/atlas bytes stay unchanged.
  Its handshake/browser-module fixtures validate export transactions, not gameplay.
- CI runs atlas widgets/native playback and separately atlas widgets/Chromium,
  retaining narrow screenshots, logs and results. Matching hosted results remain
  required. Local rendering uses Xvfb/software graphics and Chromium/SwiftShader.

Raw local evidence: `target/a03-atlas-workspace-final.log`,
`target/a03-atlas-quality-final.log`, `target/a03-atlas-current-*`,
`target/a03-atlas-recipe`, `target/a03-atlas-recipe-renamed`,
`target/a03-atlas-editor-export`, `target/a03-atlas-player`,
`target/a03-atlas-browser`, and `target/a03-atlas-export-tests.log`.
Setup/focus diagnostics are preserved in `target/a03-atlas-widget-first` and
`target/a03-atlas-focus-before` and `target/a03-atlas-gizmo-before`.
Generated captures/binaries stay outside Git.

## Limits and remaining work

Recipes produce static whole-image clips, with stable labels and deterministic
packing. They do not automatically convert existing sprites, inherit custom clip
edits, or solve nested prefab/variant propagation. Independent animation clips can
be authored separately. The schema defaults preserve older projects without recipes.

PNG replacement and subsequent manifest Save are separate operations. A crash in
between can leave generated output newer than the saved recipe hash; choose a new
output path or restore the prior generated file before rebuilding. General crash
atomicity for multi-file asset/source operations remains open. Bounds are documented
in PROJECT_FORMAT.md/BUDGETS.md; physical GPU/FPS/memory certification is separate.

Full nested prefab/variant production, combined renamed-art/prefab/override/multiple-
scene acceptance, broader A02 property/recovery coverage and A04 live source debugger/
equivalent graph-game acceptance remain open. This evidence does not close those gates.
