//! Shared authored fragment contract and deterministic, device-free validation.
use kiln_core::{project::MaterialParam, Component, Project};
use std::collections::BTreeMap;

/// Default fragment used by the renderer and new-shader editor template.
pub const DEFAULT_FRAGMENT: &str = include_str!("sprite.wgsl");
pub(crate) const VERTEX: &str = include_str!("sprite_vertex.wgsl");

/// A validated material, ready for backend compilation and texture binding.
#[derive(Debug, Clone)]
pub struct MaterialPlan {
    pub name: String,
    pub source: String,
    pub entry: String,
    pub textures: Vec<String>,
}
/// Detect authored references, including reusable prefab definitions.
pub fn project_uses_materials(project: &Project) -> bool {
    !project.materials.is_empty()
        || project
            .scenes
            .values()
            .flat_map(|s| &s.entities)
            .chain(project.prefabs.values().map(|p| &p.root_entity))
            .any(|e| {
                e.components.iter().any(|c| {
                    matches!(
                        c,
                        Component::Sprite {
                            material: Some(_),
                            ..
                        }
                    )
                })
            })
}
/// Material texture paths, including defaults and overrides, without parsing WGSL.
pub fn texture_paths(project: &Project) -> Vec<String> {
    let mut paths = Vec::new();
    for material in project.materials.values() {
        if let Some(shader) = project.shaders.get(&material.shader) {
            for (name, default) in &shader.parameters {
                if let MaterialParam::Texture(path) =
                    material.parameters.get(name).unwrap_or(default)
                {
                    paths.push(path.clone());
                }
            }
        }
    }
    paths.sort();
    paths.dedup();
    paths
}
fn identifier(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !name.starts_with("__")
}
fn parameter(name: &str, value: &MaterialParam) -> Result<String, String> {
    let (kind, values): (&str, Vec<f32>) = match value {
        MaterialParam::Float(v) => ("f32", vec![*v]),
        MaterialParam::Vec2(v) => ("vec2<f32>", v.to_vec()),
        MaterialParam::Vec3(v) => ("vec3<f32>", v.to_vec()),
        MaterialParam::Vec4(v) | MaterialParam::Color(v) => ("vec4<f32>", v.to_vec()),
        MaterialParam::Texture(_) => return Err("Texture is not a numeric constant".into()),
    };
    if values.iter().any(|v| !v.is_finite()) {
        return Err(format!("Nonfinite material parameter: {name}"));
    }
    let literal = values
        .iter()
        .map(|v| format!("{v:?}"))
        .collect::<Vec<_>>()
        .join(",");
    let expression = if values.len() == 1 {
        literal
    } else {
        format!("{kind}({literal})")
    };
    Ok(format!("const {name}: {kind} = {expression};\n"))
}
fn same_type(a: &MaterialParam, b: &MaterialParam) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}
fn compile_plan(project: &Project, name: &str) -> Result<MaterialPlan, String> {
    let material = &project.materials[name];
    let shader = project
        .shaders
        .get(&material.shader)
        .ok_or_else(|| format!("Material '{name}' has missing shader '{}'", material.shader))?;
    if shader.source.len() > 256 * 1024 || shader.parameters.len() > 64 {
        return Err("Shader source/parameter limit exceeded".into());
    }
    let entry = if shader.entry_point.is_empty() {
        "fs_main"
    } else {
        &shader.entry_point
    };
    if !identifier(entry) {
        return Err("Invalid fragment entry name".into());
    }
    for (key, value) in &material.parameters {
        let default = shader
            .parameters
            .get(key)
            .ok_or_else(|| format!("Unknown parameter override '{key}' in material '{name}'"))?;
        if !same_type(default, value) {
            return Err(format!(
                "Parameter type mismatch for '{key}' in material '{name}'"
            ));
        }
    }
    let mut declarations = String::new();
    let mut textures = Vec::new();
    let parameters: BTreeMap<_, _> = shader.parameters.iter().collect();
    for (key, default) in parameters {
        if !identifier(key) {
            return Err(format!("Invalid parameter name '{key}'"));
        }
        let value = material.parameters.get(key).unwrap_or(default);
        if let MaterialParam::Texture(path) = value {
            if path.is_empty() || textures.len() >= 4 {
                return Err("Material texture path/limit invalid".into());
            }
            let binding = textures.len() * 2;
            declarations.push_str(&format!("@group(2) @binding({binding}) var {key}: texture_2d<f32>;\n@group(2) @binding({}) var {key}_sampler: sampler;\n",binding+1));
            textures.push(path.clone());
        } else {
            declarations.push_str(&parameter(key, value)?);
        }
    }
    let source = format!("{VERTEX}\n{declarations}\n{}", shader.source);
    validate_source(&source, entry, textures.len())?;
    Ok(MaterialPlan {
        name: name.into(),
        source,
        entry: entry.into(),
        textures,
    })
}
/// Parse and validate the complete fixed-vertex/custom-fragment program.
pub fn validate_source(source: &str, entry: &str, textures: usize) -> Result<(), String> {
    let module = naga::front::wgsl::parse_str(source).map_err(|e| e.emit_to_string(source))?;
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .map_err(|e| e.to_string())?;
    if module.entry_points.len() != 2
        || !module
            .entry_points
            .iter()
            .any(|e| e.name == entry && e.stage == naga::ShaderStage::Fragment)
        || !module
            .entry_points
            .iter()
            .any(|e| e.name == "vs_main" && e.stage == naga::ShaderStage::Vertex)
    {
        return Err(
            "Material must provide one fragment entry; vertex stage is owned by Kiln".into(),
        );
    }
    let fragment = module
        .entry_points
        .iter()
        .find(|e| e.name == entry && e.stage == naga::ShaderStage::Fragment)
        .unwrap();
    let result = fragment
        .function
        .result
        .as_ref()
        .ok_or("Fragment must return RGBA")?;
    if !matches!(
        module.types[result.ty].inner,
        naga::TypeInner::Vector {
            size: naga::VectorSize::Quad,
            scalar: naga::Scalar {
                kind: naga::ScalarKind::Float,
                width: 4
            }
        }
    ) || !matches!(
        result.binding,
        Some(naga::Binding::Location { location: 0, .. })
    ) {
        return Err("Fragment must return @location(0) vec4<f32>".into());
    }
    if fragment.function.arguments.len() > 1
        || fragment
            .function
            .arguments
            .first()
            .is_some_and(|arg| module.types[arg.ty].name.as_deref() != Some("VertexOutput"))
    {
        return Err("Fragment accepts no arguments or one VertexOutput input".into());
    }
    if module
        .global_variables
        .iter()
        .filter(|(_, v)| v.binding.is_some())
        .count()
        != 3 + textures * 2
    {
        return Err("Material declares additional resource bindings".into());
    }
    for (_, variable) in module.global_variables.iter() {
        if let Some(binding) = &variable.binding {
            if !matches!((binding.group, binding.binding), (0, 0) | (1, 0) | (1, 1))
                && !(binding.group == 2 && binding.binding < textures as u32 * 2)
            {
                return Err("Material declares an unsupported resource binding".into());
            }
        }
    }
    Ok(())
}
/// Validate every material/reference without opening a graphics/audio device.
pub fn plans(project: &Project) -> Result<Vec<MaterialPlan>, String> {
    if project.materials.len() > 256
        || project.shaders.len() > 256
        || project
            .shaders
            .values()
            .map(|s| s.source.len())
            .sum::<usize>()
            > 1024 * 1024
    {
        return Err("Project shader/material limit exceeded".into());
    }
    for entity in project
        .scenes
        .values()
        .flat_map(|s| &s.entities)
        .chain(project.prefabs.values().map(|p| &p.root_entity))
    {
        for component in &entity.components {
            if let Component::Sprite {
                material: Some(name),
                ..
            } = component
            {
                if !project.materials.contains_key(name) {
                    return Err(format!(
                        "Entity '{}' references missing material '{name}'",
                        entity.id
                    ));
                }
            }
        }
    }
    let mut names: Vec<_> = project.materials.keys().collect();
    names.sort();
    names
        .into_iter()
        .map(|name| compile_plan(project, name).map_err(|e| format!("Material '{name}': {e}")))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn project() -> Project {
        let mut project = Project::default();
        project.shaders.insert(
            "shader".into(),
            kiln_core::project::Shader {
                id: "shader".into(),
                name: "shader".into(),
                source: DEFAULT_FRAGMENT.into(),
                entry_point: "fs_main".into(),
                parameters: Default::default(),
            },
        );
        project.materials.insert(
            "material".into(),
            kiln_core::project::SpriteMaterial {
                id: "material".into(),
                name: "material".into(),
                shader: "shader".into(),
                parameters: Default::default(),
            },
        );
        project
    }
    #[test]
    fn typed_constants_texture_defaults_and_overrides_are_deterministic() {
        let mut project = project();
        let params = &mut project.shaders.get_mut("shader").unwrap().parameters;
        params.insert("tint".into(), MaterialParam::Color([0.0, 1.0, 0.0, 1.0]));
        params.insert("factor".into(), MaterialParam::Float(0.5));
        params.insert("offset".into(), MaterialParam::Vec2([0.0, 0.0]));
        params.insert("gain".into(), MaterialParam::Vec3([1.0; 3]));
        params.insert("extra".into(), MaterialParam::Vec4([1.0; 4]));
        params.insert("accent".into(), MaterialParam::Texture("red.png".into()));
        project
            .materials
            .get_mut("material")
            .unwrap()
            .parameters
            .insert("accent".into(), MaterialParam::Texture("blue.png".into()));
        let first = plans(&project).unwrap();
        assert_eq!(first[0].textures, vec!["blue.png"]);
        assert_eq!(texture_paths(&project), vec!["blue.png"]);
        assert!(first[0].source.contains("const factor: f32 = 0.5"));
        assert_eq!(first[0].source, plans(&project).unwrap()[0].source);
        project
            .materials
            .get_mut("material")
            .unwrap()
            .parameters
            .insert("factor".into(), MaterialParam::Vec2([0.0; 2]));
        assert!(plans(&project).unwrap_err().contains("type mismatch"));
    }
    #[test]
    fn invalid_bindings_stages_types_sources_and_missing_references_fail_without_a_device() {
        for source in ["bad WGSL", "@fragment fn fs_main()->@location(0) u32{return 1u;}",
            "@fragment fn fs_main(@location(0) input:vec2<f32>)->@location(0)vec4<f32>{return vec4<f32>(input,0.0,1.0);}",
            "@group(3) @binding(0) var tex:texture_2d<f32>; @fragment fn fs_main()->@location(0)vec4<f32>{return vec4<f32>(1.0);}"] {
            let mut project=project();project.shaders.get_mut("shader").unwrap().source=source.into();
            assert!(plans(&project).is_err(),"{source}");
        }
        let mut project = project();
        project
            .shaders
            .get_mut("shader")
            .unwrap()
            .parameters
            .insert("factor".into(), MaterialParam::Float(f32::NAN));
        assert!(plans(&project).unwrap_err().contains("Nonfinite"));
        project.shaders.clear();
        assert!(plans(&project).unwrap_err().contains("missing shader"));
    }
}
