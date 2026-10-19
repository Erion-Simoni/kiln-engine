// Sprite batch shader — textured instanced quads with colour tint.
// Vertex buffer: unit quad corners (x,y)
// Instance buffer: position, size, color, uv_rect

struct Uniforms {
    projection: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(1) @binding(0)
var sprite_tex: texture_2d<f32>;
@group(1) @binding(1)
var sprite_sampler: sampler;

struct VertexInput {
    @location(0) corner: vec2<f32>,
}

struct InstanceInput {
    @location(1) position: vec2<f32>,
    @location(2) size:     vec2<f32>,
    @location(3) color:    vec4<f32>,
    @location(4) uv_rect:  vec4<f32>, // u0, v0, u1, v1
}

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0)       color:    vec4<f32>,
    @location(1)       uv:       vec2<f32>,
}

@vertex
fn vs_main(v: VertexInput, inst: InstanceInput) -> VertexOutput {
    let world_pos = v.corner * inst.size + inst.position;
    // Map unit-quad corner [-0.5,0.5] → [0,1] then into uv_rect
    let t = v.corner + vec2<f32>(0.5, 0.5);
    let uv = mix(inst.uv_rect.xy, inst.uv_rect.zw, t);

    var out: VertexOutput;
    out.clip_pos = uniforms.projection * vec4<f32>(world_pos, 0.0, 1.0);
    out.color = inst.color;
    out.uv = uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(sprite_tex, sprite_sampler, in.uv);
    return texel * in.color;
}
