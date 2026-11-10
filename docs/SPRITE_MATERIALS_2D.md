# Authored sprite materials — Task 8

Editor scene preview, native player and browser now render authored sprite
materials through the same pipeline. Select a sprite's material in Inspector.
Shader Editor can create a shader, typed parameters and a material, then change
its defaults/overrides. Its scene viewport is the actual preview; the former fake
preview box and incompatible new-shader template were removed. Shader errors are
shown in the panel; unresolved editor drafts are visibly magenta.

Open `samples/materials2d/kiln.toml` (also `examples/materials2d` in portable
bundles). Export it normally to Native or Browser with current matching tools.
S saves, N changes scenes and L restores the material scene.

## Fragment contract

Kiln owns vertex transforms, pivot/scale/rotation and UV mapping. Author a fragment
using the shared `VertexOutput` containing `color`, `uv` and `clip_pos`, returning
`@location(0) vec4<f32>`. It may take no arguments for constant colors, or one
VertexOutput. Example:

```wgsl
@fragment
fn paint(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(sprite_tex, sprite_sampler, in.uv);
    return texel * in.color * tint;
}
```

Set the shader entry to `paint`. An empty entry defaults to `fs_main`; the new
shader template uses the standard valid `fs_main`. Custom vertex stages, extra
entry points, additional resource bindings, integer/multiple color outputs and
arbitrary fragment argument layouts are outside this contract and fail validation.
Old definitions written for the unused placeholder interface need migration to
this fragment interface; they are not silently rendered with the default shader.

Define shader `parameters.tint = { Color = [1.0, 0.0, 0.0, 1.0] }` in the saved
project, or add a Color parameter in Shader Editor. A material references that
shader and can override declared parameters. Types are Float, Vec2, Vec3, Vec4,
Color and Texture. Overrides must match their declared type; finite numeric values
become deterministic typed WGSL constants. They specialize per material, rather
than supplying arbitrary uniform-buffer layouts. Live script parameter animation
is not introduced by this checkpoint.

A Texture parameter named `accent` generates `accent: texture_2d<f32>` and
`accent_sampler: sampler`. Use `textureSample(accent, accent_sampler, in.uv)`.
Defaults and overrides resolve through ordinary project-relative PNG assets and
texture import options. The sprite's own texture remains available as sprite_tex.
Missing/invalid textures fail export; no material may silently use a white fallback
for its required named texture. Up to four named textures are supported.

## Validation, caching and compatibility

Device-free Naga validation checks material/shader references (including prefab
and inactive-scene references), override names/types/finiteness, WGSL and the stage/
resource contract before exporting. Unused shader drafts without material
references can remain unfinished. Materials themselves are validated even when
not assigned, so later spawning cannot silently drop them. Programs use fixed
compatible camera/sprite/material bind layouts and the actual scene target format.
Backend pipeline failures are captured through wgpu error scopes and reported;
browser loading displays an actionable error. Native logs/title report a backend
material failure instead of starting a silently degraded game.

Unchanged definitions/resolved texture IDs reuse pipelines. Editor edits invalidate
the cache, and material texture changes refresh the ordinary scene resource
library. No recompile occurs on normal frames. Preparation/source/parameter counts
are bounded (BUDGETS.md). Compiling edits is an explicit native editor cost; no
reference-hardware latency claim is made. Transparent ordering is stable; only
adjacent matching texture/material runs batch.

RuntimeInfo.materials and an optional fifth WASM metadata line `materials=0/1`
advertise support. Legacy JSON and three/four-line metadata default false. Export
rejects material-bearing projects with incapable selected runtimes. Update editor,
exporter, player and prepared browser files together. The former unused renderer
placeholder Material/add_material/compile_shader APIs are replaced by validated
`SpriteRenderer::prepare_project_materials` and authored project definitions.

## Executed tests and limits

`tests/web/materials.cjs` exports with empty tools PATH, relocates output, hides the
project copy and serves only that artifact under a nested HTTP path. Pixel tests
cover Color/Float/default constants, named texture defaults/overrides,
default/custom/default reset, transparent painter order, flips/rotation and
cross-scene save/restore. It also tests invalid-source browser error feedback;
a local legacy runtime was used for real capability rejection.

`tests/native/materials_smoke.py` runs the same exported native game with no
archive argument, checks corresponding pixels/save/scene restoration and opens
the editor to verify actual material preview. Native/browser channel values agree
within one rounding value in the software test. CPU regressions cover bad WGSL/
stages/bindings/parameters, capability gating and preserved existing output.

```sh
node tests/web/materials.cjs target/debug/web-runtime
python3 tests/native/materials_smoke.py
```

See [evidence/task8-materials/README.md](evidence/task8-materials/README.md). Physics,
UI/audio/storage and editor export regressions remain checked separately. Tests
use Chromium/SwiftShader/Xvfb/software graphics; they do not certify physical GPU,
other browser/OS, startup/frame-time/memory or signed release provenance. Package
size is informational and never a completion goal.
