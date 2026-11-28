//! Explicit skin bind capture, grid generation and per-vertex weight authoring.
use kiln_core::{skin::*, Component, Entity, Scene};
#[derive(Debug, Default)]
/// Persistent inspector draft, replaced only by explicit validated Apply.
pub struct SkinEditor {
    owner: String,
    source: String,
    draft: Option<SkinMesh>,
    bones: [String; 2],
    columns: usize,
    rows: usize,
    error: String,
}
/// Generate a normalized two-bone strip with editable multi-influence weights.
pub fn grid(
    scene: &Scene,
    entity: &Entity,
    bones: [String; 2],
    columns: usize,
    rows: usize,
) -> Result<SkinMesh, String> {
    if columns < 2 || rows < 2 || columns > 16 || rows > 16 || columns * rows > 256 {
        return Err("Grid needs 2..16 rows/columns and at most 256 vertices".into());
    }
    let poses = kiln_core::hierarchy_transform::world_transforms(scene)?;
    let bind = BindPose::from_transform(*poses.get(&entity.id).ok_or("Missing mesh transform")?);
    let size = entity
        .components
        .iter()
        .find_map(|c| match c {
            Component::Sprite { width, height, .. } => Some([*width, *height]),
            _ => None,
        })
        .ok_or("Mesh needs Sprite")?;
    let layout = entity
        .components
        .iter()
        .find_map(|c| match c {
            Component::SpriteLayout { settings } => Some(*settings),
            _ => None,
        })
        .unwrap_or_default();
    layout.validate()?;
    if bones[0] == bones[1] || bones.iter().any(|b| b == &entity.id) {
        return Err("Choose two distinct bone entities other than the skin".into());
    }
    let mut binds = vec![];
    for id in bones {
        binds.push(BoneBind {
            pose: BindPose::from_transform(*poses.get(&id).ok_or("Missing bind bone")?),
            entity: id,
        });
    }
    let mut mesh = SkinMesh {
        bind,
        bones: binds,
        vertices: vec![],
        triangles: vec![],
    };
    for y in 0..rows {
        for x in 0..columns {
            let u = x as f32 / (columns - 1) as f32;
            let v = y as f32 / (rows - 1) as f32;
            let influences = if x == 0 {
                vec![Influence {
                    bone: 0,
                    weight: 1.0,
                }]
            } else if x == columns - 1 {
                vec![Influence {
                    bone: 1,
                    weight: 1.0,
                }]
            } else {
                vec![
                    Influence {
                        bone: 0,
                        weight: 1.0 - u,
                    },
                    Influence { bone: 1, weight: u },
                ]
            };
            mesh.vertices.push(SkinVertex {
                position: [
                    (u - layout.pivot[0]) * size[0] / layout.pixels_per_unit,
                    (v - layout.pivot[1]) * size[1] / layout.pixels_per_unit,
                ],
                uv: [u, v],
                influences,
            });
        }
    }
    for y in 0..rows - 1 {
        for x in 0..columns - 1 {
            let a = (y * columns + x) as u16;
            let b = a + 1;
            let d = a + columns as u16;
            let c = d + 1;
            mesh.triangles.extend([[a, b, c], [a, c, d]]);
        }
    }
    mesh.validate_scene(scene)?;
    Ok(mesh)
}
/// Edit bindings and weights without mutating the authored scene directly.
pub fn show(
    ui: &mut egui::Ui,
    state: &mut SkinEditor,
    scene: &Scene,
    entity: &Entity,
    scope: &str,
) -> Option<SkinMesh> {
    let authored = entity.components.iter().find_map(|c| match c {
        Component::SkinnedMesh { mesh } => Some(mesh),
        _ => None,
    });
    let source = serde_json::to_string(&authored).unwrap_or_default();
    let owner = format!("{scope}:{}", entity.id);
    if state.owner != owner || state.source != source {
        state.owner = owner;
        state.source = source;
        state.draft = authored.cloned();
        state.error.clear();
        state.columns = 5;
        state.rows = 2;
    }
    let mut applied = None;
    ui.collapsing("Soft Mesh Skinning",|ui|{
  for(index,bone)in state.bones.iter_mut().enumerate(){egui::ComboBox::from_id_salt(("skin_bone",index)).selected_text(if bone.is_empty(){"Choose bone"}else{bone.as_str()}).show_ui(ui,|ui|{for e in &scene.entities{if e.id!=entity.id{ui.selectable_value(bone,e.id.clone(),format!("{} ({})",e.name,e.id));}}});}
  ui.horizontal(|ui|{ui.add(egui::DragValue::new(&mut state.columns).range(2..=16).prefix("Columns "));ui.add(egui::DragValue::new(&mut state.rows).range(2..=16).prefix("Rows "));});
  if ui.button("Generate Grid and Capture Binds").clicked(){match grid(scene,entity,state.bones.clone(),state.columns,state.rows){Ok(mesh)=>{state.draft=Some(mesh);state.error.clear();},Err(e)=>state.error=e}}
  if let Some(mesh)=&mut state.draft{
   if ui.button("Recapture Bind Poses").clicked(){match kiln_core::hierarchy_transform::world_transforms(scene){Ok(poses)=>{mesh.bind=BindPose::from_transform(poses[&entity.id]);for b in &mut mesh.bones{if let Some(p)=poses.get(&b.entity){b.pose=BindPose::from_transform(*p);}}},Err(e)=>state.error=e}}
   ui.label("Bind poses use authored world coordinates. Animate bones after binding. Each vertex supports up to four normalized influences.");
   ui.collapsing("Bone bindings",|ui|{for bone in &mut mesh.bones{ui.text_edit_singleline(&mut bone.entity);}if mesh.bones.len()<64&&ui.button("Add Bone Binding").clicked(){if let Some(id)=scene.entities.iter().map(|e|&e.id).find(|id|!mesh.bones.iter().any(|b|&b.entity==*id)){if let Ok(poses)=kiln_core::hierarchy_transform::world_transforms(scene){mesh.bones.push(BoneBind{entity:id.clone(),pose:BindPose::from_transform(poses[id])});}}}});
   ui.collapsing("Vertices / Weights",|ui|{for(index,v)in mesh.vertices.iter_mut().enumerate(){ui.push_id(index,|ui|{ui.collapsing(format!("Vertex {index}"),|ui|{
    ui.horizontal(|ui|{for p in &mut v.position{ui.add(egui::DragValue::new(p).speed(0.1));}for uv in &mut v.uv{ui.add(egui::DragValue::new(uv).speed(0.01).range(0.0..=1.0));}});
    let mut remove=None;for(i,w)in v.influences.iter_mut().enumerate(){ui.horizontal(|ui|{ui.add(egui::DragValue::new(&mut w.bone).range(0..=mesh.bones.len().saturating_sub(1)as u16).prefix("Bone "));ui.add(egui::DragValue::new(&mut w.weight).speed(0.01).range(0.0..=1.0));if ui.small_button("Remove weight").clicked(){remove=Some(i);}});}
    if let Some(i)=remove{v.influences.remove(i);}
    if v.influences.len()<4&&ui.button("Add influence").clicked(){if let Some(b)=(0..mesh.bones.len()as u16).find(|b|!v.influences.iter().any(|i|i.bone==*b)){v.influences.push(Influence{bone:b,weight:0.1});}}
    if ui.button("Normalize weights").clicked(){let sum:f32=v.influences.iter().map(|i|i.weight).sum();if sum.is_finite()&&sum>0.0{for w in &mut v.influences{w.weight/=sum;}}}
   });});}});
   if ui.button("Apply Skinned Mesh").clicked(){match mesh.validate_scene(scene){Ok(())=>{applied=Some(mesh.clone());state.error.clear();},Err(e)=>state.error=e}}
  }
  if !state.error.is_empty(){ui.colored_label(egui::Color32::YELLOW,&state.error);}
 });
    applied
}
