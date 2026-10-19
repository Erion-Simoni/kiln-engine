// Particle compute shader
// Updates particle positions and ages

struct Particle {
    position: vec2<f32>,
    velocity: vec2<f32>,
    color: vec4<f32>,
    size: f32,
    normalized_age: f32,
}

struct Uniforms {
    dt: f32,
    gravity_x: f32,
    gravity_y: f32,
    time: f32,
    padding: array<f32, 4>,
};

@group(0) @binding(0)
var<storage, read_write> particles: array<Particle>;

@group(0) @binding(1)
var<uniform> uniforms: Uniforms;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;

    if (index >= arrayLength(&particles)) {
        return;
    }

    var p = particles[index];

    // Skip dead particles
    if (p.normalized_age >= 1.0) {
        return;
    }

    // Apply gravity
    p.velocity.x += uniforms.gravity_x * uniforms.dt;
    p.velocity.y += uniforms.gravity_y * uniforms.dt;

    // Update position
    p.position.x += p.velocity.x * uniforms.dt;
    p.position.y += p.velocity.y * uniforms.dt;

    // Update age (normalized_age += dt / lifetime)
    // For simplicity, we increment by a fixed amount per frame
    // In a real system, lifetime would be stored separately
    p.normalized_age += uniforms.dt * 0.5; // Approximate 2-second lifetime

    // Fade out
    p.color.a = 1.0 - p.normalized_age;

    particles[index] = p;
}
