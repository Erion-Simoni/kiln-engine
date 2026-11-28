# A04 actual gameplay VM local inspection

2026-11-23, `fix/a01-ci-baseline`, following `b186177`. A03 stays deferred by user;
A04 remains PARTIAL. ADR-0096 extends live read-only inspection without presenting
the separate bytecode debugger as gameplay debugging.

Compiler allocation records local slot names as nonserialized Program metadata.
The runtime snapshots the actual scene/attachment VMs: owner, instruction, mapped
source origin, allocated slot/name and bounded value preview. Editor Debugger shows
these while paused or after errors under Inspect live state and locals. It retains
existing persistent entity-state inspection. Duplicate/inlined names are separated
by slots; no active lexical lifetime or true call-frame claim. Reading a snapshot
never steps the VM, executes a host call or edits project/runtime values.

Bounds: 64 sessions, 128 locals per session, omitted-local count, 128-character
string previews and container counts without cloning/traversing their contents.
This is metadata/inspection only; no dependency, execution behavior, serialized
project/save format or runtime contract change. No expression evaluation or watches.

Executed tests verify:

- Imported function division failure maps to logic.ks:4 and shows parameter 3,
  computed local 7 and denominator 0 from the executing VM. Repeated snapshots
  preserve identical values.
- A runaway loop stops at the existing instruction budget, retaining its local 9.
  A 160-local program exposes 128 slots and reports 32 omitted.
- Two attached instances expose distinct actual property-derived values 1 and 3.
  The initial fixture omitted ECS entities, so the runtime correctly skipped the
  scripts. Adding matching runtime owners makes the unchanged assertions pass.
- Long Unicode string previews truncate; container previews summarize counts.

Raw evidence: `target/a04-live-locals-{preview,integration,integration-final,workspace,quality,wasm,wasm-optout}.log`.
Ordinary CI runs these runtime regressions. This is headless executing-VM proof and
compiled UI wiring, not an actual GUI replay. Live source breakpoints, statement
stepping, active frames/scopes/watches and the equivalent script/graph acceptance
game remain open. No full A04 acceptance or matching hosted result is claimed.

Final local validation: **826 Rust tests passed**, `just check` and default/opt-out
WASM strict Clippy passed. Matching hosted confirmation remains pending.
