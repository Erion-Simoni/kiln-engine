# A03 explicit prefab texture clears

2026-11-19, branch `fix/a01-ci-baseline`, following `c58b5ef`. A03 stays PARTIAL.

Reproduced an explicit texture clear (`Some(None)`) becoming an inherited override
(`None`) after JSON persistence. The controlled failing regression is retained in
`target/a03-prefab-texture-before.log`. Direct TOML sparse override serialization
also rejected empty optional fields. A clear in memory therefore lacked a reliable
saved/recovery representation and could pick up a texture again on propagation.

ADR-0091 keeps the Rust API and existing string overrides. Omitted means inherit,
a string sets the texture, and boolean false explicitly clears it. Legacy JSON
null keeps its former inherit behavior. Invalid true/numeric/list/object values
are rejected. Optional transform/color fields omit empty inherited values. No new
dependency or runtime component format was introduced.

Core regression covers all three states through JSON and TOML with sparse color
metadata, legacy null and invalid inputs. Editor integration captures an actual
projected textureless instance against a textured definition, saves/reopens the
complete project, writes/reads the actual atomic recovery file, moves the typed
asset reference, propagates across two scenes and applies another instance to its
definition. The cleared instance remains textureless, the inherited instance gets
the moved texture, and the final saved/reopened project matches completely. Both
project and recovery paths execute the same assertions. This is headless persistence
and editor-command proof, not new GUI/native/browser visual certification or the
full nested/variant prefab acceptance gate.

Raw evidence: `target/a03-prefab-texture-{before,core,integration,workspace,quality,wasm,wasm-optout}.log`.
The locked workspace and quality checks include this regression in ordinary CI;
hosted confirmation remains pending. Broader A02–A04 gates remain in ENGINE_COMPLETION_PLAN.

Validation: **807 Rust tests passed**, `just check` passed, default/opt-out WASM
strict Clippy passed. GitHub Actions lookup returned Forbidden in this environment;
no matching hosted-check result is claimed.
