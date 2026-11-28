# A03 per-instance sprite geometry

2026-11-19, `fix/a01-ci-baseline`, following `cc3c971`. A03 stays PARTIAL.

Controlled regression: an instance captured at width 48, height 20 and layer 7
changed to 64/80/2 when another instance was applied to the definition. Its geometry
had no persisted override representation. The failing comparison is retained in
`target/a03-prefab-geometry-before.log`.

ADR-0093 adds sparse `SpriteGeometry` overrides for width, height and draw layer,
separate from existing Sprite color/texture and Transform overrides. Scene capture
records per-property differences and removes the geometry delta when all values
inherit again. Ordinary Apply, propagation and Revert use the shared override path;
live instance refresh applies the same properties. The prefab override panel can
edit existing geometry deltas. No new dependency or resource-reference format.

The new enum tag requires matching runtime parsers. Shared contract
`kiln-authoring-v3` rejects older `kiln-a04-v2` native/browser tools before export;
web metadata and player identity read the same constant. The prior contract is an
explicit compatibility negative control. This also prevents mixing tools that lack
the explicit texture-clear boolean introduced by ADR-0091. Rebuild matching native
and browser tools when updating an installed editor.

Three headless regressions cover:

- Capture, actual project Save/reopen and atomic JSON recovery-file serialization,
  cross-scene Apply, runtime projection and exact full-project restart. Per-instance
  geometry remains 48/20/7 while the definition changes to 64/80/2.
- Width-only sparse TOML persistence, inherited height/layer changes, and Revert
  as one history entry with exact Undo/Redo documents.
- Live linked-instance refresh preserving all overridden properties, then recapture
  removing geometry overrides after values return to the definition.

Raw evidence: `target/a03-prefab-geometry-{before,integration,workspace-final,quality-final,wasm-final,wasm-optout-final}.log`.
Ordinary CI runs the new integration tests. This proves editor-model/persistence and
runtime projection, not a new GUI/physical GPU/export visual replay. Material/clip
and arbitrary component overrides, nested/variant prefab semantics and full combined
multiple-scene atlas/export acceptance remain open. A02–A04 are not complete.

Final local validation: **816 Rust tests passed**, `just check` passed, and default/
opt-out WASM strict Clippy passed. Hosted results for this commit remain pending.
