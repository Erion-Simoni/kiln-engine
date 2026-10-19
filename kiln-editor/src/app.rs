//! Main editor application.

use std::sync::Arc;
use winit::{
    event_loop::{ActiveEventLoop, EventLoop},
    window::Window,
};
use thiserror::Error;
use kiln_core::Transform;
use kiln_runtime::{GameWorld, Sprite};
use kiln_graph::{Graph, Node, NodeType, ConstantValue, ArithmeticOp};
use kiln_render::RenderTier;
use hecs;

#[derive(Error, Debug)]
pub enum EditorError {
    #[error("Window error: {0}")]
    Window(String),
    #[error("GPU error: {0}")]
    Gpu(String),
}

pub struct EditorApp {
    gpu: Option<GpuState>,
    egui_winit: egui_winit::State,
    dock_state: egui_dock::DockState<Tab>,
    world: GameWorld,
    selected_entity: Option<hecs::Entity>,
    is_playing: bool,
    graph: Graph,
    render_tier: RenderTier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    SceneView,
    Inspector,
    GraphEditor,
    Assets,
    Console,
    Settings,
}

struct GpuState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    renderer: kiln_render::Renderer,
    egui_renderer: egui_wgpu::Renderer,
}

impl EditorApp {
    pub fn new(event_loop: &EventLoop<()>) -> Result<Self, EditorError> {
        let egui_ctx = egui::Context::default();

        // Apply dark theme similar to Unity/Unreal
        let mut style = egui::Style::default();
        style.visuals = egui::Visuals::dark();
        style.visuals.panel_fill = egui::Color32::from_rgb(30, 30, 35);
        style.visuals.window_fill = egui::Color32::from_rgb(35, 35, 40);
        style.visuals.faint_bg_color = egui::Color32::from_rgb(40, 40, 45);
        style.visuals.extreme_bg_color = egui::Color32::from_rgb(25, 25, 30);
        style.visuals.code_bg_color = egui::Color32::from_rgb(28, 28, 32);
        style.visuals.window_rounding = 4.0.into();
        style.visuals.window_shadow = egui::epaint::Shadow {
            offset: egui::vec2(0.0, 2.0),
            blur: 8.0,
            spread: 0.0,
            color: egui::Color32::from_rgba_premultiplied(0, 0, 0, 40),
        };
        style.visuals.widgets.open.rounding = 4.0.into();
        style.visuals.widgets.inactive.rounding = 4.0.into();
        style.visuals.widgets.hovered.rounding = 4.0.into();
        style.visuals.widgets.active.rounding = 4.0.into();
        style.visuals.selection.bg_fill = egui::Color32::from_rgb(66, 135, 245);
        style.visuals.selection.stroke = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(66, 135, 245));
        egui_ctx.set_style(style);

        let egui_winit = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            event_loop,
            None,
            None,
            None,
        );
        let dock_state = egui_dock::DockState::new(vec![Tab::SceneView]);

        let mut world = GameWorld::new();

        // Create demo scene with a few entities
        for i in 0..5 {
            let x = (i as f32) * 100.0 + 50.0;
            let y = 200.0;
            let r = (i as f32) / 4.0;
            world.spawn((
                Transform::new(x, y),
                Sprite::new(r, 0.5, 0.6, 1.0).with_size(64.0, 64.0),
            ));
        }

        Ok(Self {
            gpu: None,
            egui_winit,
            dock_state,
            world,
            selected_entity: None,
            is_playing: false,
            graph: Graph::new(),
            render_tier: RenderTier::default(),
        })
    }

    async fn init_gpu(&mut self, window: Arc<Window>) {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface: wgpu::Surface<'static> = instance
            .create_surface(window.clone())
            .expect("create surface");

        let renderer = kiln_render::Renderer::new(&instance, &surface, size.width, size.height).await;

        // Capture auto-selected tier
        self.render_tier = renderer.tier();

        let egui_renderer = egui_wgpu::Renderer::new(
            renderer.device(),
            renderer.format(),
            None,
            1,
            false,
        );

        self.gpu = Some(GpuState {
            window,
            surface,
            renderer,
            egui_renderer,
        });
    }
}

impl winit::application::ApplicationHandler for EditorApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    winit::window::WindowAttributes::default()
                        .with_title("Kiln Editor - Game Engine")
                        .with_inner_size(winit::dpi::LogicalSize::new(1600u32, 900u32)),
                )
                .expect("window"),
        );
        pollster::block_on(self.init_gpu(window));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        use winit::event::WindowEvent;

        let gpu = self.gpu.as_mut().unwrap();
        let response = self.egui_winit.on_window_event(&gpu.window, &event);

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    gpu.renderer.resize(&gpu.surface, size.width, size.height);
                }
            }
            _ => {}
        }

        if response.consumed {
            return;
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let gpu = self.gpu.as_mut().unwrap();
        let size = gpu.window.inner_size();
        if size.width == 0 || size.height == 0 {
            return;
        }

        // Apply tier change if needed
        let tier = self.render_tier;
        if gpu.renderer.tier() != tier {
            gpu.renderer.set_tier(tier);
        }

        let frame = match gpu.surface.get_current_texture() {
            Ok(f) => f,
            Err(e) => {
                log::warn!("frame error: {e}");
                return;
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = gpu
            .renderer
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame_encoder"),
            });

        // egui UI pass
        let raw_input = self.egui_winit.take_egui_input(&gpu.window);
        let ctx = self.egui_winit.egui_ctx();

        ctx.begin_pass(raw_input);

        // Build UI
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut dock_style = egui_dock::Style::from_egui(ui.style());
            dock_style.tab_bar.bg_fill = egui::Color32::from_rgb(30, 30, 35);
            dock_style.tab_bar.hline_color = egui::Color32::from_rgb(60, 60, 65);
            dock_style.separator.color_idle = egui::Color32::from_rgb(60, 60, 65);
            dock_style.separator.color_hovered = egui::Color32::from_rgb(80, 80, 90);
            dock_style.separator.color_dragged = egui::Color32::from_rgb(100, 100, 110);
            dock_style.separator.extra = 4.0;

            let tree = egui_dock::DockArea::new(&mut self.dock_state)
                .style(dock_style);

            let mut tab_viewer = TabViewer {
                world: &mut self.world,
                selected_entity: &mut self.selected_entity,
                is_playing: &mut self.is_playing,
                graph: &mut self.graph,
                render_tier: &mut self.render_tier,
            };
            tree.show(ctx, &mut tab_viewer);
        });

        let full_output = ctx.end_pass();
        let paint_jobs = ctx.tessellate(full_output.shapes, full_output.pixels_per_point);

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [size.width, size.height],
            pixels_per_point: full_output.pixels_per_point,
        };

        for (id, image_delta) in &full_output.textures_delta.set {
            gpu.egui_renderer.update_texture(
                gpu.renderer.device(),
                gpu.renderer.queue(),
                *id,
                image_delta,
            );
        }

        for id in &full_output.textures_delta.free {
            gpu.egui_renderer.free_texture(id);
        }

        gpu.egui_renderer.update_buffers(
            gpu.renderer.device(),
            gpu.renderer.queue(),
            &mut encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("main_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.1,
                        b: 0.12,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        let mut pass = pass.forget_lifetime();

        gpu.egui_renderer
            .render(&mut pass, &paint_jobs, &screen_descriptor);

        drop(pass);

        gpu.renderer.queue().submit([encoder.finish()]);
        frame.present();
        gpu.window.request_redraw();
    }
}

struct TabViewer<'a> {
    world: &'a mut GameWorld,
    selected_entity: &'a mut Option<hecs::Entity>,
    is_playing: &'a mut bool,
    graph: &'a mut Graph,
    render_tier: &'a mut RenderTier,
}

impl<'a> egui_dock::TabViewer for TabViewer<'a> {
    type Tab = Tab;

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            Tab::SceneView => {
                ui.vertical(|ui| {
                    // Toolbar
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);

                        // Play/Stop button with icon-like styling
                        let play_btn = if *self.is_playing {
                            ui.add_sized([100.0, 28.0], egui::Button::new("▶ Stop"))
                        } else {
                            ui.add_sized([100.0, 28.0], egui::Button::new("▶ Play"))
                        };
                        if play_btn.clicked() {
                            *self.is_playing = !*self.is_playing;
                        }

                        ui.separator();

                        if ui.button("Save Scene").clicked() {
                            // TODO: Save scene
                        }
                        if ui.button("Load Scene").clicked() {
                            // TODO: Load scene
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!("FPS: {:.0}", 60.0));
                        });
                    });

                    ui.add_space(8.0);

                    // Game view canvas
                    let desired_size = egui::vec2(ui.available_width(), ui.available_height() * 0.65);
                    let (rect, _response) = ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());

                    // Draw background with grid pattern
                    ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgb(18, 18, 20));

                    // Draw grid lines
                    let grid_size = 50.0;
                    let painter = ui.painter();
                    for x in (rect.min.x as i32..rect.max.x as i32).step_by(grid_size as usize) {
                        painter.line_segment(
                            [egui::pos2(x as f32, rect.min.y), egui::pos2(x as f32, rect.max.y)],
                            (1.0, egui::Color32::from_rgba_premultiplied(255, 255, 255, 15)),
                        );
                    }
                    for y in (rect.min.y as i32..rect.max.y as i32).step_by(grid_size as usize) {
                        painter.line_segment(
                            [egui::pos2(rect.min.x, y as f32), egui::pos2(rect.max.x, y as f32)],
                            (1.0, egui::Color32::from_rgba_premultiplied(255, 255, 255, 15)),
                        );
                    }

                    // Draw sprites as rectangles
                    let sprites: Vec<(Transform, Sprite)> = self.world.world()
                        .query::<(&Transform, &Sprite)>()
                        .iter()
                        .map(|(t, s)| (*t, *s))
                        .collect();

                    for (transform, sprite) in sprites {
                        let screen_x = rect.min.x + transform.position.x;
                        let screen_y = rect.min.y + transform.position.y;
                        let sprite_rect = egui::Rect::from_min_size(
                            egui::pos2(screen_x, screen_y),
                            egui::vec2(sprite.size[0], sprite.size[1]),
                        );

                        let color = egui::Color32::from_rgba_premultiplied(
                            (sprite.color[0] * 255.0) as u8,
                            (sprite.color[1] * 255.0) as u8,
                            (sprite.color[2] * 255.0) as u8,
                            (sprite.color[3] * 255.0) as u8,
                        );

                        painter.rect_filled(sprite_rect, 2.0, color);
                        painter.rect_stroke(sprite_rect, 2.0, egui::Stroke::new(1.0_f32, egui::Color32::from_rgba_premultiplied(255, 255, 255, 50)));
                    }

                    // Entity list for selection
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.heading("Entities");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("+ Add Entity").clicked() {
                                // TODO: Add entity
                            }
                        });
                    });

                    ui.add_space(4.0);

                    egui::ScrollArea::vertical()
                        .max_height(ui.available_height())
                        .show(ui, |ui| {
                            let entities: Vec<(hecs::Entity, Transform)> = self.world.world()
                                .query::<(hecs::Entity, &Transform, &Sprite)>()
                                .iter()
                                .map(|(e, t, _s)| (e, *t))
                                .collect();
                            for (entity, transform) in entities {
                                let label = format!("Entity ({:.0}, {:.0})", transform.position.x, transform.position.y);
                                let selected = *self.selected_entity == Some(entity);
                                if ui.selectable_label(selected, label).clicked() {
                                    *self.selected_entity = Some(entity);
                                }
                            }
                        });
                });
            }
            Tab::Inspector => {
                ui.vertical(|ui| {
                    ui.heading("Inspector");

                    if let Some(entity) = self.selected_entity {
                        ui.label(format!("Entity ID: {:?}", entity.id()));
                        ui.add_space(8.0);

                        // Read current values once
                        let world = self.world.world();
                        let transform = world.get::<&Transform>(*entity).ok().map(|t| *t);
                        let sprite = world.get::<&Sprite>(*entity).ok().map(|s| *s);

                        // Transform component
                        if let Some(mut transform) = transform {
                            ui.add_space(4.0);
                            egui::Frame::none()
                                .fill(egui::Color32::from_rgb(35, 35, 40))
                                .rounding(4.0)
                                .inner_margin(12.0)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.strong("Transform");
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if ui.button("⋮").clicked() {
                                                // TODO: Component menu
                                            }
                                        });
                                    });
                                    ui.add_space(8.0);

                                    ui.horizontal(|ui| {
                                        ui.label("Position");
                                        ui.add_space(16.0);
                                        ui.label("X:");
                                        if ui.add(egui::DragValue::new(&mut transform.position.x).speed(1.0)).changed() {
                                            let world = self.world.world_mut();
                                            if let Ok(mut t) = world.get::<&mut Transform>(*entity) {
                                                t.position.x = transform.position.x;
                                            }
                                        }
                                        ui.add_space(8.0);
                                        ui.label("Y:");
                                        if ui.add(egui::DragValue::new(&mut transform.position.y).speed(1.0)).changed() {
                                            let world = self.world.world_mut();
                                            if let Ok(mut t) = world.get::<&mut Transform>(*entity) {
                                                t.position.y = transform.position.y;
                                            }
                                        }
                                    });
                                });
                        }

                        // Sprite component
                        if let Some(mut sprite) = sprite {
                            ui.add_space(8.0);
                            egui::Frame::none()
                                .fill(egui::Color32::from_rgb(35, 35, 40))
                                .rounding(4.0)
                                .inner_margin(12.0)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.strong("Sprite");
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if ui.button("⋮").clicked() {
                                                // TODO: Component menu
                                            }
                                        });
                                    });
                                    ui.add_space(8.0);

                                    ui.horizontal(|ui| {
                                        ui.label("Color");
                                        ui.add_space(16.0);
                                        ui.label("R:");
                                        if ui.add(egui::DragValue::new(&mut sprite.color[0]).speed(0.01).range(0.0..=1.0)).changed() {
                                            let world = self.world.world_mut();
                                            if let Ok(mut s) = world.get::<&mut Sprite>(*entity) {
                                                s.color[0] = sprite.color[0];
                                            }
                                        }
                                        ui.add_space(8.0);
                                        ui.label("G:");
                                        if ui.add(egui::DragValue::new(&mut sprite.color[1]).speed(0.01).range(0.0..=1.0)).changed() {
                                            let world = self.world.world_mut();
                                            if let Ok(mut s) = world.get::<&mut Sprite>(*entity) {
                                                s.color[1] = sprite.color[1];
                                            }
                                        }
                                        ui.add_space(8.0);
                                        ui.label("B:");
                                        if ui.add(egui::DragValue::new(&mut sprite.color[2]).speed(0.01).range(0.0..=1.0)).changed() {
                                            let world = self.world.world_mut();
                                            if let Ok(mut s) = world.get::<&mut Sprite>(*entity) {
                                                s.color[2] = sprite.color[2];
                                            }
                                        }
                                    });

                                    ui.add_space(8.0);

                                    // Color preview
                                    let color_rect = egui::Rect::from_min_size(
                                        ui.cursor().left_top(),
                                        egui::vec2(40.0, 24.0),
                                    );
                                    let preview_color = egui::Color32::from_rgba_premultiplied(
                                        (sprite.color[0] * 255.0) as u8,
                                        (sprite.color[1] * 255.0) as u8,
                                        (sprite.color[2] * 255.0) as u8,
                                        (sprite.color[3] * 255.0) as u8,
                                    );
                                    ui.painter().rect_filled(color_rect, 2.0, preview_color);
                                    ui.painter().rect_stroke(color_rect, 2.0, egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(100, 100, 100)));
                                    ui.add_space(32.0);
                                });
                        }

                        ui.add_space(16.0);
                        if ui.button("+ Add Component").clicked() {
                            // TODO: Add component
                        }
                    } else {
                        ui.add_space(32.0);
                        ui.centered_and_justified(|ui| {
                            ui.label("No entity selected");
                            ui.add_space(8.0);
                            ui.label("Select an entity from the Scene View to inspect its components");
                        });
                    }
                });
            }
            Tab::GraphEditor => {
                ui.vertical(|ui| {
                    // Toolbar
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);

                        if ui.button("Add Constant").clicked() {
                            let node = Node::new(
                                NodeType::Constant {
                                    value: ConstantValue::Integer(0),
                                },
                                (100.0, 100.0),
                            );
                            self.graph.add_node(node);
                        }
                        if ui.button("Add Add").clicked() {
                            let node = Node::new(
                                NodeType::Arithmetic {
                                    op: ArithmeticOp::Add,
                                },
                                (200.0, 100.0),
                            );
                            self.graph.add_node(node);
                        }
                        if ui.button("Add Output").clicked() {
                            let node = Node::new(NodeType::Output, (300.0, 100.0));
                            self.graph.add_node(node);
                        }

                        ui.separator();

                        if ui.button("Clear").clicked() {
                            *self.graph = Graph::new();
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label("Node Graph");
                        });
                    });

                    ui.add_space(8.0);

                    // Graph canvas
                    let desired_size = egui::vec2(ui.available_width(), ui.available_height());
                    let (rect, _response) = ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());

                    // Draw background
                    ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgb(18, 18, 20));

                    // Draw grid
                    let grid_size = 50.0;
                    let painter = ui.painter();
                    for x in (rect.min.x as i32..rect.max.x as i32).step_by(grid_size as usize) {
                        painter.line_segment(
                            [egui::pos2(x as f32, rect.min.y), egui::pos2(x as f32, rect.max.y)],
                            (1.0, egui::Color32::from_rgba_premultiplied(255, 255, 255, 15)),
                        );
                    }
                    for y in (rect.min.y as i32..rect.max.y as i32).step_by(grid_size as usize) {
                        painter.line_segment(
                            [egui::pos2(rect.min.x, y as f32), egui::pos2(rect.max.x, y as f32)],
                            (1.0, egui::Color32::from_rgba_premultiplied(255, 255, 255, 15)),
                        );
                    }

                    // Draw nodes
                    for node in self.graph.nodes.values() {
                        let pos = egui::pos2(node.position.0, node.position.1);
                        let size = egui::vec2(140.0, 70.0);
                        let rect = egui::Rect::from_min_size(pos, size);

                        // Draw node background
                        painter.rect_filled(rect, 6.0, egui::Color32::from_rgb(45, 45, 50));
                        painter.rect_stroke(rect, 6.0, egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(90, 90, 100)));

                        // Draw node header
                        let header_rect = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), 24.0));
                        painter.rect_filled(header_rect, 6.0, egui::Color32::from_rgb(66, 135, 245));

                        // Draw node title
                        let title = match &node.node_type {
                            NodeType::Constant { .. } => "Constant",
                            NodeType::Arithmetic { .. } => "Add",
                            NodeType::Output => "Output",
                            _ => "Node",
                        };
                        painter.text(
                            header_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            title,
                            egui::FontId::proportional(13.0),
                            egui::Color32::WHITE,
                        );
                    }
                });
            }
            Tab::Assets => {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Assets");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Import").clicked() {
                                // TODO: Import assets
                            }
                        });
                    });

                    ui.add_space(8.0);

                    // Asset browser placeholder
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(35, 35, 40))
                        .rounding(4.0)
                        .inner_margin(16.0)
                        .show(ui, |ui| {
                            ui.centered_and_justified(|ui| {
                                ui.label("Asset browser will appear here");
                                ui.add_space(8.0);
                                ui.label("Drag and drop assets to import");
                            });
                        });
                });
            }
            Tab::Console => {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Console");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Clear").clicked() {
                                // TODO: Clear console
                            }
                        });
                    });

                    ui.add_space(8.0);

                    // Console log placeholder
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(35, 35, 40))
                        .rounding(4.0)
                        .inner_margin(16.0)
                        .show(ui, |ui| {
                            ui.label("[INFO] System initialized");
                            ui.label("[INFO] Editor ready");
                            ui.label("[WARN] GPU driver outdated (Intel)");
                            ui.label("[INFO] WGPU backend initialized");
                        });
                });
            }
            Tab::Settings => {
                ui.vertical(|ui| {
                    ui.heading("Settings");
                    ui.add_space(8.0);

                    // Render Quality section
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(35, 35, 40))
                        .rounding(4.0)
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.strong("Render Quality");
                            ui.add_space(8.0);

                            ui.label("Current tier:");
                            ui.label(format!("{:?}", self.render_tier));
                            ui.add_space(8.0);

                            ui.label("Force tier (takes effect on next frame):");
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
                                if ui.button("Low").clicked() {
                                    *self.render_tier = RenderTier::Low;
                                }
                                if ui.button("Medium").clicked() {
                                    *self.render_tier = RenderTier::Medium;
                                }
                                if ui.button("High").clicked() {
                                    *self.render_tier = RenderTier::High;
                                }
                            });
                            ui.add_space(8.0);

                            ui.label("Enabled effects:");
                            ui.label(format!("  Bloom passes: {}", self.render_tier.max_bloom_passes()));
                            ui.label(format!("  LUT color grading: {}", self.render_tier.enable_lut()));
                            ui.label(format!("  Screen-space fog: {}", self.render_tier.enable_fog()));
                            ui.label(format!("  Outline effect: {}", self.render_tier.enable_outline()));
                        });

                    ui.add_space(8.0);

                    // Export section
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(35, 35, 40))
                        .rounding(4.0)
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.strong("Export");
                            ui.add_space(8.0);

                            ui.label("Export project for web (WASM):");
                            ui.add_space(4.0);

                            if ui.button("Export to Web").clicked() {
                                // Trigger export via CLI (non-blocking for UI)
                                std::thread::spawn(|| {
                                    let _ = std::process::Command::new("cargo")
                                        .args(["run", "--bin", "kiln-cli", "--", "export", "--output", "web-export"])
                                        .status();
                                });
                            }

                            ui.add_space(4.0);
                            ui.label("Click to export. Check terminal for progress.");
                        });
                });
            }
        }
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            Tab::SceneView => "Scene View".into(),
            Tab::Inspector => "Inspector".into(),
            Tab::GraphEditor => "Graph Editor".into(),
            Tab::Assets => "Assets".into(),
            Tab::Console => "Console".into(),
            Tab::Settings => "Settings".into(),
        }
    }
}
