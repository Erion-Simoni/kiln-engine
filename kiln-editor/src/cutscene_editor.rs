//! Visual key editing, scrub preview and guarded handoff to the Source Editor.
use crate::cutscene::{property_name, sample, Cutscene, Key, Track};
use kiln_core::{Component, Entity, Scene};
use kiln_runtime::tweens::{TweenEase, TweenProperty as Property};
#[derive(Debug)]
pub struct Composer {
    pub draft: Cutscene,
    pub path: String,
    pub message: String,
    time: f64,
    playing: bool,
    entity: String,
    property: Property,
    scope: String,
    undo: Vec<Cutscene>,
    redo: Vec<Cutscene>,
}
impl Default for Composer {
    fn default() -> Self {
        Self {
            draft: Cutscene::default(),
            path: "cutscene.ks".into(),
            message: String::new(),
            time: 0.0,
            playing: false,
            entity: String::new(),
            property: Property::Position,
            scope: String::new(),
            undo: vec![],
            redo: vec![],
        }
    }
}
fn initial(entity: &Entity, property: Property) -> Vec<f64> {
    for c in &entity.components {
        match (c, property) {
            (Component::Transform { x, y, .. }, Property::Position) => {
                return vec![*x as f64, *y as f64]
            }
            (Component::Transform { rotation, .. }, Property::Rotation) => {
                return vec![*rotation as f64]
            }
            (Component::Transform { scale, .. }, Property::Scale) => return vec![*scale as f64; 2],
            (Component::Sprite { color, .. }, Property::Color) => {
                return color.iter().map(|v| *v as f64).collect()
            }
            _ => {}
        }
    }
    match property {
        Property::Position => vec![0.0; 2],
        Property::Rotation => vec![0.0],
        Property::Scale => vec![1.0; 2],
        Property::Color => vec![1.0; 4],
    }
}
impl Composer {
    pub fn undo(&mut self) {
        if let Some(previous) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.draft, previous));
        }
    }
    pub fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.draft, next));
        }
    }
    fn record(&mut self, before: Cutscene) {
        if before != self.draft {
            self.undo.push(before);
            self.redo.clear();
            if self.undo.len() > 64 {
                self.undo.remove(0);
            }
        }
    }
    /// Returns generated source only; disk writes use ScriptEditor atomic Save.
    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        scene: &Scene,
        source: (&str, &str),
        scope: &str,
        dt: f32,
    ) -> Option<String> {
        if self.scope != scope {
            *self = Self {
                scope: scope.into(),
                ..Default::default()
            };
        }
        let mut generated = None;
        ui.collapsing("Cutscene Composer",|ui|{
            ui.label("Add entity tracks and keys, scrub values, then open generated source and Save Script. Attach the saved file to one director entity in the Inspector.");
            ui.label("Draft changes stay here until generated and saved. Reopen saved keys with Load from Source Editor.");
            ui.horizontal(|ui|{
                if ui.button("Undo Timeline").clicked(){self.undo();}
                if ui.button("Redo Timeline").clicked(){self.redo();}
                if ui.button("Load from Source Editor").clicked(){
                    match Cutscene::from_source(source.0).and_then(|d|{d.validate_scene(scene)?;Ok(d)}) {
                        Ok(doc)=>{let before=std::mem::replace(&mut self.draft,doc);self.record(before);self.path=source.1.into();self.message="Loaded timeline keys from source".into();self.playing=false;self.time=0.0;},
                        Err(e)=>self.message=e,
                    }
                }
            });
            let before=self.draft.clone();
            ui.horizontal(|ui|{
                egui::ComboBox::from_id_salt("cutscene_entity").selected_text(if self.entity.is_empty(){"Choose entity"}else{&self.entity}).show_ui(ui,|ui|{
                    for entity in &scene.entities {ui.selectable_value(&mut self.entity,entity.id.clone(),&entity.name);}
                });
                egui::ComboBox::from_id_salt("cutscene_property").selected_text(property_name(self.property)).show_ui(ui,|ui|{
                    for p in [Property::Position,Property::Rotation,Property::Scale,Property::Color]{ui.selectable_value(&mut self.property,p,property_name(p));}
                });
                if ui.add_enabled(self.draft.tracks.len()<16,egui::Button::new("Add Track")).clicked(){
                    if let Some(entity)=scene.entities.iter().find(|e|e.id==self.entity) {
                        if !self.draft.tracks.iter().any(|t|t.entity==self.entity&&t.property==self.property){
                            let value=initial(entity,self.property);
                            self.draft.tracks.push(Track{entity:self.entity.clone(),property:self.property,keys:vec![Key{time:0.0,value:value.clone(),ease:TweenEase::Linear},Key{time:1.0,value,ease:TweenEase::Smooth}]});
                        }
                    }
                }
            });
            let duration=self.draft.duration().max(0.001);
            if self.playing {self.time=(self.time+f64::from(dt.max(0.0))).min(duration);if self.time>=duration{self.playing=false;}ui.ctx().request_repaint();}
            ui.horizontal(|ui|{
                if ui.button(if self.playing{"Pause Preview"}else{"Play Values"}).clicked(){if self.time>=duration{self.time=0.0;}self.playing = !self.playing;}
                if ui.add(egui::Slider::new(&mut self.time,0.0..=duration).text("Seconds")).changed(){self.playing=false;}
            });
            let total_keys:usize=self.draft.tracks.iter().map(|t|t.keys.len()).sum();
            let mut remove_track=None;
            for (index,track) in self.draft.tracks.iter_mut().enumerate(){
                ui.push_id(index,|ui|{
                    ui.separator();
                    ui.horizontal(|ui|{ui.strong(format!("{} / {}",track.entity,property_name(track.property)));if ui.small_button("Remove Track").clicked(){remove_track=Some(index);}});
                    let (rect,response)=ui.allocate_exact_size(egui::vec2(ui.available_width().max(80.0),24.0),egui::Sense::click_and_drag());
                    ui.painter().rect_filled(rect,2.0,egui::Color32::from_gray(30));
                    for key in &track.keys {let x=rect.left()+rect.width()*(key.time/duration)as f32;ui.painter().circle_filled(egui::pos2(x,rect.center().y),4.0,egui::Color32::LIGHT_BLUE);}
                    let x=rect.left()+rect.width()*(self.time/duration)as f32;
                    ui.painter().line_segment([egui::pos2(x,rect.top()),egui::pos2(x,rect.bottom())],egui::Stroke::new(1.0_f32,egui::Color32::YELLOW));
                    if response.clicked()||response.dragged(){if let Some(pos)=response.interact_pointer_pos(){self.time=f64::from(((pos.x-rect.left())/rect.width()).clamp(0.0,1.0))*duration;self.playing=false;}}
                    ui.monospace(format!("Value preview: {:?}",sample(track,self.time)));
                    let mut remove_key=None;
                    for (ki,key) in track.keys.iter_mut().enumerate(){
                        ui.push_id(ki,|ui|{ui.horizontal_wrapped(|ui|{
                            ui.label(format!("Key {ki}"));ui.add_enabled(ki>0,egui::DragValue::new(&mut key.time).speed(0.05).range(0.0..=60.0).prefix("t "));
                            for value in &mut key.value{ui.add(egui::DragValue::new(value).speed(0.05));}
                            egui::ComboBox::from_id_salt("ease").selected_text(format!("{:?}",key.ease)).show_ui(ui,|ui|{for e in [TweenEase::Linear,TweenEase::Smooth,TweenEase::In,TweenEase::Out]{ui.selectable_value(&mut key.ease,e,format!("{e:?}"));}});
                            if ki>0&&ui.small_button("Remove Key").clicked(){remove_key=Some(ki);}
                        });});
                    }
                    if let Some(i)=remove_key{track.keys.remove(i);}
                    if ui.add_enabled(total_keys<64&&track.keys.last().is_some_and(|k|k.time<60.0),egui::Button::new("Add Key")).clicked(){if let Some(last)=track.keys.last().cloned(){track.keys.push(Key{time:(last.time+1.0).min(60.0),..last});}}
                    if ui.small_button("Sort Keys by Time").clicked(){track.keys.sort_by(|a,b|a.time.total_cmp(&b.time));}
                });
            }
            if let Some(i)=remove_track{self.draft.tracks.remove(i);}
            ui.collapsing("Runtime control actions",|ui|{for (label,action) in ["Pause","Restart","Seek midpoint","Cancel"].into_iter().zip(&mut self.draft.actions){ui.horizontal(|ui|{ui.label(label);ui.text_edit_singleline(action);});}ui.label("Bind these action names in Project input settings; empty disables that control.");});
            self.record(before);
            ui.horizontal(|ui|{ui.label("Source path");ui.text_edit_singleline(&mut self.path);});
            if ui.button("Open Generated Source").clicked(){match self.draft.validate_scene(scene).and_then(|_|self.draft.source()){Ok(source)=>generated=Some(source),Err(e)=>self.message=e}}
            if let Err(e)=self.draft.validate_scene(scene){ui.colored_label(egui::Color32::YELLOW,e);}
            if !self.message.is_empty(){ui.label(&self.message);}
        });
        generated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(
        ctx: &egui::Context,
        composer: &mut Composer,
        scene: &Scene,
        events: Vec<egui::Event>,
    ) -> (egui::FullOutput, Option<String>) {
        let mut result = None;
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1100.0, 1400.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    result = composer.render(ui, scene, ("", ""), "widgets", 0.0);
                });
            },
        );
        (output, result)
    }
    fn click(
        ctx: &egui::Context,
        composer: &mut Composer,
        scene: &Scene,
        label: &str,
    ) -> Option<String> {
        let (output, _) = frame(ctx, composer, scene, vec![]);
        let pos = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.job.text == label => {
                    Some(t.pos + t.galley.size() * 0.5)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("Missing widget: {label}"));
        frame(
            ctx,
            composer,
            scene,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        frame(
            ctx,
            composer,
            scene,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        )
        .1
    }
    #[test]
    fn widgets_add_keys_undo_redo_and_generate_reopenable_source() {
        let scene = Scene {
            id: "main".into(),
            name: "Main".into(),
            script: None,
            nested_scenes: vec![],
            entities: vec![Entity {
                id: "actor".into(),
                name: "Actor".into(),
                components: vec![Component::Transform {
                    x: 80.0,
                    y: 120.0,
                    rotation: 0.0,
                    scale: 1.0,
                }],
            }],
        };
        let ctx = egui::Context::default();
        ctx.style_mut(|s| s.animation_time = 0.0);
        let mut composer = Composer {
            scope: "widgets".into(),
            entity: "actor".into(),
            ..Default::default()
        };
        click(&ctx, &mut composer, &scene, "Cutscene Composer");
        click(&ctx, &mut composer, &scene, "Add Track");
        assert_eq!(composer.draft.tracks.len(), 1);
        click(&ctx, &mut composer, &scene, "Add Key");
        assert_eq!(composer.draft.tracks[0].keys.len(), 3);
        click(&ctx, &mut composer, &scene, "Undo Timeline");
        assert_eq!(composer.draft.tracks[0].keys.len(), 2);
        click(&ctx, &mut composer, &scene, "Redo Timeline");
        assert_eq!(composer.draft.tracks[0].keys.len(), 3);
        let source =
            click(&ctx, &mut composer, &scene, "Open Generated Source").expect("generated source");
        assert_eq!(Cutscene::from_source(&source).unwrap(), composer.draft);
        click(&ctx, &mut composer, &scene, "Play Values");
        frame(&ctx, &mut composer, &scene, vec![]);
        assert!(composer.playing);
    }
}
