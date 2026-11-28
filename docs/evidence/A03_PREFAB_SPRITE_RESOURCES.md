# A03 prefab material and animation-clip overrides

2026-11-20, `fix/a01-ci-baseline`, following `cebb055`. A03 stays PARTIAL.

The controlled pre-fix regression captures an instance with material/clip cleared,
saves/reopens it and applies another scene's instance. The clear is lost and becomes
new-base/new-idle. Raw failing evidence: `target/a03-prefab-resources-before.log`.

ADR-0094 adds sparse `SpriteResources` overrides for material and animation_clip.
Each field independently inherits when absent, assigns a named resource when a
string, and clears when false. The existing nullable-string adapter handles both
TOML project files and JSON recovery; true/invalid value types remain rejected.
Capture records differences and removes the delta when both fields match the
current definition. Core Apply/propagation/Revert and live instance refresh honor
the same values. Live refresh also copies inherited material/clip from the definition,
which it previously omitted. Existing override panel entries show their state and
permit editing assigned IDs. Ordinary Sprite inspector edits produce these deltas.

Atlas removal guards now scan animation-clip overrides in scene and prefab links,
including dormant references with no current Sprite component. Removing a referenced
generated clip rejects without changing project or PNG bytes; removing its final
reference permits a rebuild. Named material/clip IDs are not filesystem paths.
No dependency or file-resource visitor was added.

Runtime contract advances to `kiln-authoring-v4`, with v3 an explicit negative
compatibility control. Rebuild matching native/browser tools with updated editors;
older parsers cannot read the new override tag and are rejected before export.

Executed tests cover all three states through TOML/JSON, malformed values, actual
atomic recovery files, complete saved-project reopen, cross-scene Apply, sparse
independent inheritance, live refresh, Revert as one undoable command with exact
Undo/Redo documents, and dormant generated-clip reference protection. Raw logs:
`target/a03-prefab-resources-{before,after,integration,atlas,workspace,quality,wasm,wasm-optout}.log`.

These are headless editor-model/persistence/runtime-component checks, not a new
GUI/material shader/animation playback/export visual replay. Arbitrary component
overrides, scoped nested prefab identities, variants and combined multi-scene
prefab/atlas/export visual acceptance remain open. A02–A04 are not complete.

Final local validation: **820 Rust tests passed**, `just check` and default/opt-out
WASM strict Clippy passed. Matching hosted results remain pending.
