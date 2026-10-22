//! 3D mesh and material types (feature-gated behind `3d`).
//!
//! These types are the shared data layer between the asset pipeline
//! (`kiln-assets`), the renderer (`kiln-render`), and the runtime (`kiln-runtime`).
//! They are only compiled when the `3d` feature is enabled to keep the
//! 2D-only binary small.

#[cfg(feature = "3d")]
use glam::{Mat4, Vec2, Vec3, Vec4};
use serde::{Deserialize, Serialize};

/// Handle to a mesh uploaded to the GPU.
#[cfg(feature = "3d")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MeshHandle(pub u32);

#[cfg(not(feature = "3d"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MeshHandle;

/// Primitive topology for mesh rendering.
#[cfg(feature = "3d")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimitiveTopology {
    TriangleList,
    TriangleStrip,
    LineList,
    LineStrip,
    PointList,
}

#[cfg(not(feature = "3d"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimitiveTopology {
    TriangleList,
}

/// Vertex attribute layout for 3D meshes.
#[cfg(feature = "3d")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VertexFormat {
    Float32x3,
    Float32x2,
    Float32x4,
    Uint32,
    Sint32,
}

#[cfg(not(feature = "3d"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VertexFormat {
    Float32x3,
}

/// A single vertex in a 3D mesh.
#[cfg(feature = "3d")]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub tangent: Vec4,
    pub uv: Vec2,
    pub color: Vec4,
}

#[cfg(not(feature = "3d"))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Vertex;

/// A sub-mesh within a mesh (for multi-material models).
#[cfg(feature = "3d")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubMesh {
    pub index_offset: u32,
    pub index_count: u32,
    pub material_index: usize,
}

#[cfg(not(feature = "3d"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubMesh;

/// CPU-side mesh data before GPU upload.
#[cfg(feature = "3d")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub submeshes: Vec<SubMesh>,
    pub bounds: MeshBounds,
}

#[cfg(not(feature = "3d"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mesh;

/// World-space bounding box of a mesh.
#[cfg(feature = "3d")]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MeshBounds {
    pub min: Vec3,
    pub max: Vec3,
}

#[cfg(not(feature = "3d"))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MeshBounds;

impl MeshBounds {
    /// Create an empty bounds.
    #[cfg(feature = "3d")]
    pub fn empty() -> Self {
        Self {
            min: Vec3::splat(f32::INFINITY),
            max: Vec3::splat(f32::NEG_INFINITY),
        }
    }

    #[cfg(not(feature = "3d"))]
    pub fn empty() -> Self {
        Self
    }

    /// Expand to include a point.
    #[cfg(feature = "3d")]
    pub fn expand(&mut self, point: Vec3) {
        self.min = self.min.min(point);
        self.max = self.max.max(point);
    }

    #[cfg(not(feature = "3d"))]
    pub fn expand(&mut self, _point: ()) {}

    /// Center of the bounds.
    #[cfg(feature = "3d")]
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    #[cfg(not(feature = "3d"))]
    pub fn center(&self) {}

    /// Extents (half-size) of the bounds.
    #[cfg(feature = "3d")]
    pub fn extents(&self) -> Vec3 {
        (self.max - self.min) * 0.5
    }

    #[cfg(not(feature = "3d"))]
    pub fn extents(&self) {}
}

/// PBR material parameters.
#[cfg(feature = "3d")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Material {
    pub name: String,
    pub base_color: Vec4,
    pub base_color_texture: Option<String>,
    pub normal_texture: Option<String>,
    pub metallic_roughness_texture: Option<String>,
    pub occlusion_texture: Option<String>,
    pub emissive_texture: Option<String>,
    pub emissive_color: Vec3,
    pub metallic: f32,
    pub roughness: f32,
    pub alpha_mode: AlphaMode,
    pub alpha_cutoff: f32,
    pub double_sided: bool,
}

#[cfg(not(feature = "3d"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Material;

#[cfg(feature = "3d")]
impl Default for Material {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            base_color: Vec4::ONE,
            base_color_texture: None,
            normal_texture: None,
            metallic_roughness_texture: None,
            occlusion_texture: None,
            emissive_texture: None,
            emissive_color: Vec3::ZERO,
            metallic: 1.0,
            roughness: 1.0,
            alpha_mode: AlphaMode::Opaque,
            alpha_cutoff: 0.5,
            double_sided: false,
        }
    }
}

#[cfg(not(feature = "3d"))]
impl Default for Material {
    fn default() -> Self {
        Self
    }
}

/// Alpha blending mode for materials.
#[cfg(feature = "3d")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlphaMode {
    Opaque,
    Mask,
    Blend,
}

#[cfg(not(feature = "3d"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlphaMode {
    Opaque,
}

/// A loaded 3D model (can contain multiple meshes and materials).
#[cfg(feature = "3d")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
    pub root_transform: Mat4,
}

#[cfg(not(feature = "3d"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model;

#[cfg(test)]
mod tests {
    #[cfg(feature = "3d")]
    use super::*;

    #[cfg(feature = "3d")]
    #[test]
    fn mesh_bounds_expand() {
        let mut bounds = MeshBounds::empty();
        bounds.expand(Vec3::new(1.0, 2.0, 3.0));
        bounds.expand(Vec3::new(-1.0, -2.0, -3.0));
        assert_eq!(bounds.min, Vec3::new(-1.0, -2.0, -3.0));
        assert_eq!(bounds.max, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(bounds.center(), Vec3::ZERO);
        assert_eq!(bounds.extents(), Vec3::new(1.0, 2.0, 3.0));
    }

    #[cfg(feature = "3d")]
    #[test]
    fn default_material_is_opaque() {
        let mat = Material::default();
        assert_eq!(mat.alpha_mode, AlphaMode::Opaque);
        assert_eq!(mat.base_color, Vec4::ONE);
    }
}