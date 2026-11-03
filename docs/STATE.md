# Kiln Session Handoff

**Active focus:** Task 1 of `docs/2D_COMPLETION_PLAN.md` completed locally; next is task 2.
**Branch:** `feat/2d-production-workflow`.
**Scope:** Continue the user's ordered 2D completion program. 3D remains paused. Historical milestone claims are not proof of complete authoring/export workflows.

## Done

- Native and WASM baseline repairs from prior checkpoints remain validated.
- Workspace formatted; strict native all-target Clippy now passes without added blanket suppressions. Removed unused duplicate state and legacy methods, corrected API/style diagnostics, and preserved the reachable graph-tab behavior.
- Existing plugin enable configuration now controls its checkbox state. Regressions verify independent/saved enable state.
- Replaced the invalid plugin C trait-object return with API v2 opaque handles and paired destruction; tested macro expansion, ABI function types, null handling, and exactly-once destruction. See ADR-0038. A plugin loader/method dispatch protocol remains future work.
- CI retains strict native checks and now checks Linux audio configurations with the required ALSA headers. Browser smoke coverage remains configured.

## Validation

- `just check`: passed (formatting, strict native all-target Clippy, workspace check, standalone audio sample check).
- `just test`: 455 passed; zero failed/skipped.
- `cargo build --workspace`: passed.
- `CHROMIUM_PATH=/usr/bin/chromium just web-smoke`: passed after cleanup; real assets/rendering/input/startup rejection exercised.
- Rebuilt native editor welcome and main workspace visually verified in Xvfb/software GL; no panic or GPU validation error. Virtual-display/framebuffer warnings remain.
- `cargo check -p kiln-editor --features audio`: passed; audible playback not claimed.
- `git diff --check`: passed.
- Remote CI/matrix execution remains pending. Optional audio and WASM builds have warnings outside the strict default-native Clippy invocation; no all-feature lint/physical-device guarantee is claimed.

## Next

1. Begin task 2 in `docs/2D_COMPLETION_PLAN.md`: project create/open/save/save-as, authoritative scene edits, round-trip property preservation, scene tabs, undo/redo, and dirty/recovery behavior. Start by reproducing the existing editor workflow and adding focused acceptance tests; document architecture decisions before restructuring.
2. Keep the restored gates green as changes land. Do not regress browser rendering/input or native script-state persistence.
3. Tasks 2–9 remain pending; use real reference-game/export evidence rather than panel/type existence.

## Limits and environment

Browser validation is Chromium software WebGL, not physical WebGPU, other browsers,
touch devices, or complete CLI exports. Full music/mixer/save behavior, physical
hardware frame budgets, release sizes, clean-machine packaging, and cross-platform
runner validation remain separate gates. Plugin ABI v2 is a compatibility change;
handles may only be released by their matching creator module.

Source `/workspace/.kiln-tools/activate.sh` for cloud Cargo commands.
`CHROMIUM_PATH=/usr/bin/chromium just web-smoke` uses the installed browser.
