// Post-processing shaders: bright extract, separable blur, composite.
//
// All passes use a fullscreen triangle (3 vertices, no vertex buffer).
// Group 0: source texture (0), sampler (1), bloom texture (2), params (3).

struct PostUniforms {
    bloom_threshold: f32,
    bloom_intensity: f32,
    fog_density: f32,
    outline_threshold: f32,
    // Blur direction in texel units (x only, or y only).
    blur_dir: vec2<f32>,
    _pad: vec2<f32>,
    fog_color: vec4<f32>,
    outline_color: vec4<f32>,
};

@group(0) @binding(0)
var src_tex: texture_2d<f32>;
@group(0) @binding(1)
var src_sampler: sampler;
@group(0) @binding(2)
var bloom_tex: texture_2d<f32>;
@group(0) @binding(3)
var<uniform> params: PostUniforms;

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let pos = positions[vi];
    var out: VsOut;
    out.clip_pos = vec4<f32>(pos, 0.0, 1.0);
    // Flip Y so texture V matches screen space.
    out.uv = vec2<f32>(pos.x * 0.5 + 0.5, 0.5 - pos.y * 0.5);
    return out;
}

fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

/// Bright-pass extract: keeps pixels above the luminance threshold.
@fragment
fn fs_bright(in: VsOut) -> @location(0) vec4<f32> {
    let color = textureSample(src_tex, src_sampler, in.uv);
    let keep = smoothstep(params.bloom_threshold, params.bloom_threshold + 0.1, luma(color.rgb));
    return vec4<f32>(color.rgb * keep, color.a);
}

/// Separable 9-tap gaussian blur along `params.blur_dir`.
@fragment
fn fs_blur(in: VsOut) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(textureDimensions(src_tex, 0));
    let texel = 1.0 / dims;
    var weights = array<f32, 5>(
        0.2270270270,
        0.1945945946,
        0.1216216216,
        0.0540540541,
        0.0162162162,
    );
    var sum = textureSample(src_tex, src_sampler, in.uv) * weights[0];
    for (var i = 1; i < 5; i = i + 1) {
        let offset = params.blur_dir * texel * f32(i) * 1.4;
        sum = sum
            + textureSample(src_tex, src_sampler, in.uv + offset) * weights[i]
            + textureSample(src_tex, src_sampler, in.uv - offset) * weights[i];
    }
    return sum;
}

/// Composite: scene + bloom + outline + fog.
@fragment
fn fs_composite(in: VsOut) -> @location(0) vec4<f32> {
    let scene = textureSample(src_tex, src_sampler, in.uv);
    let bloom = textureSample(bloom_tex, src_sampler, in.uv);
    var color = scene.rgb + bloom.rgb * params.bloom_intensity;

    // Outline: luminance edge detect against horizontal/vertical neighbours.
    let dims = vec2<f32>(textureDimensions(src_tex, 0));
    let texel = 1.0 / dims;
    let l = luma(scene.rgb);
    let l_l = luma(textureSample(src_tex, src_sampler, in.uv - vec2<f32>(texel.x, 0.0)).rgb);
    let l_r = luma(textureSample(src_tex, src_sampler, in.uv + vec2<f32>(texel.x, 0.0)).rgb);
    let l_d = luma(textureSample(src_tex, src_sampler, in.uv - vec2<f32>(0.0, texel.y)).rgb);
    let l_u = luma(textureSample(src_tex, src_sampler, in.uv + vec2<f32>(0.0, texel.y)).rgb);
    let edge = max(max(abs(l - l_l), abs(l - l_r)), max(abs(l - l_d), abs(l - l_u)));
    let outline_mask = smoothstep(params.outline_threshold, params.outline_threshold + 0.05, edge);
    color = mix(color, params.outline_color.rgb, outline_mask * params.outline_color.a);

    // Fog: blend towards fog colour by density.
    color = mix(color, params.fog_color.rgb, clamp(params.fog_density, 0.0, 1.0));

    return vec4<f32>(color, scene.a);
}
