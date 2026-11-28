struct Uniforms { projection: mat4x4<f32>, }
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var sprite_tex: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;
@group(1) @binding(2) var sprite_tex_b: texture_2d<f32>;
@group(1) @binding(3) var sprite_sampler_b: sampler;
struct VertexInput { @location(0) corner: vec2<f32>, }
struct InstanceInput {
 @location(1) position_size:vec4<f32>, @location(2) color:vec4<f32>,
 @location(3) uv_rect:vec4<f32>, @location(5) rotation:f32,
 @location(6) p01:vec4<f32>, @location(7) p23:vec4<f32>,
 @location(8) uv01:vec4<f32>, @location(9) uv23:vec4<f32>,
 @location(10) deformed:u32, @location(11) blend_uv:vec4<f32>, @location(12) weight:f32,
}
struct VertexOutput {
 @builtin(position) clip_pos:vec4<f32>, @location(0) color:vec4<f32>,
 @location(1) uv:vec2<f32>, @location(2) uv_b:vec2<f32>, @location(3) weight:f32,
}
@vertex fn vs_main(v:VertexInput,inst:InstanceInput)->VertexOutput {
 let local=v.corner*inst.position_size.zw;let c=cos(inst.rotation);let s=sin(inst.rotation);
 var p=vec2<f32>(local.x*c-local.y*s,local.x*s+local.y*c)+inst.position_size.xy;
 var t=v.corner+vec2<f32>(0.5);
 if inst.deformed!=0u {
  if v.corner.y<0.0 {if v.corner.x<0.0 {p=inst.p01.xy;t=inst.uv01.xy;}else{p=inst.p01.zw;t=inst.uv01.zw;}}
  else {if v.corner.x>0.0 {p=inst.p23.xy;t=inst.uv23.xy;}else{p=inst.p23.zw;t=inst.uv23.zw;}}
 }
 var out:VertexOutput;out.clip_pos=uniforms.projection*vec4<f32>(p,0.0,1.0);out.color=inst.color;
 out.uv=mix(inst.uv_rect.xy,inst.uv_rect.zw,t);out.uv_b=mix(inst.blend_uv.xy,inst.blend_uv.zw,t);out.weight=inst.weight;return out;
}
// Blend in premultiplied space so transparent texels do not darken a crossfade.
fn sample_sprite(in:VertexOutput)->vec4<f32> {
 let a=textureSample(sprite_tex,sprite_sampler,in.uv);let b=textureSample(sprite_tex_b,sprite_sampler_b,in.uv_b);
 let w=clamp(in.weight,0.0,1.0);let alpha=mix(a.a,b.a,w);let rgb=mix(a.rgb*a.a,b.rgb*b.a,w);
 if alpha>0.0 {return vec4<f32>(rgb/alpha,alpha);}return vec4<f32>(0.0);
}
