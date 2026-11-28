# Kiln Session Handoff

Branch fix/a01-ci-baseline. Current A07 preview/cues/blending/cutout-rig checkpoint,
2026-11-28, following 15e3dbc. User explicitly requested those four feature areas.
A05/A06/A07 remain PARTIAL; do not advance to A08 unasked. At most three focused
attempts per blocked check; record and move on. A03/A04 gates remain deferred.
Full 3D remains behind G2D.

## Done

- ADR-0117: draw-only Scene View preview samples composer tracks/blends/IK, restores
  local/global transforms and tint after drawing and unwind, and never mutates
  authoring/saves. Hidden composer tabs keep preview playback advancing. Scope
  changes clear draft. Scene gizmos are inactive; Stop Scene Preview exits.
- Audio/signal cues: chronological, at most 32, saved once-per-playthrough latches,
  silent seek, pause, restart rearming. Signals use next-tick entity callbacks.
  Sound names validate against project SFX assets. queue_sound (141,1) commits
  after successful callback; failed/debug-suspended callbacks do not start audio.
- Weighted two-take property/pose animation and optional saved game variable for
  live weight. Preview uses authored weight. Local rotations stay unwrapped;
  positive XY scale/tint/position obey existing bounds. Shared 16 tracks/64 keys.
- Custom cutout skeletal workflow supersedes ADR-0022 deferral: bone hierarchy,
  rigid single-bone sprite skin attachments (weight 1), bone-local FK animation,
  root target tracks and weighted two-bone IK. Create Two-Bone Cutout Rig is one
  undoable scene transaction creating bones, skins and target. Inspector/hierarchy
  edit existing transforms/textures/parents; project and logical saves retain them.
- solve_ik_2d (140,7): world target, bend +/-1, IK/FK weight 0..1. +X rest links,
  directly parented bones and positive uniform scale ancestry required. Both bends,
  unreachable-target clamp, rotated/scaled parents and zero-distance target work.
  Physics-owned/unsupported chains reject without partial local-pose changes.
- Runtime contract v16; native tools and browser SDK rebuilt. No dependency or VM
  layout change. Legacy unextended composer canonical source unchanged.
- samples/rigged_cutscene demonstrates blend, rigid skins, IK, sound and events;
  generated source reproducible from timeline.json. Guide A07_ANIMATION_WORKFLOWS.md.
  Source-backed draft/save guards and existing animation/camera/tile features kept.

## Executed validation

- Locked workspace: 946 Rust tests pass, zero failed, 103 suites. Six new editor
  integration tests and six runtime tests cover combined workflows, cue delivery,
  silent seeking, paused JSON restore, preview draw/restore/unwind, rig undo,
  native decoded audio playback, IK bounds/ownership and callback fail/debug stops.
  egui pointer-event controls cover preview, blend, cue and IK creation UI.
- just check and strict WASM default/opt-out Clippy pass. Matching v16 tools/SDK
  rebuilt serially before GUI replays. Python helper tests: 49 pass.
- Rigged cutscene native/browser exports pass: green tip 1024 pixels at
  [224,104,256,136], blue beacon 64. Browser audit: zero Web Audio starts before
  consent, exactly one cue start. Platform audio adapter routing fixed and covered
  by a new callback/attachment regression. Prior v15 SDK export rejects.
- Original authored cutscene regression passes native/browser: 8192 green at
  [192,56,256,184], blue 64. Four final replays run serially after builds, relocated
  with authoring copies hidden; native PATH has no developer tools. Receipts:
  target/a07-complete-{native,browser},
  target/a07-complete-legacy-{native,browser}. Final pre-commit just check and
  whitespace checks pass (target/a07-complete-precommit.log).
- Logs target/a07-complete-{focused,rig,widgets,legacy,quality,workspace,wasm,
  wasm-optout,tools,web,python}.log. Final export receipts, stale SDK rejection
  and pre-commit checks are recorded in evidence/A07_ANIMATION_WORKFLOWS.md.
  Generated artifacts stay under target.

## Open / deferred

- Soft multi-influence weighted vertex mesh deformation, external Spine/DragonBones
  imports, sprite-frame texture crossfades and automatic parameter blend trees.
  Implemented skeleton is cutout, with rigid one-bone attachment skins; do not
  claim soft mesh skinning. Full OS authoring/reimport and hardware/performance
  acceptance remain open, so A07 stays PARTIAL. No physical audio/DPI claim.
- Editor camera navigation stays under user control during draw-only preview;
  gameplay camera-follow rigs save their poses/clocks, direct controller effect
  history remains transient. Existing logical-save script signatures still apply.
- A05 full split-screen authoring/host and remaining rendering/capacity acceptance;
  A06 stamps/streaming and complete level-authoring acceptance remain open.
  Animation 4096-cycle cap/extreme overload and physical performance acceptance
  remain open; no budgets bypassed.
- A04 source widget gate stopped after three attempts; saved-physics pose divergence
  remains. A03 identity/override/crash-transaction and A02 combined inspector gates
  deferred. No release/certification or full 3D acceptance claimed.

## Next

Continue A07 acceptance or explicitly requested extensions. Preserve bounded
attempts and deferred A03/A04 gates; do not advance past A07 unasked.
