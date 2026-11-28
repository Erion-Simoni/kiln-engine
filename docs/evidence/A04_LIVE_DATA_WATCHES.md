# A04 read-only watches on executing gameplay VMs

2026-11-23, `fix/a01-ci-baseline`, following `465328d`. A03 stays deferred by user;
A04 remains PARTIAL. ADR-0097 adds data watches to actual gameplay VMs.

Owner selects scene or entity:<stable-id>. Selectors read slot:number,
local:unique-allocated-name or state:key and optionally descend through slash-
separated array indices/map keys. ~0/~1 escapes retain literal tilde/slash keys.
Duplicate owners/names are rejected instead of selecting arbitrary instances/scopes.
Missing/scalar/out-of-range paths return diagnostics. No parser/compiler/evaluator
or host call executes while reading watches; repeated reads preserve local/state
values. Existing bounded previews summarize containers and truncate strings.

Debugger live controls add/remove up to 32 watches, evaluated only while paused
or errored. Owner is limited to 256 bytes, selector to 512 bytes and path to 16
components including root. Watches can read slots beyond the 128-local snapshot
preview without expanding whole VM snapshots. Editor-only watch state is not
project/save data. No dependency, execution, serialized format or runtime contract
change. Arbitrary expressions and active lexical frames remain unsupported.

Three headless regressions cover nested dictionary/array reads with escaped keys,
slot 159 beyond snapshot preview, distinct attached VM values, duplicate inlined
parameter-name rejection and exact slot selection, malformed/missing/out-of-range
paths and input bounds. Host-call-like input rejects and repeated snapshots/state
values remain unchanged. The initial ambiguity fixture redeclared a local in a way
the language disallows; it was replaced by two valid inlined function calls with
the same parameter name. Assertions were not relaxed.

Raw evidence: `target/a04-live-watches-{check,integration,integration-final,workspace-final,quality-final,wasm,wasm-optout}.log`.
Ordinary CI includes the runtime regressions. UI is compiled, not an actual GUI
replay. Source breakpoints, statement stepping, active-frame expressions and the
new equivalent script/graph acceptance game remain open; this is not A04 closure.

Final local validation: **829 Rust tests passed**, `just check` and default/opt-out
WASM strict Clippy passed. Hosted confirmation for this commit remains pending.
