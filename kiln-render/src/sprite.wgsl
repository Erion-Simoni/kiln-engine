// Sprite batch shader — one instance per sprite
// Vertex buffer: unit quad corners (x,y)
// Instance buffer: position (x,y), size (w,h), color (rgba)

struct Uniforms {
    projection: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
    // unit-quad corner [-0.5 .. 0.5]
    @location(0) corner: vec2<f32>,
}

struct InstanceInput {
    @location(1) position: vec2<f32>,
    @location(2) size:     vec2<f32>,
    @location(3) color:    vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0)       color:    vec4<f32>,
}

@vertex
fn vs_main(v: VertexInput, inst: InstanceInput) -> VertexOutput {
    // Scale the unit quad, then translate to world position
    let world_pos = v.corner * inst.size + inst.position;
    var out: VertexOutput;
    out.clip_pos = uniforms.projection * vec4<f32>(world_pos, 0.0, 1.0);
    out.color    = inst.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
