//! Deterministic baking of saved atlas recipes without mutating authoring files.
use crate::{atlas::AtlasBuilder, decode_png, encode_png_rgba};
use kiln_core::{
    atlas::{self, AtlasRecipe},
    Component, Project,
};
use sha2::{Digest, Sha256};

pub const MAX_PNG_BYTES: u64 = 80 * 1024 * 1024;

pub struct BakedAtlas {
    pub png: Vec<u8>,
    pub recipe: AtlasRecipe,
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}

/// Validate a portable, ordinary project PNG resource path.
pub fn png_path(path: &str) -> Result<(), String> {
    if !path.ends_with(".png")
        || path.contains(['\\', ':', '"'])
        || path.chars().any(char::is_control)
        || !path.split('/').all(|p| {
            !p.is_empty() && !p.starts_with('.') && !["target", "export", "dist"].contains(&p)
        })
    {
        return Err("Choose a project-relative .png path outside generated/private folders".into());
    }
    Ok(())
}

/// Decode inputs in stable label order and build pixels/regions before publication.
pub fn bake(
    id: &str,
    recipe: &AtlasRecipe,
    read: impl Fn(&str) -> Result<Vec<u8>, String>,
) -> Result<BakedAtlas, String> {
    if !identifier(id) || recipe.sources.keys().any(|label| !identifier(label)) {
        return Err(
            "Atlas names and source labels need 1–64 letters, digits, underscores or hyphens"
                .into(),
        );
    }
    png_path(&recipe.output)?;
    if recipe.size == 0
        || recipe.size > 4096
        || recipe.padding > 64
        || recipe.sources.is_empty()
        || recipe.sources.len() > 256
    {
        return Err("Atlas needs 1–256 sources, size 1–4096 and padding 0–64".into());
    }
    let mut builder = AtlasBuilder::new(recipe.size).with_padding(recipe.padding);
    let mut total = 0_u64;
    for (label, path) in &recipe.sources {
        png_path(path)?;
        if path == &recipe.output {
            return Err("Atlas output cannot also be an input".into());
        }
        let bytes = read(path)?;
        if bytes.len() as u64 > MAX_PNG_BYTES || bytes.len() < 24 || !crate::is_png(&bytes) {
            return Err(format!("{path}: invalid or oversized PNG"));
        }
        let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
        let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
        if width > recipe.size || height > recipe.size {
            return Err("Atlas inputs exceed output dimensions".into());
        }
        total = total.saturating_add(u64::from(width) * u64::from(height) * 4);
        if total > 256 * 1024 * 1024 {
            return Err(
                "Atlas inputs exceed output dimensions or 256 MiB decoded input limit".into(),
            );
        }
        let image = decode_png(&bytes).map_err(|e| format!("{path}: {e}"))?;
        builder
            .add_sprite(label, image)
            .map_err(|e| format!("{label}: {e}"))?;
    }
    let atlas = builder.try_build().map_err(|e| e.to_string())?;
    let png = encode_png_rgba(
        atlas.texture.width,
        atlas.texture.height,
        &atlas.texture.pixels,
    )
    .ok_or("Could not encode atlas")?;
    let mut baked = recipe.clone();
    baked.built_hash = image_hash(&png);
    baked.built_regions = atlas
        .sprites
        .into_iter()
        .map(|(label, uv)| {
            let size = recipe.size as f32;
            let x = (uv.u0 * size).round() as u32;
            let y = (uv.v0 * size).round() as u32;
            (
                label,
                [
                    x,
                    y,
                    (uv.u1 * size).round() as u32 - x,
                    (uv.v1 * size).round() as u32 - y,
                ],
            )
        })
        .collect();
    Ok(BakedAtlas { png, recipe: baked })
}

fn used_clip(project: &Project, name: &str) -> bool {
    let referenced = project.scenes.values().flat_map(|s| &s.entities)
        .chain(project.prefabs.values().map(|p| &p.root_entity)).any(|entity| {
            entity.components.iter().any(|component| match component {
                Component::Sprite { animation_clip: Some(clip), .. } => clip == name,
                Component::PrefabInstance { overrides, .. } => overrides.overrides.values().any(|value| matches!(value,
                    kiln_core::ComponentOverride::SpriteResources { animation_clip: Some(Some(clip)), .. } if clip == name)),
                _ => false,
            })
        });
    referenced
        || project
            .animation_clips
            .values()
            .any(|clip| clip.next.as_deref() == Some(name))
}

/// Check generated clip ownership without changing project state. Independent
/// edits must be moved to a separately named clip before rebuilding this recipe.
pub fn validate_clips(project: &Project, id: &str, recipe: &AtlasRecipe) -> Result<(), String> {
    let old = project.atlases.get(id);
    if let Some(old) = old {
        for (label, rect) in &old.built_regions {
            let name = atlas::clip_name(id, label);
            let expected = atlas::clip(id, label, &old.output, *rect);
            let actual = project
                .animation_clips
                .get(&name)
                .ok_or_else(|| format!("Generated clip {name} is missing"))?;
            if serde_json::to_value(actual).map_err(|e| e.to_string())?
                != serde_json::to_value(expected).map_err(|e| e.to_string())?
            {
                return Err(format!(
                    "Generated clip {name} was edited; undo those edits before rebuilding, and duplicate the clip first for custom animation"
                ));
            }
            if !recipe.sources.contains_key(label) && used_clip(project, &name) {
                return Err(format!("Generated clip {name} is still referenced; remove its uses before removing that atlas source"));
            }
        }
    }
    for label in recipe.sources.keys() {
        let name = atlas::clip_name(id, label);
        if project.animation_clips.contains_key(&name)
            && !old.is_some_and(|r| r.built_regions.contains_key(label))
        {
            return Err(format!(
                "Clip {name} already exists and is not owned by this atlas"
            ));
        }
    }
    Ok(())
}

/// Publish a successful bake to the in-memory project after its PNG was written.
pub fn apply(project: &mut Project, id: &str, baked: &BakedAtlas) {
    if let Some(old) = project.atlases.get(id) {
        for label in old.built_regions.keys() {
            project.animation_clips.remove(&atlas::clip_name(id, label));
        }
    }
    for (label, rect) in &baked.recipe.built_regions {
        let clip = atlas::clip(id, label, &baked.recipe.output, *rect);
        project.animation_clips.insert(clip.name.clone(), clip);
    }
    project.atlases.insert(id.into(), baked.recipe.clone());
}

/// Content identity used to protect generated atlas output against external edits.
pub fn image_hash(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
