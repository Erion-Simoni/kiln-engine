use super::*;

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
/// Fail explicitly for authored features that this browser host has not integrated.
pub fn validate_target(project: &kiln_core::Project) -> Result<()> {
    let entities = project
        .scenes
        .values()
        .flat_map(|s| &s.entities)
        .chain(project.prefabs.values().map(|p| &p.root_entity));
    for entity in entities {
        for component in &entity.components {
            if let kiln_core::Component::Sprite {
                material: Some(_), ..
            } = component
            {
                bail!("This project uses material overrides, which this Browser runtime does not support. Export a native game.");
            }
        }
    }
    Ok(())
}
/// Copy a prepared runtime and emit a relocatable loading/start shell.
pub fn write(
    stage: &Path,
    bundle: &Path,
    project: &kiln_core::Project,
    marker: &Path,
) -> Result<()> {
    let destination = stage.join("runtime");
    fs::create_dir(&destination)?;
    for name in kiln_assets::web_bundle::FILES {
        cancelled(marker)?;
        fs::copy(bundle.join(name), destination.join(name))?;
    }
    kiln_assets::web_bundle::verify(&destination).map_err(anyhow::Error::msg)?;
    let width = project.window.width.max(1);
    let height = project.window.height.max(1);
    let title = escape(&project.name);
    let html = include_str!("web_shell.html")
        .replace("{{WIDTH}}", &width.to_string())
        .replace("{{HEIGHT}}", &height.to_string())
        .replace("{{TITLE}}", &title);
    fs::write(stage.join("index.html"), html)?;
    fs::write(stage.join("game.js"), include_str!("web_shell.js"))?;
    fs::write(stage.join("_headers"),"/*\n  Cross-Origin-Opener-Policy: same-origin\n  Cross-Origin-Embedder-Policy: require-corp\n")?;
    Ok(())
}
