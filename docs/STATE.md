# Kiln Session Handoff

Owner scope: A02–A04 on `fix/a01-ci-baseline`.
Current checkpoint: A03 reference-safe prefab joint propagation (2026-11-19),
following 0431883. A02–A04 remain PARTIAL. A01 accepted baseline stays 6de428b;
full 3D stays behind G2D. MIT OR Apache-2.0 settled; no package-size ceiling.

## Done

- ADR-0092: prefab Apply remaps instance-owner joint endpoints to the template;
  propagation remaps them to each destination owner. Changed external endpoints
  must resolve uniquely in the fully staged scene composition. Missing/ambiguous
  targets reject atomically; valid nested targets work and unchanged legacy joints
  do not block unrelated edits. No serialized format/dependency change.
- Apply now stages current captured unsaved scene edits before applying rather
  than reading an older scene document. The prefab panel stages its entire batch
  before publishing definitions/live updates and restores displayed definitions
  after rejection. History records only successful Apply operations.

- ADR-0091 fixes a reproduced explicit prefab texture clear being lost in JSON
  recovery and unsupported in sparse TOML overrides. Omitted texture inherits,
  a string sets the path, and false clears it. Legacy string/null behavior is
  preserved; true/invalid types are rejected. Optional inherited override fields
  are omitted. No new dependency or runtime component format.
- Headless editor integration captures a cleared instance against its textured
  definition, saves/reopens the complete project, writes/reads actual atomic
  recovery, moves typed resource paths, propagates across two scenes and applies
  another instance. The clear persists while inherited paths update. Final saved
  projects compare completely. No new GUI or native/browser visual claims.

- ADR-0090: bounded generated-atlas provenance allows explicit Open to reconstruct
  the saved recipe and Restore to reconstruct its selected autosave draft. Multiple
  interrupted builds/older drafts are supported. Source art and manifest stay intact;
  unknown output edits remain protected. Save compacts matching committed provenance.
- Save As reconstructs only its staged copy and reloads its normalized saved project.
  Native/browser export use copied validated provenance, rebuild only in staging,
  preserve source files/provenance, and strip private metadata before publication.
- Standard-library advisory publication locks release after crashes. Create-new
  temporary names skip orphan files without deleting them. Private bookkeeping is
  hidden from asset browsing/copying. No dependency added; ledger 512 KiB/4096 rows.
- Actual crash widgets exposed a Save regression during development: disk conflict
  checks used a loader with Open recovery side effects, reverting restored PNGs.
  Read-only load and explicit load_recovering now have separate contracts. Regression
  verifies pending atlas survives conflict checking and Save. Recovery refreshes
  scene textures; manual Save/Save As restarts the autosave interval.
- Previous 5bc6e0a: saved deterministic atlas recipes/static clips, stable asset IDs,
  source-move/rebuild/reopen, current-art exports and loaded-ID collision fix.
  836a2d8: source-module moves preserve imports/paths/IDs/clean open buffers.
  6ddcc44: whole-project/two-scene recovery oracle and all-12-component coverage.

## Executed validation

- Three prefab-joint before regressions fail, all six final regressions pass:
  Apply/propagation owner remapping, whole-project atomic missing/ambiguous target
  refusal, unique nested target, exact save/reopen/runtime spawn, legacy tolerance.
  Raw target/a03-prefab-joints-{before,after,integration,workspace,quality-final}.log.
  Evidence: docs/evidence/A03_PREFAB_JOINT_PROPAGATION.md. This checkpoint is
  headless model/runtime-spawn proof; no new GUI/export/physics visual claim.

- Explicit-clear controlled regression fails on previous serializer (None differs
  from Some(None)), passes now for all three states through TOML/JSON. Legacy null
  and malformed-type coverage passes. Raw logs:
  target/a03-prefab-texture-{before,core,integration,workspace,quality,wasm,wasm-optout}.log.
  Evidence: docs/evidence/A03_PREFAB_TEXTURE_PERSISTENCE.md.

- Current locked Rust workspace: 813 PASS, zero failures including docs. just check PASS.
  Previous shared-core checkpoint: default/opt-out WASM strict Clippy PASS.
  Previous Python helper suite 49 PASS;
  script compilation and diff checks PASS.
- Real native widgets PASS: force a crash before autosave, reconstruct saved atlas,
  republish, wait for actual timer snapshot, crash/Restore/Save/restart. Entire
  recovered project matches snapshot (typed f32 duration representation normalized),
  exact PNG bytes and rendered green pixels match, original source art/scripts
  remain unchanged, committed provenance is removed. Screenshots inspected.
- Unit/integration tests cover older JSON draft reconstruction, external/foreign
  output preservation, invalid ledger/active lock, orphan temp preservation, Open
  exact saved PNG, Save As source isolation, and read-only disk conflict checks.
- Native KAR/browser staging regression PASS after interrupted 32px generation:
  export uses saved 16px recipe/current blue art, preserves authoring files/ledger,
  and excludes private metadata. Final asset-browser exclusion regression PASS.
- Raw artifacts: target/a03-atlas-recovery-workspace.log,
  target/a03-atlas-recovery-{quality-final,export-final,private,python,integration}.log,
  target/a03-atlas-recovery-final-widget.log, target/a03-atlas-recovery-final/.
  Detailed evidence: docs/evidence/A03_ATLAS_CRASH_RECOVERY.md.
- Previous checkpoint: 797 Rust tests, default/opt-out WASM Clippy, saved-atlas widget
  workflow, relocated native/Chromium 1024 green pixels, and portable editor exports
  passed. See docs/evidence/A03_SAVED_ATLAS_RECIPES.md. Visual scope remains Linux
  Xvfb/software graphics; no physical GPU or other-OS certification.

## Next / known issues

Obtain matching hosted checks for the new commit. GitHub Actions lookup in this
environment returned Forbidden; local checks passed. Earlier 7f7c60e reference
44370197155, CI 44370197413 and Linux candidate 44370197855 passed; those checks
accept earlier source, not this checkpoint. Native CI now includes atlas crashes;
ordinary workspace tests include the prefab persistence integration.

A03: older snapshots that already lost an explicit clear cannot recover its intent.
Nested/variant prefab runtime/override contracts and combined renamed-art,
multiple-scene/prefab/atlas/export production acceptance remain open. Recipes own
whole-image static clips; independent animation edits need separate clip assets.
Derived-atlas reconstruction uses current source art, not historical source backups.
It is not a general multi-file transaction or power-loss durability guarantee.
General asset/source-module move crash atomicity remains open. Bounds are in
PROJECT_FORMAT/BUDGETS. Physical GPU/FPS/memory and cross-platform visuals remain.

A02: every-inspector-property combined widgets and cross-platform GUI remain open.
Scoped/repeated nested identities need a deliberate runtime/script contract;
ambiguous links are refused and authored data preserved. Joint form requires Apply;
inspector edits remain primary-entity scoped. Save As rescue requires source
resources and explicit Save/Revert of dirty scripts. Source can consume Ctrl+S;
explicit File / Save Project is available. Source moves require clean saved buffers
and bounded portable .ks paths; all-file crash atomicity remains separate.

A04: live gameplay source breakpoints, statement stepping, imported function frames/
locals/watches, safe reload specification and the enemy/projectile/trigger game
with equivalent graph and complete exports remain open. Cooperative task payloads
are explicit state machines, not suspended function stacks/yield syntax. The
separate bytecode debugger is not the live gameplay source debugger.

User requested finishing A02–A04; continue with validated checkpoints. The current
checkpoint is complete; the full three-milestone acceptance is still open.

## Tools / boundaries

Source /workspace/.kiln-tools/activate.sh (Rust 1.99.1, rootless libraries/software
Vulkan, CHROMIUM_PATH Debian Chromium 156). Run timed UI/gameplay checks serially;
concurrent build/export load previously exposed timing issues. Generated target
artifacts stay out of Git. Do not kill unrelated editor/Xvfb. just check before
every commit. No physical-GPU/audio/controller claims.
