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
            backends: wgpu::Backends::PRIMARY,
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
                        .with_title("Kiln Editor")
                        .with_inner_size(winit::dpi::LogicalSize::new(1280u32, 720u32)),
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
            let tree = egui_dock::DockArea::new(&mut self.dock_state)
                .style(egui_dock::Style::from_egui(ui.style()));

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
                ui.heading("Scene View");

                // Play/Stop button
                ui.horizontal(|ui| {
                    if ui.button(if *self.is_playing { "Stop" } else { "Play" }).clicked() {
                        *self.is_playing = !*self.is_playing;
                    }
                    if *self.is_playing {
                        ui.label("(Running)");
                    }
                });

                // Game view canvas
                let desired_size = egui::vec2(ui.available_width(), ui.available_height() * 0.6);
                let (rect, _response) = ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());

                // Draw background
                ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgb(51, 51, 64));

                // Draw sprites as rectangles
                let sprites: Vec<(Transform, Sprite)> = self.world.world()
                    .query::<(&Transform, &Sprite)>()
                    .iter()
                    .map(|(t, s)| (*t, *s))
                    .collect();

                let painter = ui.painter();
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

                    painter.rect_filled(sprite_rect, 0.0, color);
                }

                // Entity list for selection
                ui.separator();
                ui.heading("Entities");
                let entities: Vec<(hecs::Entity, Transform)> = self.world.world()
                    .query::<(hecs::Entity, &Transform, &Sprite)>()
                    .iter()
                    .map(|(e, t, _s)| (e, *t))
                    .collect();
                for (entity, transform) in entities {
                    let label = format!("Entity {:?} at ({:.1}, {:.1})", entity, transform.position.x, transform.position.y);
                    if ui.selectable_label(*self.selected_entity == Some(entity), label).clicked() {
                        *self.selected_entity = Some(entity);
                    }
                }
            }
            Tab::Inspector => {
                ui.heading("Inspector");
                if let Some(entity) = self.selected_entity {
                    ui.label(format!("Selected: {:?}", entity));

                    // Read current values once
                    let world = self.world.world();
                    let transform = world.get::<&Transform>(*entity).ok().map(|t| *t);
                    let sprite = world.get::<&Sprite>(*entity).ok().map(|s| *s);

                    // Transform component
                    if let Some(mut transform) = transform {
                        ui.separator();
                        ui.heading("Transform");
                        ui.horizontal(|ui| {
                            ui.label("X:");
                            if ui.add(egui::DragValue::new(&mut transform.position.x).speed(1.0)).changed() {
                                let world = self.world.world_mut();
                                if let Ok(mut t) = world.get::<&mut Transform>(*entity) {
                                    t.position.x = transform.position.x;
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Y:");
                            if ui.add(egui::DragValue::new(&mut transform.position.y).speed(1.0)).changed() {
                                let world = self.world.world_mut();
                                if let Ok(mut t) = world.get::<&mut Transform>(*entity) {
                                    t.position.y = transform.position.y;
                                }
                            }
                        });
                    }

                    // Sprite component
                    if let Some(mut sprite) = sprite {
                        ui.separator();
                        ui.heading("Sprite");
                        ui.horizontal(|ui| {
                            ui.label("R:");
                            if ui.add(egui::DragValue::new(&mut sprite.color[0]).speed(0.01).range(0.0..=1.0)).changed() {
                                let world = self.world.world_mut();
                                if let Ok(mut s) = world.get::<&mut Sprite>(*entity) {
                                    s.color[0] = sprite.color[0];
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("G:");
                            if ui.add(egui::DragValue::new(&mut sprite.color[1]).speed(0.01).range(0.0..=1.0)).changed() {
                                let world = self.world.world_mut();
                                if let Ok(mut s) = world.get::<&mut Sprite>(*entity) {
                                    s.color[1] = sprite.color[1];
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("B:");
                            if ui.add(egui::DragValue::new(&mut sprite.color[2]).speed(0.01).range(0.0..=1.0)).changed() {
                                let world = self.world.world_mut();
                                if let Ok(mut s) = world.get::<&mut Sprite>(*entity) {
                                    s.color[2] = sprite.color[2];
                                }
                            }
                        });
                    }
                } else {
                    ui.label("No entity selected.");
                }
            }
            Tab::GraphEditor => {
                ui.heading("Graph Editor");

                // Add node buttons
                ui.horizontal(|ui| {
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
                });

                ui.separator();

                // Draw nodes
                let painter = ui.painter();
                for node in self.graph.nodes.values() {
                    let pos = egui::pos2(node.position.0, node.position.1);
                    let size = egui::vec2(120.0, 60.0);
                    let rect = egui::Rect::from_min_size(pos, size);

                    // Draw node background
                    painter.rect_filled(rect, 5.0, egui::Color32::from_rgb(60, 60, 70));
                    painter.rect_stroke(rect, 5.0, egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(100, 100, 120)));

                    // Draw node title
                    let title = match &node.node_type {
                        NodeType::Constant { .. } => "Constant",
                        NodeType::Arithmetic { .. } => "Add",
                        NodeType::Output => "Output",
                        _ => "Node",
                    };
                    painter.text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        title,
                        egui::FontId::default(),
                        egui::Color32::WHITE,
                    );
                }
            }
            Tab::Assets => {
                ui.heading("Assets");
                ui.label("Asset browser will appear here.");
            }
            Tab::Console => {
                ui.heading("Console");
                ui.label("Logs and messages will appear here.");
            }
            Tab::Settings => {
                ui.heading("Settings");

                ui.separator();
                ui.heading("Render Quality");

                ui.label("Current tier:");
                ui.label(format!("{:?}", self.render_tier));

                ui.separator();

                ui.label("Force tier (takes effect on next frame):");
                ui.horizontal(|ui| {
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

                ui.separator();

                // Show tier capabilities
                ui.label("Enabled effects:");
                ui.label(format!("Bloom passes: {}", self.render_tier.max_bloom_passes()));
                ui.label(format!("LUT color grading: {}", self.render_tier.enable_lut()));
                ui.label(format!("Screen-space fog: {}", self.render_tier.enable_fog()));
                ui.label(format!("Outline effect: {}", self.render_tier.enable_outline()));
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
