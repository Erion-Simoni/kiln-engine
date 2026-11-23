# A04 aggregate gameplay execution budget

2026-11-23, `fix/a01-ci-baseline`, following `bb337cb`. A03 remains deferred by user.

The controlled previous-behavior regression runs four finite, individually bounded
entity programs. All finish despite their aggregate work exceeding the intended
one-million tick allowance. The pre-fix assertion fails in
`target/a04-shared-budget-before.log`. This is a cross-session scheduler defect,
not a missing per-loop cap.

ADR-0098 adds a shared instruction pool across the root and all attachment VMs.
Each invocation reserves remaining allowance once, runs within both outer and VM
limits, then refunds unused work on success or error. VM accounting includes the
failing instruction, leaves configuration unchanged, and charges nothing when no
instruction can execute. No atomic operation is added to each bytecode dispatch.
The public gameplay step resets once before saved-UI processing. All Start/Update,
signal/collision/animation/UI callbacks share the pool. Explicit Stop receives a
fresh bounded cleanup allowance. Saved counters are not restored as gameplay state.

Project script_instruction_budget defaults to 1,000,000 (omitted in saved default
projects) and accepts 1..10,000,000. Validate before source reads. Individual VM
invocations still cap at one million. Debugger displays aggregate used/limit;
existing diagnostics retain source/entity/phase and live locals/watches. The native/
web compatibility contract is v5, rejecting v4 tools that cannot enforce the policy.
No dependency was added. Successful prior callbacks are not rolled back and this
is not a host-call cost meter or physical frame-time claim.

Executed coverage:

- Original multiplying entity programs now stop within one shared allowance.
- Combined root/entity work fails at 2000 with source/entity context, succeeds at
  10000 without changing engine code; all successful attachment state is verified.
- Allowance resets over 20 ticks; per-call one-million guard still applies when
  aggregate configuration is larger. Reported consumption never exceeds its cap.
- Signal fan-out, collision callbacks and paused saved-UI callbacks cannot multiply
  allowance. Errors preserve useful lifecycle context and future ticks can proceed.
- Stop cleanup gets fresh allowance after exhaustion but is itself bounded.
- Configuration TOML/JSON/default persistence and invalid values rejected before
  source reads. Existing native/web protocol test rejects stale v4 identities.
- VM tests verify zero/partial allowance, actual host-call counts, restoring normal
  allowance, individual limit retention and instruction charges on division error.

Raw: `target/a04-shared-budget-{before,after,vm,integration,workspace,quality,wasm,wasm-optout}.log`.
The optional actual-widget replay also passes: launch a copied project with a
1000-instruction limit, observe shared exhaustion/automatic Pause and counter
1000/1000, add a real local watch returning I64(9), correct the saved script, Restart,
Pause and observe I64(7) with usage 5/1000 and no error. Manifest bytes stay unchanged.
Screenshots were inspected. Raw receipt/screenshots/OCR are in
`target/a04-shared-budget-widgets-verified/` and its sibling `.log`. Two initial
attempts hit OCR target ambiguity (faint placeholder and repeated scene text);
normalized field coordinates fix the observer without changing runtime assertions.
The replay requires existing tesseract/Xvfb/xdotool/ImageMagick and is opt-in, not a
new default CI/tool installation. Run `python3 tests/native/shared_script_budget_smoke.py`.

This is runtime/VM, saved-UI scheduling and actual Linux software-rendered editor
proof. Physical device/frame-time and new browser-runtime certification remain
separate. Source breakpoints, statement stepping, active frames and full equivalent-
game acceptance remain open. A04 remains PARTIAL.

Final local validation: **837 Rust tests passed**, `just check` and default/opt-out
WASM strict Clippy passed; Python helper suite 49 passed; actual budget/watch/restart
widgets passed. Hosted results
for this new source remain pending.
