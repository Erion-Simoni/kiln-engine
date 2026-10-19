//! Main editor application.

use hecs;
use kiln_core::Transform;
use kiln_graph::{ArithmeticOp, ConstantValue, Graph, Node, NodeType};
use kiln_render::RenderTier;
use kiln_runtime::{GameWorld, Sprite};
use kiln_ui::{Button, Label, List, Panel, Widget, WidgetId};
use kiln_gameplay::{Dialogue, Quest, DataTable};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use winit::{
    event_loop::{ActiveEventLoop, EventLoop},
    window::Window,
};

use crate::prefab_editor::PrefabEditorState;

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
    show_welcome: bool,
    fps: f32,
    frame_count: u32,
    ui_designer: UIDesignerState,
    prefab_editor: PrefabEditorState,
    gameplay_editor: GameplayEditorState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum Tab {
    SceneView,
    Inspector,
    GraphEditor,
    Assets,
    Console,
    Settings,
    UIDesigner,
    PrefabEditor,
    SceneEditor,
    GameplayEditor,
}

struct GpuState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    renderer: kiln_render::Renderer,
    egui_renderer: egui_wgpu::Renderer,
}

/// UI designer state for the visual UI editor.
#[derive(Debug, Clone)]
enum UiWidget {
    Panel(Panel),
    Button(Button),
    Label(Label),
    List(List),
}

impl UiWidget {
    fn as_widget(&self) -> &dyn Widget {
        match self {
            UiWidget::Panel(p) => p,
            UiWidget::Button(b) => b,
            UiWidget::Label(l) => l,
            UiWidget::List(l) => l,
        }
    }

    fn as_widget_mut(&mut self) -> &mut dyn Widget {
        match self {
            UiWidget::Panel(p) => p,
            UiWidget::Button(b) => b,
            UiWidget::Label(l) => l,
            UiWidget::List(l) => l,
        }
    }

    fn name(&self) -> &str {
        match self {
            UiWidget::Panel(_) => "Panel",
            UiWidget::Button(_) => "Button",
            UiWidget::Label(_) => "Label",
            UiWidget::List(_) => "List",
        }
    }
}

/// Hierarchy node for UI designer tree.
#[derive(Debug, Clone)]
struct UiNode {
    widget: UiWidget,
    children: Vec<WidgetId>,
    parent: Option<WidgetId>,
}

/// Gameplay editor state for dialogue, quests, and data tables.
struct GameplayEditorState {
    dialogues: HashMap<String, Dialogue>,
    quests: HashMap<String, Quest>,
    data_tables: HashMap<String, DataTable>,
    selected_dialogue: Option<String>,
    selected_quest: Option<String>,
    selected_table: Option<String>,
}

impl GameplayEditorState {
    fn new() -> Self {
        Self {
            dialogues: HashMap::new(),
            quests: HashMap::new(),
            data_tables: HashMap::new(),
            selected_dialogue: None,
            selected_quest: None,
            selected_table: None,
        }
    }

    fn add_dialogue(&mut self, dialogue: Dialogue) {
        self.dialogues.insert(dialogue.id.clone(), dialogue);
    }

    fn add_quest(&mut self, quest: Quest) {
        self.quests.insert(quest.id.clone(), quest);
    }

    fn add_data_table(&mut self, table: DataTable) {
        self.data_tables.insert(table.id.clone(), table);
    }
}

struct UIDesignerState {
    root_nodes: Vec<WidgetId>,
    widgets: HashMap<WidgetId, UiNode>,
    selected_widget: Option<WidgetId>,
}

impl UIDesignerState {
    fn new() -> Self {
        Self {
            root_nodes: Vec::new(),
            widgets: HashMap::new(),
            selected_widget: None,
        }
    }

    fn add_widget(&mut self, widget: UiWidget, parent: Option<WidgetId>) -> WidgetId {
        let id = widget.as_widget().id();
        let node = UiNode {
            widget,
            children: Vec::new(),
            parent,
        };

        self.widgets.insert(id, node);

        if let Some(parent_id) = parent {
            if let Some(parent_node) = self.widgets.get_mut(&parent_id) {
                parent_node.children.push(id);
            }
        } else {
            self.root_nodes.push(id);
        }

        id
    }

    fn remove_widget(&mut self, id: WidgetId) {
        if let Some(node) = self.widgets.remove(&id) {
            // Remove from parent's children
            if let Some(parent_id) = node.parent {
                if let Some(parent_node) = self.widgets.get_mut(&parent_id) {
                    parent_node.children.retain(|child_id| *child_id != id);
                }
            } else {
                self.root_nodes.retain(|root_id| *root_id != id);
            }

            // Recursively remove children
            for child_id in node.children {
                self.remove_widget(child_id);
            }
        }

        if self.selected_widget == Some(id) {
            self.selected_widget = None;
        }
    }

    fn get_widget(&self, id: WidgetId) -> Option<&UiWidget> {
        self.widgets.get(&id).map(|node| &node.widget)
    }

    fn get_widget_mut(&mut self, id: WidgetId) -> Option<&mut UiWidget> {
        self.widgets.get_mut(&id).map(|node| &mut node.widget)
    }
}

impl Default for UIDesignerState {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorApp {
    pub fn new(event_loop: &EventLoop<()>) -> Result<Self, EditorError> {
        let egui_ctx = egui::Context::default();

        // Apply dark theme similar to Unity/Unreal
        let mut style = egui::Style {
            visuals: egui::Visuals::dark(),
            ..Default::default()
        };
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
        style.visuals.selection.stroke =
            egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(66, 135, 245));
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
            show_welcome: true,
            fps: 60.0,
            frame_count: 0,
            ui_designer: UIDesignerState::new(),
            prefab_editor: PrefabEditorState::new(),
            gameplay_editor: GameplayEditorState::new(),
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

        let renderer =
            kiln_render::Renderer::new(&instance, &surface, size.width, size.height).await;

        // Capture auto-selected tier
        self.render_tier = renderer.tier();

        let egui_renderer =
            egui_wgpu::Renderer::new(renderer.device(), renderer.format(), None, 1, false);

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

        #[allow(clippy::collapsible_match)]
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    gpu.renderer.resize(&gpu.surface, size.width, size.height);
                }
            }
            _ => {}
        }

        // Early return if egui handled the event
        #[allow(clippy::needless_return)]
        if response.consumed {
            return;
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Collect status bar data before any mutable borrows
        let fps = self.fps;
        let entity_count = self.world.world().len();
        let tier = self.render_tier;
        let is_playing = self.is_playing;

        let gpu = self.gpu.as_mut().unwrap();
        let size = gpu.window.inner_size();
        if size.width == 0 || size.height == 0 {
            return;
        }

        // Apply tier change if needed
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

        let mut encoder =
            gpu.renderer
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("frame_encoder"),
                });

        // egui UI pass
        let raw_input = self.egui_winit.take_egui_input(&gpu.window);
        let ctx = self.egui_winit.egui_ctx();

        ctx.begin_pass(raw_input);

        // Top menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Project").clicked() {
                        self.show_welcome = true;
                        ui.close_menu();
                    }
                    if ui.button("Open Project...").clicked() {
                        // TODO: Open project dialog
                        ui.close_menu();
                    }
                    if ui.button("Save Project").clicked() {
                        // TODO: Save project
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Export...").clicked() {
                        // TODO: Export dialog
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        std::process::exit(0);
                    }
                });

                ui.menu_button("Edit", |ui| {
                    if ui.button("Undo").clicked() {
                        // TODO: Undo
                        ui.close_menu();
                    }
                    if ui.button("Redo").clicked() {
                        // TODO: Redo
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Preferences...").clicked() {
                        // Open Settings tab
                        self.dock_state.add_window(vec![Tab::Settings]);
                        ui.close_menu();
                    }
                });

                ui.menu_button("View", |ui| {
                    if ui.button("Scene View").clicked() {
                        self.dock_state.add_window(vec![Tab::SceneView]);
                        ui.close_menu();
                    }
                    if ui.button("Inspector").clicked() {
                        self.dock_state.add_window(vec![Tab::Inspector]);
                        ui.close_menu();
                    }
                    if ui.button("Assets").clicked() {
                        self.dock_state.add_window(vec![Tab::Assets]);
                        ui.close_menu();
                    }
                    if ui.button("Console").clicked() {
                        self.dock_state.add_window(vec![Tab::Console]);
                        ui.close_menu();
                    }
                    if ui.button("Graph Editor").clicked() {
                        self.dock_state.add_window(vec![Tab::GraphEditor]);
                        ui.close_menu();
                    }
                    if ui.button("UI Designer").clicked() {
                        self.dock_state.add_window(vec![Tab::UIDesigner]);
                        ui.close_menu();
                    }
                    if ui.button("Prefab Editor").clicked() {
                        self.dock_state.add_window(vec![Tab::PrefabEditor]);
                        ui.close_menu();
                    }
                    if ui.button("Scene Editor").clicked() {
                        self.dock_state.add_window(vec![Tab::SceneEditor]);
                        ui.close_menu();
                    }
                    if ui.button("Gameplay Editor").clicked() {
                        self.dock_state.add_window(vec![Tab::GameplayEditor]);
                        ui.close_menu();
                    }
                });

                ui.menu_button("Project", |ui| {
                    if ui.button("Build Settings").clicked() {
                        self.dock_state.add_window(vec![Tab::Settings]);
                        ui.close_menu();
                    }
                    if ui.button("Package").clicked() {
                        // TODO: Package
                        ui.close_menu();
                    }
                });

                ui.menu_button("Window", |ui| {
                    if ui.button("Reset Layout").clicked() {
                        self.dock_state = egui_dock::DockState::new(vec![Tab::SceneView]);
                        ui.close_menu();
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui.button("Documentation").clicked() {
                        // TODO: Open docs
                        ui.close_menu();
                    }
                    if ui.button("About Kiln").clicked() {
                        // TODO: About dialog
                        ui.close_menu();
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("v{}", env!("CARGO_PKG_VERSION")));
                });
            });
        });

        // Welcome screen overlay
        if self.show_welcome {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.centered_and_justified(|ui| {
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgba_premultiplied(0, 0, 0, 200))
                        .rounding(8.0)
                        .inner_margin(32.0)
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.add_space(16.0);
                                ui.heading("Welcome to Kiln");
                                ui.label("Lightweight game engine for indie developers");
                                ui.add_space(24.0);

                                ui.horizontal(|ui| {
                                    if ui.add_sized([200.0, 40.0], egui::Button::new("New Project")).clicked() {
                                        self.show_welcome = false;
                                    }
                                    ui.add_space(16.0);
                                    if ui.add_sized([200.0, 40.0], egui::Button::new("Open Project")).clicked() {
                                        self.show_welcome = false;
                                    }
                                });

                                ui.add_space(16.0);
                                ui.label("Recent Projects:");
                                ui.add_space(8.0);
                                ui.label("(No recent projects)");
                                ui.add_space(16.0);
                                if ui.button("Start with Demo Scene").clicked() {
                                    self.show_welcome = false;
                                }
                            });
                        });
                });
            });
        } else {
            // Main editor area
            egui::CentralPanel::default().show(ctx, |ui| {
                let mut dock_style = egui_dock::Style::from_egui(ui.style());
                dock_style.tab_bar.bg_fill = egui::Color32::from_rgb(30, 30, 35);
                dock_style.tab_bar.hline_color = egui::Color32::from_rgb(60, 60, 65);
                dock_style.separator.color_idle = egui::Color32::from_rgb(60, 60, 65);
                dock_style.separator.color_hovered = egui::Color32::from_rgb(80, 80, 90);
                dock_style.separator.color_dragged = egui::Color32::from_rgb(100, 100, 110);
                dock_style.separator.extra = 4.0;

                let tree = egui_dock::DockArea::new(&mut self.dock_state).style(dock_style);

                let mut tab_viewer = TabViewer {
                    world: &mut self.world,
                    selected_entity: &mut self.selected_entity,
                    is_playing: &mut self.is_playing,
                    graph: &mut self.graph,
                    render_tier: &mut self.render_tier,
                    ui_designer: &mut self.ui_designer,
                    prefab_editor: &mut self.prefab_editor,
                    gameplay_editor: &mut self.gameplay_editor,
                };
                tree.show(ctx, &mut tab_viewer);
            });

            // Status bar
            show_status_bar(ctx, fps, entity_count, tier, is_playing);
        }

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

        // Update FPS counter
        self.frame_count += 1;
        // TODO: Proper FPS timing
    }
}

// Standalone function for status bar (no self borrow issues)
fn show_status_bar(ctx: &egui::Context, fps: f32, entity_count: u32, tier: kiln_render::RenderTier, is_playing: bool) {
    egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(16.0, 0.0);

            // Left side: status info
            ui.label(format!("FPS: {:.0}", fps));
            ui.separator();
            ui.label(format!("Entities: {}", entity_count));
            ui.separator();
            ui.label(format!("Tier: {:?}", tier));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Right side: shortcuts and info
                ui.label("Ctrl+S: Save | Ctrl+Z: Undo | Ctrl+Y: Redo");
                ui.separator();
                if is_playing {
                    ui.label("🔴 Playing");
                } else {
                    ui.label("⏸ Paused");
                }
            });
        });
    });
}

struct TabViewer<'a> {
    world: &'a mut GameWorld,
    selected_entity: &'a mut Option<hecs::Entity>,
    is_playing: &'a mut bool,
    graph: &'a mut Graph,
    render_tier: &'a mut RenderTier,
    ui_designer: &'a mut UIDesignerState,
    prefab_editor: &'a mut PrefabEditorState,
    gameplay_editor: &'a mut GameplayEditorState,
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
                    let desired_size =
                        egui::vec2(ui.available_width(), ui.available_height() * 0.65);
                    let (rect, _response) =
                        ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());

                    // Draw background with grid pattern
                    ui.painter()
                        .rect_filled(rect, 0.0, egui::Color32::from_rgb(18, 18, 20));

                    // Draw grid lines
                    let grid_size = 50.0;
                    let painter = ui.painter();
                    for x in (rect.min.x as i32..rect.max.x as i32).step_by(grid_size as usize) {
                        painter.line_segment(
                            [
                                egui::pos2(x as f32, rect.min.y),
                                egui::pos2(x as f32, rect.max.y),
                            ],
                            (
                                1.0,
                                egui::Color32::from_rgba_premultiplied(255, 255, 255, 15),
                            ),
                        );
                    }
                    for y in (rect.min.y as i32..rect.max.y as i32).step_by(grid_size as usize) {
                        painter.line_segment(
                            [
                                egui::pos2(rect.min.x, y as f32),
                                egui::pos2(rect.max.x, y as f32),
                            ],
                            (
                                1.0,
                                egui::Color32::from_rgba_premultiplied(255, 255, 255, 15),
                            ),
                        );
                    }

                    // Draw sprites as rectangles
                    let sprites: Vec<(Transform, Sprite)> = self
                        .world
                        .world()
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
                        painter.rect_stroke(
                            sprite_rect,
                            2.0,
                            egui::Stroke::new(
                                1.0_f32,
                                egui::Color32::from_rgba_premultiplied(255, 255, 255, 50),
                            ),
                        );
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
                            let entities: Vec<(hecs::Entity, Transform)> = self
                                .world
                                .world()
                                .query::<(hecs::Entity, &Transform, &Sprite)>()
                                .iter()
                                .map(|(e, t, _s)| (e, *t))
                                .collect();
                            for (entity, transform) in entities {
                                let label = format!(
                                    "Entity ({:.0}, {:.0})",
                                    transform.position.x, transform.position.y
                                );
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
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui.button("⋮").clicked() {
                                                    // TODO: Component menu
                                                }
                                            },
                                        );
                                    });
                                    ui.add_space(8.0);

                                    ui.horizontal(|ui| {
                                        ui.label("Position");
                                        ui.add_space(16.0);
                                        ui.label("X:");
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut transform.position.x)
                                                    .speed(1.0),
                                            )
                                            .changed()
                                        {
                                            let world = self.world.world_mut();
                                            if let Ok(mut t) = world.get::<&mut Transform>(*entity)
                                            {
                                                t.position.x = transform.position.x;
                                            }
                                        }
                                        ui.add_space(8.0);
                                        ui.label("Y:");
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut transform.position.y)
                                                    .speed(1.0),
                                            )
                                            .changed()
                                        {
                                            let world = self.world.world_mut();
                                            if let Ok(mut t) = world.get::<&mut Transform>(*entity)
                                            {
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
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui.button("⋮").clicked() {
                                                    // TODO: Component menu
                                                }
                                            },
                                        );
                                    });
                                    ui.add_space(8.0);

                                    ui.horizontal(|ui| {
                                        ui.label("Color");
                                        ui.add_space(16.0);
                                        ui.label("R:");
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut sprite.color[0])
                                                    .speed(0.01)
                                                    .range(0.0..=1.0),
                                            )
                                            .changed()
                                        {
                                            let world = self.world.world_mut();
                                            if let Ok(mut s) = world.get::<&mut Sprite>(*entity) {
                                                s.color[0] = sprite.color[0];
                                            }
                                        }
                                        ui.add_space(8.0);
                                        ui.label("G:");
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut sprite.color[1])
                                                    .speed(0.01)
                                                    .range(0.0..=1.0),
                                            )
                                            .changed()
                                        {
                                            let world = self.world.world_mut();
                                            if let Ok(mut s) = world.get::<&mut Sprite>(*entity) {
                                                s.color[1] = sprite.color[1];
                                            }
                                        }
                                        ui.add_space(8.0);
                                        ui.label("B:");
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut sprite.color[2])
                                                    .speed(0.01)
                                                    .range(0.0..=1.0),
                                            )
                                            .changed()
                                        {
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
                                    ui.painter().rect_stroke(
                                        color_rect,
                                        2.0,
                                        egui::Stroke::new(
                                            1.0_f32,
                                            egui::Color32::from_rgb(100, 100, 100),
                                        ),
                                    );
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
                            ui.label(
                                "Select an entity from the Scene View to inspect its components",
                            );
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

                        // AI node buttons
                        if ui.button("Selector").clicked() {
                            let node = Node::new(NodeType::Selector, (400.0, 100.0));
                            self.graph.add_node(node);
                        }
                        if ui.button("Sequence").clicked() {
                            let node = Node::new(NodeType::Sequence, (500.0, 100.0));
                            self.graph.add_node(node);
                        }
                        if ui.button("Behavior").clicked() {
                            let node = Node::new(
                                NodeType::BehaviorAction {
                                    name: "idle".to_string(),
                                },
                                (600.0, 100.0),
                            );
                            self.graph.add_node(node);
                        }
                        if ui.button("AI State").clicked() {
                            let node = Node::new(
                                NodeType::AIState {
                                    state_name: "patrol".to_string(),
                                },
                                (700.0, 100.0),
                            );
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
                    let (rect, _response) =
                        ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());

                    // Draw background
                    ui.painter()
                        .rect_filled(rect, 0.0, egui::Color32::from_rgb(18, 18, 20));

                    // Draw grid
                    let grid_size = 50.0;
                    let painter = ui.painter();
                    for x in (rect.min.x as i32..rect.max.x as i32).step_by(grid_size as usize) {
                        painter.line_segment(
                            [
                                egui::pos2(x as f32, rect.min.y),
                                egui::pos2(x as f32, rect.max.y),
                            ],
                            (
                                1.0,
                                egui::Color32::from_rgba_premultiplied(255, 255, 255, 15),
                            ),
                        );
                    }
                    for y in (rect.min.y as i32..rect.max.y as i32).step_by(grid_size as usize) {
                        painter.line_segment(
                            [
                                egui::pos2(rect.min.x, y as f32),
                                egui::pos2(rect.max.x, y as f32),
                            ],
                            (
                                1.0,
                                egui::Color32::from_rgba_premultiplied(255, 255, 255, 15),
                            ),
                        );
                    }

                    // Draw nodes
                    for node in self.graph.nodes.values() {
                        let pos = egui::pos2(node.position.0, node.position.1);
                        let size = egui::vec2(140.0, 70.0);
                        let rect = egui::Rect::from_min_size(pos, size);

                        // Draw node background
                        painter.rect_filled(rect, 6.0, egui::Color32::from_rgb(45, 45, 50));
                        painter.rect_stroke(
                            rect,
                            6.0,
                            egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(90, 90, 100)),
                        );

                        // Draw node header
                        let header_rect =
                            egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), 24.0));
                        painter.rect_filled(
                            header_rect,
                            6.0,
                            egui::Color32::from_rgb(66, 135, 245),
                        );

                        // Draw node title
                        let title = match &node.node_type {
                            NodeType::Constant { .. } => "Constant",
                            NodeType::Arithmetic { .. } => "Add",
                            NodeType::Output => "Output",
                            NodeType::Selector => "Selector",
                            NodeType::Sequence => "Sequence",
                            NodeType::BehaviorAction { .. } => "Behavior",
                            NodeType::BehaviorCondition => "Condition",
                            NodeType::AIState { .. } => "AI State",
                            NodeType::AITransition => "AI Transition",
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
                            ui.label(format!(
                                "  Bloom passes: {}",
                                self.render_tier.max_bloom_passes()
                            ));
                            ui.label(format!(
                                "  LUT color grading: {}",
                                self.render_tier.enable_lut()
                            ));
                            ui.label(format!(
                                "  Screen-space fog: {}",
                                self.render_tier.enable_fog()
                            ));
                            ui.label(format!(
                                "  Outline effect: {}",
                                self.render_tier.enable_outline()
                            ));
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
                                        .args([
                                            "run",
                                            "--bin",
                                            "kiln-cli",
                                            "--",
                                            "export",
                                            "--output",
                                            "web-export",
                                        ])
                                        .status();
                                });
                            }

                            ui.add_space(4.0);
                            ui.label("Click to export. Check terminal for progress.");
                        });
                });
            }
            Tab::UIDesigner => {
                ui.vertical(|ui| {
                    // Toolbar
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);

                        ui.label("Add Widget:");
                        if ui.button("Panel").clicked() {
                            let panel = Panel::new(WidgetId::new());
                            self.ui_designer.add_widget(UiWidget::Panel(panel), None);
                        }
                        if ui.button("Button").clicked() {
                            let button = Button::new(WidgetId::new(), "button.label");
                            self.ui_designer.add_widget(UiWidget::Button(button), None);
                        }
                        if ui.button("Label").clicked() {
                            let label = Label::new(WidgetId::new(), "label.text");
                            self.ui_designer.add_widget(UiWidget::Label(label), None);
                        }
                        if ui.button("List").clicked() {
                            let list = List::new(WidgetId::new());
                            self.ui_designer.add_widget(UiWidget::List(list), None);
                        }

                        ui.separator();

                        if ui.button("Clear All").clicked() {
                            *self.ui_designer = UIDesignerState::new();
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label("UI Designer");
                        });
                    });

                    ui.add_space(8.0);

                    // Split view: hierarchy tree on left, properties on right
                    ui.horizontal(|ui| {
                        // Left: Hierarchy tree
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.strong("Hierarchy");
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("Refresh").clicked() {
                                        // Refresh hierarchy
                                    }
                                });
                            });

                            ui.add_space(4.0);

                            egui::ScrollArea::vertical()
                                .max_height(ui.available_height())
                                .show(ui, |ui| {
                                    let root_ids: Vec<WidgetId> = self.ui_designer.root_nodes.clone();
                                    for root_id in root_ids {
                                        self.show_widget_tree(ui, root_id, 0);
                                    }
                                });
                        });

                        ui.separator();

                        // Right: Visual canvas
                        ui.vertical(|ui| {
                            ui.strong("Canvas");
                            ui.add_space(4.0);

                            let desired_size = egui::vec2(ui.available_width(), ui.available_height());
                            let (rect, _response) =
                                ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());

                            // Draw canvas background
                            ui.painter()
                                .rect_filled(rect, 0.0, egui::Color32::from_rgb(18, 18, 20));

                            // Draw grid
                            let grid_size = 50.0;
                            let painter = ui.painter();
                            for x in (rect.min.x as i32..rect.max.x as i32).step_by(grid_size as usize) {
                                painter.line_segment(
                                    [
                                        egui::pos2(x as f32, rect.min.y),
                                        egui::pos2(x as f32, rect.max.y),
                                    ],
                                    (
                                        1.0,
                                        egui::Color32::from_rgba_premultiplied(255, 255, 255, 15),
                                    ),
                                );
                            }
                            for y in (rect.min.y as i32..rect.max.y as i32).step_by(grid_size as usize) {
                                painter.line_segment(
                                    [
                                        egui::pos2(rect.min.x, y as f32),
                                        egui::pos2(rect.max.x, y as f32),
                                    ],
                                    (
                                        1.0,
                                        egui::Color32::from_rgba_premultiplied(255, 255, 255, 15),
                                    ),
                                );
                            }

                            // Draw widgets
                            for root_id in &self.ui_designer.root_nodes {
                                self.draw_widget_canvas(ui, rect, *root_id);
                            }
                        });
                    });

                    ui.add_space(8.0);

                    // Bottom: Property inspector for selected widget
                    if let Some(selected_id) = self.ui_designer.selected_widget {
                        ui.separator();
                        ui.add_space(4.0);
                        ui.strong("Properties");
                        ui.add_space(4.0);

                        self.show_widget_properties(ui, selected_id);
                    }
                });
            }
            Tab::PrefabEditor => {
                ui.vertical(|ui| {
                    ui.heading("Prefab Editor");
                    ui.add_space(8.0);

                    // Toolbar
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);

                        if ui.button("Create Prefab from Selection").clicked() {
                            // TODO: Create prefab from selected entity
                        }

                        ui.separator();

                        if ui.button("Instantiate Prefab").clicked() {
                            // TODO: Instantiate selected prefab
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!("Prefabs: {}", self.prefab_editor.prefab_list().len()));
                        });
                    });

                    ui.add_space(8.0);

                    // Prefab list
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(35, 35, 40))
                        .rounding(4.0)
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.strong("Prefabs");
                            ui.add_space(8.0);

                            egui::ScrollArea::vertical()
                                .max_height(ui.available_height())
                                .show(ui, |ui| {
                                    let prefab_ids: Vec<(String, String)> = self.prefab_editor
                                        .prefab_list()
                                        .iter()
                                        .map(|p| (p.id.clone(), p.name.clone()))
                                        .collect();

                                    let is_empty = prefab_ids.is_empty();

                                    for (prefab_id, prefab_name) in prefab_ids {
                                        ui.horizontal(|ui| {
                                            ui.label(&prefab_name);
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if ui.button("Edit").clicked() {
                                                    // TODO: Edit prefab
                                                }
                                                if ui.button("Delete").clicked() {
                                                    self.prefab_editor.remove_prefab(&prefab_id);
                                                }
                                            });
                                        });
                                        ui.add_space(4.0);
                                    }

                                    if is_empty {
                                        ui.label("No prefabs created yet");
                                        ui.label("Select an entity and click 'Create Prefab from Selection'");
                                    }
                                });
                        });
                });
            }
            Tab::SceneEditor => {
                ui.vertical(|ui| {
                    ui.heading("Scene Editor");
                    ui.add_space(8.0);

                    // Toolbar
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);

                        if ui.button("Create New Scene").clicked() {
                            // TODO: Create new scene
                        }

                        ui.separator();

                        if ui.button("Add Nested Scene").clicked() {
                            // TODO: Add nested scene
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label("Scene Composition");
                        });
                    });

                    ui.add_space(8.0);

                    // Scene composition info
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(35, 35, 40))
                        .rounding(4.0)
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.strong("Nested Scenes");
                            ui.add_space(8.0);

                            ui.label("Scenes can be nested with optional additive loading.");
                            ui.label("Each nested scene can have an offset for positioning.");
                            ui.add_space(8.0);

                            ui.label("Features:");
                            ui.label("  • Load multiple scenes into one view");
                            ui.label("  • Additive loading (merge entities)");
                            ui.label("  • Scene offset for world positioning");
                            ui.label("  • Scene hierarchy support");
                            ui.add_space(8.0);

                            ui.label("Current scene has no nested scenes.");
                            ui.label("Use 'Add Nested Scene' to compose scenes.");
                        });
                });
            }
            Tab::GameplayEditor => {
                ui.vertical(|ui| {
                    ui.heading("Gameplay Editor");
                    ui.add_space(8.0);

                    // Toolbar
                    ui.horizontal(|ui| {
                        if ui.button("New Dialogue").clicked() {
                            let dialogue = Dialogue::new("new_dialogue".to_string(), "start".to_string());
                            self.gameplay_editor.add_dialogue(dialogue);
                        }
                        if ui.button("New Quest").clicked() {
                            let quest = Quest::new("new_quest".to_string(), "quest_name".to_string(), "quest_desc".to_string());
                            self.gameplay_editor.add_quest(quest);
                        }
                        if ui.button("New Data Table").clicked() {
                            let table = DataTable::new("new_table".to_string());
                            self.gameplay_editor.add_data_table(table);
                        }
                    });

                    ui.add_space(8.0);

                    // Split view: list panel and detail panel
                    ui.horizontal(|ui| {
                        // Left panel: list of all gameplay content
                        ui.vertical(|ui| {
                            ui.label("Dialogues");
                            for (id, _) in &self.gameplay_editor.dialogues {
                                let is_selected = self.gameplay_editor.selected_dialogue.as_ref() == Some(id);
                                if ui.selectable_label(is_selected, id).clicked() {
                                    self.gameplay_editor.selected_dialogue = Some(id.clone());
                                }
                            }

                            ui.add_space(8.0);
                            ui.label("Quests");
                            for (id, _) in &self.gameplay_editor.quests {
                                let is_selected = self.gameplay_editor.selected_quest.as_ref() == Some(id);
                                if ui.selectable_label(is_selected, id).clicked() {
                                    self.gameplay_editor.selected_quest = Some(id.clone());
                                }
                            }

                            ui.add_space(8.0);
                            ui.label("Data Tables");
                            for (id, _) in &self.gameplay_editor.data_tables {
                                let is_selected = self.gameplay_editor.selected_table.as_ref() == Some(id);
                                if ui.selectable_label(is_selected, id).clicked() {
                                    self.gameplay_editor.selected_table = Some(id.clone());
                                }
                            }
                        });

                        ui.add_space(8.0);

                        // Right panel: detail view
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            if let Some(id) = &self.gameplay_editor.selected_dialogue {
                                if let Some(dialogue) = self.gameplay_editor.dialogues.get(id) {
                                    ui.heading(format!("Dialogue: {}", dialogue.id));
                                    ui.label(format!("Start node: {}", dialogue.start_node));
                                    ui.label(format!("Nodes: {}", dialogue.nodes.len()));
                                    ui.add_space(8.0);
                                    ui.label("Dialogue nodes are edited in the Graph Editor");
                                    ui.label("with Dialogue, DialogueChoice, and variable nodes.");
                                }
                            } else if let Some(id) = &self.gameplay_editor.selected_quest {
                                if let Some(quest) = self.gameplay_editor.quests.get(id) {
                                    ui.heading(format!("Quest: {}", quest.id));
                                    ui.label(format!("Name key: {}", quest.name_key));
                                    ui.label(format!("Objectives: {}", quest.objectives.len()));
                                    ui.label(format!("Prerequisites: {:?}", quest.prerequisites));
                                    ui.add_space(8.0);
                                    ui.label("Quest objectives are managed via graph nodes");
                                    ui.label("(QuestStart, QuestComplete, QuestObjectiveUpdate).");
                                }
                            } else if let Some(id) = &self.gameplay_editor.selected_table {
                                if let Some(table) = self.gameplay_editor.data_tables.get(id) {
                                    ui.heading(format!("Data Table: {}", table.id));
                                    ui.label(format!("Columns: {}", table.columns.len()));
                                    ui.label(format!("Rows: {}", table.rows.len()));
                                    ui.add_space(8.0);
                                    ui.label("Import CSV format:");
                                    ui.label("Row 1: Column names");
                                    ui.label("Row 2: Data types (Integer, Float, String, Boolean)");
                                    ui.label("Row 3+: Data values");
                                }
                            } else {
                                ui.label("Select a dialogue, quest, or data table to view details.");
                            }
                        });
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
            Tab::UIDesigner => "UI Designer".into(),
            Tab::PrefabEditor => "Prefab Editor".into(),
            Tab::SceneEditor => "Scene Editor".into(),
            Tab::GameplayEditor => "Gameplay Editor".into(),
        }
    }
}

impl<'a> TabViewer<'a> {
    fn show_widget_tree(&mut self, ui: &mut egui::Ui, widget_id: WidgetId, depth: usize) {
        if let Some(node) = self.ui_designer.widgets.get(&widget_id) {
            let indent = depth as f32 * 16.0;
            let is_selected = self.ui_designer.selected_widget == Some(widget_id);

            ui.horizontal(|ui| {
                ui.add_space(indent);

                let label = format!("{} ({})", node.widget.name(), widget_id.as_u64());
                if ui.selectable_label(is_selected, label).clicked() {
                    self.ui_designer.selected_widget = Some(widget_id);
                }
            });

            let children: Vec<WidgetId> = node.children.clone();
            for child_id in children {
                self.show_widget_tree(ui, child_id, depth + 1);
            }
        }
    }

    fn draw_widget_canvas(&self, ui: &mut egui::Ui, canvas_rect: egui::Rect, widget_id: WidgetId) {
        if let Some(node) = self.ui_designer.widgets.get(&widget_id) {
            let widget = &node.widget;
            let bounds = widget.as_widget().bounds();

            // Offset widget to canvas
            let screen_pos = egui::pos2(
                canvas_rect.min.x + bounds.x,
                canvas_rect.min.y + bounds.y,
            );
            let size = egui::vec2(bounds.width, bounds.height);
            let rect = egui::Rect::from_min_size(screen_pos, size);

            // Draw widget based on type
            let color = match widget {
                UiWidget::Panel(_) => egui::Color32::from_rgb(60, 60, 70),
                UiWidget::Button(_) => egui::Color32::from_rgb(66, 135, 245),
                UiWidget::Label(_) => egui::Color32::from_rgb(100, 100, 110),
                UiWidget::List(_) => egui::Color32::from_rgb(70, 70, 80),
            };

            let is_selected = self.ui_designer.selected_widget == Some(widget_id);
            let stroke = if is_selected {
                egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(255, 200, 50))
            } else {
                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(100, 100, 110))
            };

            ui.painter().rect_filled(rect, 4.0, color);
            ui.painter().rect_stroke(rect, 4.0, stroke);

            // Draw widget label
            let label = widget.name();
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(12.0),
                egui::Color32::WHITE,
            );

            // Draw children
            for child_id in &node.children {
                self.draw_widget_canvas(ui, canvas_rect, *child_id);
            }
        }
    }

    fn show_widget_properties(&mut self, ui: &mut egui::Ui, widget_id: WidgetId) {
        let widget_clone = self.ui_designer.get_widget(widget_id).cloned();
        let widget_name = widget_clone.as_ref().map(|w| w.name().to_string());
        let widget_visible = widget_clone.as_ref().map(|w| w.as_widget().visible());
        let widget_bounds = widget_clone.as_ref().map(|w| w.as_widget().bounds());

        egui::Frame::none()
            .fill(egui::Color32::from_rgb(35, 35, 40))
            .rounding(4.0)
            .inner_margin(12.0)
            .show(ui, |ui| {
                if let Some(name) = widget_name {
                    ui.label(format!("Type: {}", name));
                }
                ui.label(format!("ID: {}", widget_id.as_u64()));

                ui.add_space(8.0);

                // Visibility
                if let Some(visible) = widget_visible {
                    let mut visible_mut = visible;
                    if ui.checkbox(&mut visible_mut, "Visible").changed() {
                        if let Some(w) = self.ui_designer.get_widget_mut(widget_id) {
                            w.as_widget_mut().set_visible(visible_mut);
                        }
                    }
                }

                ui.add_space(8.0);

                // Layout properties
                ui.strong("Layout");
                ui.add_space(4.0);

                if let Some(bounds) = widget_bounds {
                    ui.horizontal(|ui| {
                        ui.label("Position:");
                        ui.label(format!("x: {:.0}", bounds.x));
                        ui.label(format!("y: {:.0}", bounds.y));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Size:");
                        ui.label(format!("w: {:.0}", bounds.width));
                        ui.label(format!("h: {:.0}", bounds.height));
                    });
                }

                ui.add_space(8.0);

                // Delete button
                if ui.button("Delete Widget").clicked() {
                    self.ui_designer.remove_widget(widget_id);
                }
            });
    }
}
