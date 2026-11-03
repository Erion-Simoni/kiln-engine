# Kiln Session Handoff

**Active focus:** Native editor startup repair on top of master `7dc980c` (2026-11-03).
**Milestones:** M0–M28 and the 2D gate remain complete; Kiln 2D v1.0 hardware/device verification remains outstanding. M29 remains inactive. This session does not advance a milestone.

## Done

- Updated the clean cloud checkout to the latest merged master.
- Repaired editor compilation: duplicate definitions, mismatched panel and ECS APIs, missing sprite/shader fields, invalid egui calls, and conflicting UI borrows.
- Kept plugin configuration per plugin instead of returning a shared mutable static.
- Encapsulated Rapier body/collider removal so companion sets remain consistent.
- Routed project hub open/create callbacks to EditorApp rather than stub methods.
- Fixed the black startup window: render on RedrawRequested, request subsequent redraws, and present submitted surface frames.
- Honor WGPU_BACKEND for backend selection and propagate main/event-loop startup errors.
- `just run` now starts kiln-editor; `just demo` retains access to the demo.

## Validation

- `cargo build -p kiln-editor`: passed with existing warnings.
- `cargo test -p kiln-editor --lib`: 33 passed, zero failed or ignored; includes animation panel state preservation and isolated plugin settings regressions.
- Native launch on Xvfb using software OpenGL (`DISPLAY=:99 WGPU_BACKEND=gl`): welcome screen visibly rendered; entered the main workspace with a real pointer click; no panic or GPU validation error. Virtual display/framebuffer warnings remain.
- `git diff --check`: passed.
- `just check`: failed during formatting because kiln-web/src/lib.rs:1160 has an existing unmatched closing delimiter. Full-workspace checks are not passing. The user explicitly requested committing and pushing these editor fixes to a new branch despite this previously reported unrelated failure.

## Next

- Review the editor changes and test on a physical desktop GPU. Launch from the repository with `cargo run --bin kiln-editor` or `just run`.
- In this cloud machine, source `/workspace/.kiln-tools/activate.sh` before Cargo commands. A graphical display is required. Virtual-display verification used the GL backend; this machine has no Vulkan driver.
- Separately repair the web crate parser error and the previously identified core/player test and optional audio-feature defects before claiming full-workspace readiness. Audio preview is gated behind the optional editor audio feature, forwarding to kiln-runtime/audio; it is disabled in the validated default build and not validated here.
- Physical Low/Medium GPU frame budgets and mobile/device acceptance checks remain outstanding.
