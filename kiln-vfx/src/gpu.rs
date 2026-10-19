//! GPU-based particle system using compute shaders.

use crate::particle::ParticleData;
use wgpu::{Device, Queue};

/// Maximum particles for GPU system.
pub const MAX_GPU_PARTICLES: usize = 10_000;

/// GPU particle system configuration.
#[derive(Debug, Clone, Copy)]
pub struct GpuParticleSystemConfig {
    /// Maximum number of particles.
    pub max_particles: usize,
}

impl Default for GpuParticleSystemConfig {
    fn default() -> Self {
        Self {
            max_particles: MAX_GPU_PARTICLES,
        }
    }
}

/// GPU particle system using compute shaders.
pub struct GpuParticleSystem {
    config: GpuParticleSystemConfig,
    particle_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    compute_pipeline: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,
    particle_count: u32,
}

impl GpuParticleSystem {
    /// Create a new GPU particle system.
    pub fn new(device: &Device, config: GpuParticleSystemConfig) -> Self {
        // Particle buffer (read/write for compute, vertex for render)
        let particle_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("particle_buffer"),
            size: ParticleData::STRIDE * config.max_particles as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Uniform buffer (dt, gravity, time)
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("particle_uniform"),
            size: 32, // dt (4) + gravity (8) + time (4) + padding (16)
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Compute shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("particle_compute"),
            source: wgpu::ShaderSource::Wgsl(include_str!("particle_compute.wgsl").into()),
        });

        // Bind group layout
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("particle_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("particle_bg"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        // Compute pipeline
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("particle_compute_layout"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("particle_compute_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            config,
            particle_buffer,
            uniform_buffer,
            compute_pipeline,
            bind_group,
            particle_count: 0,
        }
    }

    /// Get the particle buffer for rendering.
    pub fn particle_buffer(&self) -> &wgpu::Buffer {
        &self.particle_buffer
    }

    /// Get the current particle count.
    pub fn particle_count(&self) -> u32 {
        self.particle_count
    }

    /// Set the particle count (for spawning).
    pub fn set_particle_count(&mut self, count: u32) {
        self.particle_count = count.min(self.config.max_particles as u32);
    }

    /// Update particles via compute shader.
    pub fn update(&self, device: &Device, queue: &Queue, dt: f32, gravity: [f32; 2], time: f32) {
        // Upload uniforms
        let mut uniforms = [0.0f32; 8];
        uniforms[0] = dt;
        uniforms[1] = gravity[0];
        uniforms[2] = gravity[1];
        uniforms[3] = time;
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&uniforms));

        // Dispatch compute (1 thread per particle)
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("particle_update"),
        });

        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("particle_compute_pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.compute_pipeline);
            cpass.set_bind_group(0, &self.bind_group, &[]);
            cpass.dispatch_workgroups(
                self.particle_count.div_ceil(64), // 64 threads per workgroup
                1,
                1,
            );
        }

        queue.submit(Some(encoder.finish()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_config_default() {
        let config = GpuParticleSystemConfig::default();
        assert_eq!(config.max_particles, MAX_GPU_PARTICLES);
    }
}
