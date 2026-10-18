//! Render quality tiers with auto-selection based on GPU capabilities.

use wgpu::Adapter;

/// Render quality tier for post-processing and effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderTier {
    /// Low quality: no post-processing, minimal effects, baked lighting.
    #[default]
    Low,
    /// Medium quality: basic post-processing (bloom, fog), soft shadows.
    Medium,
    /// High quality: full post-processing (bloom, LUT, fog, outline), real-time lighting.
    High,
}

impl RenderTier {
    /// Auto-select tier based on GPU adapter capabilities.
    ///
    /// Heuristics:
    /// - Integrated GPUs → Low
    /// - Discrete low-end → Medium
    /// - Discrete mid/high-end → High
    pub fn auto_select(adapter: &Adapter) -> Self {
        let info = adapter.get_info();
        let device_type = adapter.get_info().device_type;

        match device_type {
            wgpu::DeviceType::IntegratedGpu => {
                // Integrated GPUs default to Low for performance
                RenderTier::Low
            }
            wgpu::DeviceType::DiscreteGpu => {
                // Discrete GPUs: check memory for tier
                // This is a simple heuristic; could be refined with benchmarking
                if info.backend == wgpu::Backend::Gl {
                    // OpenGL backend often indicates older hardware
                    RenderTier::Medium
                } else {
                    RenderTier::High
                }
            }
            wgpu::DeviceType::Other | wgpu::DeviceType::VirtualGpu => {
                // Unknown/virtual: conservative Low
                RenderTier::Low
            }
            wgpu::DeviceType::Cpu => {
                // Software renderer: Low
                RenderTier::Low
            }
        }
    }

    /// Get the maximum number of bloom passes for this tier.
    pub fn max_bloom_passes(&self) -> u32 {
        match self {
            RenderTier::Low => 0,
            RenderTier::Medium => 3,
            RenderTier::High => 5,
        }
    }

    /// Whether LUT color grading is enabled.
    pub fn enable_lut(&self) -> bool {
        matches!(self, RenderTier::High)
    }

    /// Whether screen-space fog is enabled.
    pub fn enable_fog(&self) -> bool {
        matches!(self, RenderTier::Medium | RenderTier::High)
    }

    /// Whether outline effect is enabled.
    pub fn enable_outline(&self) -> bool {
        matches!(self, RenderTier::High)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_defaults_to_low() {
        assert_eq!(RenderTier::default(), RenderTier::Low);
    }

    #[test]
    fn low_tier_disables_all_effects() {
        let tier = RenderTier::Low;
        assert_eq!(tier.max_bloom_passes(), 0);
        assert!(!tier.enable_lut());
        assert!(!tier.enable_fog());
        assert!(!tier.enable_outline());
    }

    #[test]
    fn medium_tier_enables_basic_effects() {
        let tier = RenderTier::Medium;
        assert_eq!(tier.max_bloom_passes(), 3);
        assert!(!tier.enable_lut());
        assert!(tier.enable_fog());
        assert!(!tier.enable_outline());
    }

    #[test]
    fn high_tier_enables_all_effects() {
        let tier = RenderTier::High;
        assert_eq!(tier.max_bloom_passes(), 5);
        assert!(tier.enable_lut());
        assert!(tier.enable_fog());
        assert!(tier.enable_outline());
    }
}
