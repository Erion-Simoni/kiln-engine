@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(sprite_tex, sprite_sampler, in.uv);
    return texel * in.color;
}
