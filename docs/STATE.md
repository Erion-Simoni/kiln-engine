# Kiln Session Handoff

**Active milestone:** A01 — Restore trustworthy CI and baseline (PARTIAL).
**Branch:** fix/a01-ci-baseline, including the unified plan commit 93b0ee0.
**Owner preferences:** no release-size ceiling; resource/performance limits remain.
MIT OR Apache-2.0 settled. Full 3D remains gated on G2D.

## Done

- Unified plan remains authoritative: docs/ENGINE_COMPLETION_PLAN.md.
- Reproduced and repaired mdBook failures: malformed link, duplicate chapters,
  missing introduction and nonexistent AI chapter. Pinned tested mdBook 0.4.52;
  generated docs/book is ignored.
- Reproduced stale saved-UI smoke interactions; repaired menu, label, drag and
  history positions without dropping assertions.
- Repaired native/browser Build/Export interactions and cancellation positions.
  All three repaired UI smokes pass with save/reopen and deliberate failure checks.
- Reference-game workflow now covers master/main pushes, relevant pull requests,
  manual dispatch and its original feature branch; removed path filters.
- Engine source unchanged. Evidence: docs/evidence/A01_CI_BASELINE.md.

## Validation

- just check passed; 700 Rust tests passed (0 skipped); 29 Python tests passed.
- Native feature opt-out checks and both default/feature-off WASM strict clippy passed.
- mdBook 0.4.52 build passed; all workflows passed actionlint 1.7.7 (shellcheck disabled).
- Saved-UI author/save/reopen/resume passed: [104,80] offset, health 66, one script start.
- Browser/native exports passed: saved inspector value 88, inventory checks,
  output preservation on deliberate failure and relocation. Native cancellation
  and staging cleanup passed. Raw local evidence is under target/a01-*.
- Xvfb/software graphics/null audio establish no physical-device acceptance.

## Known issues and next

1. A01 remains open. Retrieve authenticated detailed cargo-dist, release-candidate
   and reference-game failure logs; public step-log URLs return HTTP 404. The
   public run pages identify failing steps, not their complete error text.
2. Previous master ae492f1 hosted CI 43405013257 and release 43405013163 failed;
   reference games 43404644543/f7c8f9d failed. Windows candidate 43405013305 passed,
   which does not establish full Windows GUI acceptance. Fresh hosted acceptance
   of this revision and complete three-game replay/artifact verification are pending.
3. Installer workflow has no checked-in cargo-dist metadata. Confirm and repair
   its actual failure and intended contract; retain the separate portable path.
4. Proceed to A02 only after A01 acceptance: durable authoring, GUI breadth and
   unsaved script-buffer recovery. Later 2D scope includes richer tools, measured
   hardware budgets and independent tutorial/target acceptance; see unified plan.

No release gate is closed. No physical-device result or engine-parity percentage
is claimed. This session implements one A01 repair step, not the full roadmap.
