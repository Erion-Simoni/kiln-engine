//! Main editor application.

use hecs;
use kiln_core::project::{Component, Entity, Scene};
use kiln_core::{project::{Project, Scene}, Transform};
use kiln_gameplay::{DataTable, Dialogue, Quest};
use kiln_graph::{ArithmeticOp, ConstantValue, Graph, Node, NodeType};
use kiln_render::{
    clear_view, execute_passes, Frame2dGraph, PassTiming, PostConfig, PostFramebuffer,
    PostPipeline, RenderTier, SpriteInstance, SpriteRenderer,
};
use kiln_runtime::{GameWorld, SourceId, Sprite, Tilemap};
use kiln_ui::{Button, Label, List, Panel, Widget, WidgetId};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use thiserror::Error;
use winit::{
    event_loop::{ActiveEventLoop, EventLoop},
    window::Window,
};

use crate::anim_editor::AnimationEditorState;
use crate::assets::{AssetBrowser, AssetKind};
use crate::build_panel::BuildPanel;
use crate::history::SceneHistory;
use crate::hierarchy::HierarchyPanel;
use crate::inspector::InspectorPanel;
use crate::prefab_editor_panel::PrefabEditorPanel;
use crate::plugin_manager_panel::PluginManagerPanel;
use crate::preferences::PreferencesPanel;
use crate::version_control_panel::VersionControlPanel;
use crate::ui_layout_editor::UiLayoutEditorState;
use crate::physics_editor::PhysicsEditorState;
use crate::audio_editor::AudioEditorState;
use crate::vfx_editor::VfxEditorState;
use crate::debugger_panel::DebuggerPanel;
use crate::profiler_panel::ProfilerPanel;
use crate::project_hub::ProjectHubState;
use crate::scene_view::{self, SceneTextures, ViewportRect};
use crate::tilemap_editor::TilemapEditorState;
use crate::graph_editor::GraphEditorState;

/// A running game session in the editor.
///
/// The editor runs the same script-driven game loop as the standalone
/// player, so a project behaves the same in both.
struct PlaySession {
    /// The gameplay script of the played scene.
    slice: kiln_runtime::SliceSession,
    /// The scene being played.
    scene_id: String,
    /// The last error of the script, shown in the status bar.
    error: Option<String>,
}

/// The world size the scene view falls back to when no project is open.
const DEFAULT_WORLD_SIZE: (f32, f32) = (1280.0, 720.0);

/// The spacing of the scene view's grid, in world units.
const GRID_SPACING: f32 = 50.0;

/// The colour of the scene view's grid lines.
const GRID_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 0.06];

/// The colour behind the scene view.
const BACKGROUND_COLOR: [f32; 4] = [0.07, 0.07, 0.08, 1.0];

/// The components an entity can have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ComponentKind {
    Transform,
    Sprite,
    Role,
    Tilemap,
}

/// The default size of a sprite without a texture, in world units.
const DEFAULT_SPRITE_SIZE: [f32; 2] = [32.0, 32.0];

#[derive(Error, Debug)]
pub enum EditorError {
    #[error("Window error: {0}")]
    Window(String),
    #[error("GPU error: {0}")]
    Gpu(String),
}

#[derive(Debug, Clone)]
enum MenuAction {
    NewProject,
    OpenProject,
    SaveProject,
    SaveProjectAs,
    SaveScene,
    LoadScene,
    Undo,
    Redo,
    ExportProject,
    PackageProject,
    OpenDocs,
    About,
}

pub struct EditorApp {
    gpu: Option<GpuState>,
    egui_winit: egui_winit::State,
    dock_state: egui_dock::DockState<Tab>,
    world: GameWorld,
    selected_entity: Option<hecs::Entity>,
    is_playing: bool,
    /// The running game session, present while playing.
    play: Option<PlaySession>,
    /// Lines shown in the console panel.
    console_log: Vec<String>,
    /// Directory of the open project, where the assets live.
    project_root: Option<PathBuf>,
    /// The path typed in the open-project prompt.
    project_path_input: String,
    /// Asset path of each scene entity's texture, keyed by entity id.
    entity_textures: HashMap<String, String>,
    /// Gameplay role of each scene entity, keyed by entity id.
    entity_roles: HashMap<String, String>,
    /// Screen-space rectangle of the scene view panel from the last frame.
    scene_view_rect: Option<egui::Rect>,
    graph: Graph,
    render_tier: RenderTier,
    show_welcome: bool,
    /// Whether the about window is open.
    show_about: bool,
    /// The measured frame rate, in frames per second.
    fps: f32,
    /// The time the previous frame was rendered, used to measure the frame
    /// rate.
    last_frame: Instant,
    /// The number of frames rendered so far.
    frame_count: u32,
    /// The history of scene edits, used by undo and redo.
    history: SceneHistory,
    ui_designer: UIDesignerState,
    gameplay_editor: GameplayEditorState,
    tilemap_editor: TilemapEditorState,
    hierarchy_panel: HierarchyPanel,
    anim_editor: AnimationEditorState,
    build_panel: BuildPanel,
    project_hub: ProjectHubState,
    profiler_panel: ProfilerPanel,
    debugger_panel: DebuggerPanel,
    vfx_editor: VfxEditorState,
    audio_editor: AudioEditorState,
    physics_editor: PhysicsEditorState,
    ui_layout_editor: UiLayoutEditorState,
    preferences: PreferencesPanel,
    prefab_editor: PrefabEditorPanel,
    version_control: VersionControlPanel,
    plugin_manager: PluginManagerPanel,
    /// Mapping from project entity ID (source_id) to hecs Entity for selection sync.
    entity_id_to_hecs: HashMap<String, hecs::Entity>,
    graph_editor: GraphEditorState,
    inspector_panel: InspectorPanel,
    project_path: Option<PathBuf>,
    current_project: Option<Project>,
    asset_browser: AssetBrowser,
    /// Set by the welcome screen, handled after the UI pass.
    open_project_requested: bool,
    /// Set by the scene toolbar, handled after the UI pass.
    play_toggle_requested: bool,
    /// Set by the scene toolbar, handled after the UI pass.
    scene_save_requested: bool,
    /// Set by the scene toolbar, handled after the UI pass.
    scene_load_requested: bool,
    /// Scene tabs for the scene tabs bar.
    scene_tabs: Vec<SceneTab>,
    /// Currently active scene tab index.
    active_scene_tab: Option<usize>,
}

/// A scene tab entry for the scene tabs bar.
#[derive(Debug, Clone)]
struct SceneTab {
    /// The scene ID.
    scene_id: String,
    /// Whether this tab is the active one.
    is_active: bool,
    /// Whether the scene has unsaved changes.
    dirty: bool,
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
    Hierarchy,
    UIDesigner,
    PrefabEditor,
    SceneEditor,
    GameplayEditor,
    TilemapEditor,
    AnimationEditor,
    BuildPanel,
    ProjectHub,
    ProfilerPanel,
    DebuggerPanel,
    VfxEditor,
    AudioEditor,
    PhysicsEditor,
    UiLayoutEditor,
    Preferences,
    PluginManagerPanel,
    PrefabEditorPanel,
    VersionControl,
}
}

/// A scene tab entry for the scene tabs bar.
#[derive(Debug, Clone)]
struct SceneTab {
    /// The scene ID.
    scene_id: String,
    /// Whether this tab is the active one.
    is_active: bool,
    /// Whether the scene has unsaved changes.
    dirty: bool,
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
    Hierarchy,
    UIDesigner,
    PrefabEditor,
    SceneEditor,
    GameplayEditor,
    TilemapEditor,
    AnimationEditor,
    BuildPanel,
    ProjectHub,
    ProfilerPanel,
    DebuggerPanel,
    VfxEditor,
    AudioEditor,
    PhysicsEditor,
    UiLayoutEditor,
    Preferences,
}

struct GpuState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    renderer: kiln_render::Renderer,
    egui_renderer: egui_wgpu::Renderer,
    /// Sprite batch renderer for the scene view. Draws scene entities using the
    /// same GPU pipeline as the standalone player.
    scene_view_sprite_renderer: SpriteRenderer,
    /// Sprite batch renderer for the screen-space overlay drawn over the scene
    /// view (the HUD).
    scene_view_overlay_renderer: SpriteRenderer,
    /// Textures of the open project, uploaded to the scene view renderer.
    scene_textures: SceneTextures,
    /// Post-processing framebuffer for the scene view (separate from main renderer).
    scene_view_post_framebuffer: Option<PostFramebuffer>,
    /// Post-processing pipeline for the scene view.
    scene_view_post_pipeline: Option<PostPipeline>,
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

/// Start a game session for a project.
///
/// The scene's script is read from the project directory, compiled and
/// paired with a world holding the scene. The player and the editor run the
/// same session, so a project behaves the same in both.
fn start_play_session(
    project: &Project,
    root: &Path,
    textures: &SceneTextures,
) -> Result<(kiln_runtime::SliceSession, String, GameWorld), String> {
    let scene_id = project
        .default_scene_id()
        .ok_or("The project has no scenes")?;
    let scene = project
        .scenes
        .get(&scene_id)
        .ok_or_else(|| format!("Scene '{scene_id}' not found"))?;

    let path = scene.script.as_ref().ok_or("The scene has no script")?;
    let source = std::fs::read_to_string(root.join(path))
        .map_err(|e| format!("could not read the scene script: {e}"))?;

    let slice = kiln_runtime::SliceSession::new(&source).map_err(|d| {
        d.iter()
            .map(|d| d.message.clone())
            .collect::<Vec<_>>()
            .join("; ")
    })?;

    let (world, _spawned) = spawn_editor_scene(project, &scene_id, textures);
    Ok((slice, scene_id, world))
}

/// Build the scene data of a world.
///
/// The source id keeps the entity's identity, so the roles and textures of
/// the scene survive a save.
pub fn scene_from_world(
    world: &GameWorld,
    entity_textures: &HashMap<String, String>,
    entity_roles: &HashMap<String, String>,
    project: Option<&Project>,
) -> Scene {
    let scene_id = project
        .and_then(|p| p.default_scene_id())
        .unwrap_or_else(|| "main_scene".to_string());
    let script = project
        .and_then(|p| p.scenes.get(&scene_id))
        .and_then(|s| s.script.clone());

    let mut entities = Vec::new();
    let mut entity_id = 0;
    let world = world.world_ref();

    // Sprite entities: SourceId + Transform + Sprite.
    let mut query = world.query::<(&SourceId, &Transform, &Sprite)>();
    for (source_id, transform, sprite) in query.iter() {
        entity_id += 1;
        let id = source_id.0.clone();
        let texture = entity_textures.get(&id).cloned();
        let role = entity_roles.get(&id).cloned();
        let mut components = vec![
            kiln_core::project::Component::Transform {
                x: transform.position.x,
                y: transform.position.y,
                rotation: transform.rotation,
                scale: transform.scale.x,
            },
            kiln_core::project::Component::Sprite {
                color: sprite.color,
                texture,
            },
        ];
        if let Some(role) = role {
            components.push(kiln_core::project::Component::Role { role });
        }
        entities.push(Entity {
            id,
            name: format!("Entity_{}", entity_id),
            components,
        });
    }

    // Tilemap entities: SourceId + Transform + Tilemap.
    let mut query = world.query::<(&SourceId, &Transform, &Tilemap)>();
    for (source_id, transform, tilemap) in query.iter() {
        entity_id += 1;
        let id = source_id.0.clone();
        let role = entity_roles.get(&id).cloned();
        let mut components = vec![
            kiln_core::project::Component::Transform {
                x: transform.position.x,
                y: transform.position.y,
                rotation: transform.rotation,
                scale: transform.scale.x,
            },
            kiln_core::project::Component::Tilemap {
                map: tilemap.map_id.clone(),
            },
        ];
        if let Some(role) = role {
            components.push(kiln_core::project::Component::Role { role });
        }
        entities.push(Entity {
            id,
            name: format!("Entity_{}", entity_id),
            components,
        });
    }

    // The nested scenes of the scene stay as they are: the world holds the
    // entities of the nested scenes too, so they are not written again here.
    let nested_scenes = project
        .and_then(|p| p.scenes.get(&scene_id))
        .map(|s| s.nested_scenes.clone())
        .unwrap_or_default();

    Scene {
        id: scene_id,
        name: "Main Scene".to_string(),
        entities,
        nested_scenes,
        script,
    }
}

/// Spawn a project scene into a fresh world for the editor.
///
/// The sprites are bound to the textures that were uploaded for the project,
/// so the editor's scene view shows the same art as the player. Tilemaps are
/// spawned with the editor's texture resolver.
fn spawn_editor_scene(
    project: &Project,
    scene_id: &str,
    textures: &SceneTextures,
) -> (GameWorld, Vec<kiln_runtime::SpawnedEntity>) {
    let mut world = GameWorld::new();
    let mut make_sprite =
        |_entity: &kiln_core::project::Entity, color: [f32; 4], texture: Option<&str>| {
            // A sprite is as big as its texture, like in the player.
            match texture.and_then(|path| textures.get(path)) {
                Some(tex) => Sprite::new(color[0], color[1], color[2], color[3])
                    .with_texture(tex.id)
                    .with_size(tex.width as f32, tex.height as f32),
                None => Sprite::new(color[0], color[1], color[2], color[3])
                    .with_size(DEFAULT_SPRITE_SIZE[0], DEFAULT_SPRITE_SIZE[1]),
            }
        };
    let scene = project
        .scenes
        .get(scene_id)
        .ok_or_else(|| format!("Scene '{scene_id}' not found"))
        .ok();
    let spawned = kiln_runtime::spawn_scene(&mut world, project, scene_id, &mut make_sprite);
    if let Some(scene) = scene {
        let mut resolve = |tileset_id: &str| textures.get(tileset_id).map(|tex| tex.id);
        kiln_runtime::spawn_tilemaps(
            &mut world,
            project,
            scene,
            kiln_core::SceneOffset::default(),
            &mut resolve,
        );
    }
    (world, spawned)
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
            play: None,
            console_log: Vec::new(),
            project_root: None,
            project_path_input: "kiln.toml".to_string(),
            entity_textures: HashMap::new(),
            entity_roles: HashMap::new(),
            scene_view_rect: None,
            graph: Graph::new(),
            render_tier: RenderTier::default(),
            show_welcome: true,
            show_about: false,
            fps: 60.0,
            last_frame: Instant::now(),
            frame_count: 0,
            history: SceneHistory::new(),
            ui_designer: UIDesignerState::new(),
            gameplay_editor: GameplayEditorState::new(),
            tilemap_editor: TilemapEditorState::new(),
            hierarchy_panel: HierarchyPanel::new(),
            anim_editor: AnimationEditorState::new(),
            build_panel: BuildPanel::new(),
            project_hub: ProjectHubState::new(),
            profiler_panel: ProfilerPanel::new(),
            debugger_panel: DebuggerPanel::new(),
            vfx_editor: VfxEditorState::new(),
            audio_editor: AudioEditorState::new(),
            physics_editor: PhysicsEditorState::new(),
            ui_layout_editor: UiLayoutEditorState::new(),
            preferences: PreferencesPanel::new(),
            prefab_editor: PrefabEditorPanel::new(),
            version_control: VersionControlPanel::new(),
            plugin_manager: PluginManagerPanel::new(),
            entity_id_to_hecs: HashMap::new(),
            graph_editor: GraphEditorState::new(),
            inspector_panel: InspectorPanel::new(),
            project_path: None,
            current_project: None,
            asset_browser: AssetBrowser::new(),
            open_project_requested: false,
            play_toggle_requested: false,
            scene_save_requested: false,
            scene_load_requested: false,
            scene_tabs: Vec::new(),
            active_scene_tab: None,
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

        // Create sprite and egui renderers while borrowing `renderer`,
        // then move `renderer` into GpuState once borrows end.
        let (scene_view_sprite_renderer, scene_view_overlay_renderer, egui_renderer) = {
            let device = renderer.device();
            let queue = renderer.queue();
            let format = renderer.format();
            let sr = SpriteRenderer::new(device, queue, format);
            let overlay = SpriteRenderer::new(device, queue, format);
            let er = egui_wgpu::Renderer::new(device, format, None, 1, false);
            (sr, overlay, er)
        };
        scene_view_sprite_renderer.upload_quad(renderer.queue());
        scene_view_overlay_renderer.upload_quad(renderer.queue());

        // Create post-processing framebuffer and pipeline for the scene view.
        // Only create them if the tier supports post-processing (not Low).
        let (scene_view_post_framebuffer, scene_view_post_pipeline) = if renderer.tier() != RenderTier::Low {
            let config = PostConfig::for_tier(renderer.tier());
            let fb = PostFramebuffer::new(renderer.device(), size.width, size.height, renderer.tier());
            let pipe = PostPipeline::new(renderer.device(), renderer.tier(), renderer.format());
            (Some(fb), Some(pipe))
        } else {
            (None, None)
        };

        self.gpu = Some(GpuState {
            window,
            surface,
            renderer,
            egui_renderer,
            scene_view_sprite_renderer,
            scene_view_overlay_renderer,
            scene_textures: SceneTextures::default(),
            scene_view_post_framebuffer,
            scene_view_post_pipeline,
        });
    }

    /// Save the current project to its file path.
    ///
    /// The scene of the world is written into the project first, so the
    /// edits of the inspector are not lost.
    pub fn save_project(&mut self) -> Result<(), String> {
        let path = self
            .project_path
            .as_ref()
            .ok_or("No project file open. Use Save As first.")?
            .clone();

        // The world is what the editor shows, so it is what is written.
        if self.current_project.is_some() {
            let scene = self.scene_from_world();
            if let Some(project) = &mut self.current_project {
                project.scenes.insert(scene.id.clone(), scene);
            }
        }

        let Some(project) = &self.current_project else {
            return Err("No project loaded".to_string());
        };
        let toml_str = project.to_toml().map_err(|e| e.to_string())?;
        std::fs::write(&path, toml_str).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Save the current project to a new file path.
    pub fn save_project_as(&mut self, path: PathBuf) -> Result<(), String> {
        self.project_path = Some(path.clone());

        if self.current_project.is_none() {
            self.current_project = Some(Project::default());
        }

        self.save_project()
    }

    /// Load a project from a file path.
    pub fn load_project(&mut self, path: PathBuf) -> Result<(), String> {
        let toml_str = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let project = Project::from_toml(&toml_str).map_err(|e| e.to_string())?;

        self.project_path = Some(path.clone());
        self.current_project = Some(project);

        // Scan the project directory so the asset browser lists its files.
        if let Some(dir) = path.parent() {
            self.project_root = Some(dir.to_path_buf());
            self.asset_browser.scan(dir);
        }

        // Upload the project's textures to the GPU, so the scene view can
        // draw the same art the player draws.
        self.upload_scene_textures();

        // Initialize scene tabs from project scenes
        self.scene_tabs.clear();
        if let Some(project) = &self.current_project {
            for (scene_id, _) in &project.scenes {
                self.scene_tabs.push(SceneTab {
                    scene_id: scene_id.clone(),
                    is_active: false,
                    dirty: false,
                });
            }
            // Activate the first scene tab by default
            if !self.scene_tabs.is_empty() {
                self.scene_tabs[0].is_active = true;
                self.active_scene_tab = Some(0);
            }
        }

        // Clear current world and load scene entities
        self.load_project_scene();

        Ok(())
    }

    /// Create a new project from a template.
    pub fn create_project(
        &mut self,
        name: String,
        path: PathBuf,
        template: String,
    ) -> Result<(), String> {
        // Create project directory
        std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;

        // Create project file
        let project = match template.as_str() {
            "empty_2d" => self.create_empty_2d_project(name, &path)?,
            "platformer" => self.create_platformer_project(name, &path)?,
            "topdown" => self.create_topdown_project(name, &path)?,
            _ => return Err(format!("Unknown template: {}", template)),
        };

        // Write project file
        let project_file = path.join("kiln.toml");
        std::fs::write(&project_file, project.to_toml().map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;

        // Create asset directories
        std::fs::create_dir_all(path.join("art")).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(path.join("scripts")).map_err(|e| e.to_string())?;

        // Create a basic script file
        let script_file = path.join("scripts/main.ks");
        std::fs::write(&script_file, "// Game script\nlet speed = 100.0;\nlet dt = get_delta_time();\nlet dx = get_input_x() * speed * dt;\nlet dy = get_input_y() * speed * dt;\nmove_player(dx, dy);\n").map_err(|e| e.to_string())?;

        // Load the new project
        self.load_project(path)
    }

    /// Create an empty 2D project.
    fn create_empty_2d_project(&self, name: String, path: &Path) -> Result<Project, String> {
        let project_dir = path.file_name().unwrap().to_string_lossy().to_string();
        Ok(Project {
            id: project_dir,
            name,
            version: "0.1.0".to_string(),
            publisher: "Kiln".to_string(),
            icon: "".to_string(),
            features: Vec::new(),
            window: Default::default(),
            mobile: Default::default(),
            scenes: {
                let mut scenes = HashMap::new();
                scenes.insert(
                    "main".to_string(),
                    Scene {
                        id: "main".to_string(),
                        name: "Main".to_string(),
                        script: Some("scripts/main.ks".to_string()),
                        entities: Vec::new(),
                        nested_scenes: Vec::new(),
                    },
                );
                scenes
            },
            tilemaps: HashMap::new(),
            tilesets: HashMap::new(),
        })
    }

    /// Create a platformer project template.
    fn create_platformer_project(&self, name: String, path: &Path) -> Result<Project, String> {
        // For now, same as empty but with platformer script
        let mut project = self.create_empty_2d_project(name, path)?;
        if let Some(scene) = project.scenes.get_mut("main") {
            scene.script = Some("scripts/main.ks".to_string());
        }
        Ok(project)
    }

    /// Create a top-down project template.
    fn create_topdown_project(&self, name: String, path: &Path) -> Result<Project, String> {
        // For now, same as empty but with top-down script
        let mut project = self.create_empty_2d_project(name, path)?;
        if let Some(scene) = project.scenes.get_mut("main") {
            scene.script = Some("scripts/main.ks".to_string());
        }
        Ok(project)
    }

    /// Upload the open project's textures to the scene view renderer.
    ///
    /// Textures that are missing or cannot be decoded are logged, and the
    /// entities that use them fall back to the white texture.
    fn upload_scene_textures(&mut self) {
        let Some(gpu) = self.gpu.as_mut() else {
            return;
        };
        let Some(project) = self.current_project.clone() else {
            gpu.scene_textures = SceneTextures::default();
            return;
        };
        let Some(root) = self.project_root.clone() else {
            gpu.scene_textures = SceneTextures::default();
            return;
        };

        let (textures, errors) = SceneTextures::upload_for_project(
            &mut gpu.scene_view_sprite_renderer,
            gpu.renderer.device(),
            gpu.renderer.queue(),
            &project,
            &root,
        );
        for error in &errors {
            log::warn!("{error}");
        }
        gpu.scene_textures = textures;
    }

    /// Start playing the current project's scene.
    ///
    /// The game runs the scene's script against a fresh world, exactly like
    /// the standalone player does.
    fn start_play(&mut self) -> Result<(), String> {
        let project = self
            .current_project
            .as_ref()
            .ok_or("No project loaded")?
            .clone();
        let root = self
            .project_path
            .as_ref()
            .and_then(|p| p.parent())
            .ok_or("The project has no directory")?
            .to_path_buf();

        let textures = self
            .gpu
            .as_ref()
            .map(|gpu| gpu.scene_textures.clone())
            .unwrap_or_default();
        let (slice, scene_id, world) = start_play_session(&project, &root, &textures)?;

        // The played scene replaces the edited one, so the scene view shows
        // the running game.
        self.world = world;
        self.play = Some(PlaySession {
            slice,
            scene_id,
            error: None,
        });

        // Resume the game from its save, if there is one.
        self.load_game();
        Ok(())
    }

    /// Stop playing and return to editing the scene.
    fn stop_play(&mut self) {
        self.play = None;
        self.is_playing = false;
        self.load_project_scene();
    }

    /// The size of the world the scene view shows, in world units.
    ///
    /// This is the size of the project's game window, so the editor frames
    /// the scene the same way the player does.
    fn world_size(&self) -> (f32, f32) {
        let base = match &self.current_project {
            Some(project) => (project.window.width as f32, project.window.height as f32),
            None => (DEFAULT_WORLD_SIZE.0, DEFAULT_WORLD_SIZE.1),
        };
        let mut max_x = base.0;
        let mut max_y = base.1;
        for (_, tilemap) in self
            .world
            .world_ref()
            .query::<(&Transform, &Tilemap)>()
            .iter()
        {
            let [_, _, right, bottom] = tilemap.world_bounds();
            max_x = max_x.max(right);
            max_y = max_y.max(bottom);
        }
        (max_x, max_y)
    }

    /// Reload the edited scene into the editor world.
    fn load_project_scene(&mut self) {
        let Some(project) = self.current_project.clone() else {
            self.world = GameWorld::new();
            self.entity_textures.clear();
            self.entity_roles.clear();
            self.entity_id_to_hecs.clear();
            self.hierarchy_panel.clear_selection();
            return;
        };
        let Some(scene_id) = project.default_scene_id() else {
            self.world = GameWorld::new();
            self.entity_textures.clear();
            self.entity_roles.clear();
            self.entity_id_to_hecs.clear();
            self.hierarchy_panel.clear_selection();
            return;
        };
        self.world = GameWorld::new();
        self.entity_textures.clear();
        self.entity_roles.clear();
        self.entity_id_to_hecs.clear();
        let textures = self
            .gpu
            .as_ref()
            .map(|gpu| gpu.scene_textures.clone())
            .unwrap_or_default();
        let (world, spawned) = spawn_editor_scene(&project, &scene_id, &textures);
        // Build mapping from project entity ID (SourceId) to hecs Entity.
        for (hecs_entity, source_id) in world.world_ref().query::<&SourceId>().iter() {
            self.entity_id_to_hecs
                .insert(source_id.0.clone(), hecs_entity);
        }
        self.world = world;
        for entity in spawned {
            if let Some(texture) = entity.texture {
                self.entity_textures
                    .insert(entity.source_id.clone(), texture);
            }
            if let Some(role) = entity.role {
                self.entity_roles.insert(entity.source_id, role);
            }
        }
        // Rebuild hierarchy panel with the current scene.
        self.hierarchy_panel.rebuild(&project, &scene_id);
    }

    /// Advance the running game by one frame.
    fn update_play(&mut self, dt: f32) {
        let axis = self.play_axis();
        let EditorApp { world, play, .. } = self;
        let Some(play) = play.as_mut() else {
            return;
        };
        let result = play.slice.step(world, axis, dt as f64);
        play.error = match result {
            Ok(()) => None,
            Err(e) => Some(e.to_string()),
        };
    }

    /// The movement axis from the keys held down in the editor.
    fn play_axis(&self) -> (f64, f64) {
        let ctx = self.egui_winit.egui_ctx();
        let mut x = 0.0;
        let mut y = 0.0;
        ctx.input(|i| {
            for key in &i.keys_down {
                match key {
                    egui::Key::ArrowLeft => x -= 1.0,
                    egui::Key::ArrowRight => x += 1.0,
                    egui::Key::ArrowUp => y += 1.0,
                    egui::Key::ArrowDown => y -= 1.0,
                    egui::Key::A => x -= 1.0,
                    egui::Key::D => x += 1.0,
                    egui::Key::W => y += 1.0,
                    egui::Key::S => y -= 1.0,
                    _ => {}
                }
            }
        });
        (x, y)
    }

    /// Save the current world state as a scene.
    ///
    /// The scene keeps its id and script, so saving does not break the game.
    /// The scene is written into the project and the project is written to
    /// disk, so an edit survives closing the editor.
    pub fn save_scene(&mut self) -> Result<(), String> {
        if self.current_project.is_none() {
            return Err("No project loaded".to_string());
        }
        self.save_project()
    }

    /// Build the scene data of the current world.
    ///
    /// The source id keeps the entity's identity, so the roles and textures
    /// of the scene survive a save.
    fn scene_from_world(&self) -> Scene {
        scene_from_world(
            &self.world,
            &self.entity_textures,
            &self.entity_roles,
            self.current_project.as_ref(),
        )
    }

    /// Pack the project into an archive the player can run.
    ///
    /// The archive holds the manifest and the assets of the project, so it is
    /// the exported game.
    fn export_project(&mut self) {
        let Some(root) = self.project_root.clone() else {
            self.console_log
                .push("[ERROR] Open a project to export it".to_string());
            return;
        };
        let name = self
            .current_project
            .as_ref()
            .map(|p| p.id.clone())
            .unwrap_or_else(|| "game".to_string());
        let output = root.join(format!("{name}.kar"));
        match crate::editing::pack_project(&root, &output) {
            Ok(path) => self
                .console_log
                .push(format!("[INFO] Exported the project to {}", path.display())),
            Err(message) => self.console_log.push(format!("[ERROR] {message}")),
        }
    }

    /// Pack the project with its version, the way a release is built.
    fn package_project(&mut self) {
        let Some(root) = self.project_root.clone() else {
            self.console_log
                .push("[ERROR] Open a project to package it".to_string());
            return;
        };
        let name = self
            .current_project
            .as_ref()
            .map(|p| p.id.clone())
            .unwrap_or_else(|| "game".to_string());
        let version = self
            .current_project
            .as_ref()
            .map(|p| p.version.clone())
            .unwrap_or_else(|| "0.1.0".to_string());
        let output = root.join(format!("{name}-{version}.kar"));
        match crate::editing::pack_project(&root, &output) {
            Ok(path) => self.console_log.push(format!(
                "[INFO] Packaged the project as {} (version {version})",
                path.display()
            )),
            Err(message) => self.console_log.push(format!("[ERROR] {message}")),
        }
    }

    /// Open the documentation of the project.
    ///
    /// The docs are part of the repository, so the editor shows where they
    /// are instead of opening a browser.
    fn open_docs(&mut self) {
        let docs = self
            .project_root
            .as_ref()
            .map(|root| root.join("docs"))
            .unwrap_or_else(|| PathBuf::from("docs"));
        if docs.is_dir() {
            self.console_log
                .push(format!("[INFO] The documentation is in {}", docs.display()));
        } else {
            self.console_log.push(format!(
                "[INFO] The documentation is in {} (the folder does not exist yet)",
                docs.display()
            ));
        }
    }

    /// Undo the last scene edit.
    pub fn undo(&mut self) -> Result<(), String> {
        let current = self.scene_from_world();
        let Some(previous) = self.history.undo(current) else {
            return Err("Nothing to undo".to_string());
        };
        self.apply_scene(previous)
    }

    /// Redo the last undone scene edit.
    pub fn redo(&mut self) -> Result<(), String> {
        let current = self.scene_from_world();
        let Some(next) = self.history.redo(current) else {
            return Err("Nothing to redo".to_string());
        };
        self.apply_scene(next)
    }

    /// Apply a scene to the project and the editor world.
    fn apply_scene(&mut self, scene: Scene) -> Result<(), String> {
        let scene_id = scene.id.clone();
        if let Some(project) = &mut self.current_project {
            project.scenes.insert(scene_id, scene);
        }
        self.load_project_scene();
        Ok(())
    }

    /// Save the running game to its save slot.
    fn save_game(&mut self) {
        let Some(play) = self.play.as_ref() else {
            return;
        };
        let Some(dir) = self.save_dir() else {
            return;
        };
        match kiln_runtime::save_state(&dir, 0, play.slice.state()) {
            Ok(()) => self.console_log.push("[INFO] Game saved".to_string()),
            Err(e) => {
                let message = format!("[ERROR] Could not save the game: {e}");
                log::error!("{message}");
                self.console_log.push(message);
            }
        }
    }

    /// Load the running game from its save slot.
    fn load_game(&mut self) {
        let Some(dir) = self.save_dir() else {
            return;
        };
        let state = match kiln_runtime::load_state(&dir, 0) {
            Ok(state) => state,
            // No save yet is not an error; the game just starts fresh.
            Err(_) => return,
        };
        let Some(play) = self.play.as_mut() else {
            return;
        };
        play.slice.set_state(state);
        play.slice.sync_world(&mut self.world);
        self.console_log.push("[INFO] Game loaded".to_string());
    }

    /// The directory the game state is saved in.
    fn save_dir(&self) -> Option<PathBuf> {
        self.project_root
            .as_ref()
            .map(|root| root.join(".kiln-save"))
    }

    /// Load a scene into the current world.
    pub fn load_scene(&mut self) -> Result<(), String> {
        if self.current_project.is_none() {
            return Err("No project loaded".to_string());
        }
        let Some(scene_id) = self
            .current_project
            .as_ref()
            .and_then(|p| p.default_scene_id())
        else {
            return Err("The project has no scenes".to_string());
        };
        self.load_project_scene();
        let _ = scene_id;
        Ok(())
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

        // Auto-reopen last project if enabled and no project is currently open
        if self.project_hub.auto_reopen_last && self.current_project.is_none() {
            if let Some(last_project) = self.project_hub.last_project() {
                let path = last_project.path.clone();
                if path.exists() {
                    if let Err(e) = self.load_project(path) {
                        eprintln!("Failed to auto-reopen last project: {}", e);
                    } else {
                        // Add back to recent projects since we just opened it
                        self.project_hub.add_recent(
                            last_project.path.clone(),
                            last_project.name.clone(),
                        );
                    }
                }
            }
        }
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
                    // Also resize the scene view post framebuffer if it exists.
                    if let Some(fb) = gpu.scene_view_post_framebuffer.as_mut() {
                        fb.resize(gpu.renderer.device(), size.width, size.height, gpu.renderer.tier());
                    }
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
        let pass_timings: Vec<PassTiming> = self
            .gpu
            .as_ref()
            .map(|g| g.renderer.pass_timings().to_vec())
            .unwrap_or_default();
        let profiler_supported = self
            .gpu
            .as_ref()
            .map(|g| g.renderer.profiler().supported())
            .unwrap_or(false);

        // Advance the running game, if the editor is in play mode.
        if self.play.is_some() {
            self.update_play(1.0 / 60.0);
        }

        // The world size and the scene view rectangle are read before the GPU
        // state is borrowed, so the scene can be drawn into the panel.
        let world = self.world_size();
        let scene_view_rect = self.scene_view_rect;

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

        // Track menu actions to execute after UI pass
        let mut menu_actions = Vec::new();
        // Track the save and load requests from the keyboard.
        let mut save_requested = false;
        let mut load_requested = false;

        // Top menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Project").clicked() {
                        menu_actions.push(MenuAction::NewProject);
                        ui.close_menu();
                    }
                    if ui.button("Open Project...").clicked() {
                        menu_actions.push(MenuAction::OpenProject);
                        ui.close_menu();
                    }
                    if ui.button("Save Project").clicked() {
                        menu_actions.push(MenuAction::SaveProject);
                        ui.close_menu();
                    }
                    if ui.button("Save Project As...").clicked() {
                        menu_actions.push(MenuAction::SaveProjectAs);
                        ui.close_menu();
                    }
                    if ui.button("Save Scene").clicked() {
                        menu_actions.push(MenuAction::SaveScene);
                        ui.close_menu();
                    }
                    if ui.button("Load Scene").clicked() {
                        menu_actions.push(MenuAction::LoadScene);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Export...").clicked() {
                        menu_actions.push(MenuAction::ExportProject);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        std::process::exit(0);
                    }
                });

                ui.menu_button("Edit", |ui| {
                    if ui
                        .add_enabled(self.history.can_undo(), egui::Button::new("Undo"))
                        .clicked()
                    {
                        menu_actions.push(MenuAction::Undo);
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(self.history.can_redo(), egui::Button::new("Redo"))
                        .clicked()
                    {
                        menu_actions.push(MenuAction::Redo);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Preferences...").clicked() {
                        self.dock_state.add_window(vec![Tab::Preferences]);
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
                    if ui.button("Animation Editor").clicked() {
                        self.dock_state.add_window(vec![Tab::AnimationEditor]);
                        ui.close_menu();
                    }
                    if ui.button("Build / Export").clicked() {
                        self.dock_state.add_window(vec![Tab::BuildPanel]);
                        ui.close_menu();
                    }
                    if ui.button("Project Hub").clicked() {
                        self.dock_state.add_window(vec![Tab::ProjectHub]);
                        ui.close_menu();
                    }
                    if ui.button("Profiler").clicked() {
                        self.dock_state.add_window(vec![Tab::ProfilerPanel]);
                        ui.close_menu();
                    }
                    if ui.button("Debugger").clicked() {
                        self.dock_state.add_window(vec![Tab::DebuggerPanel]);
                        ui.close_menu();
                    }
                    if ui.button("VFX Editor").clicked() {
                        self.dock_state.add_window(vec![Tab::VfxEditor]);
                        ui.close_menu();
                    }
                    if ui.button("Audio Editor").clicked() {
                        self.dock_state.add_window(vec![Tab::AudioEditor]);
                        ui.close_menu();
                    }
                    if ui.button("Physics Editor").clicked() {
                        self.dock_state.add_window(vec![Tab::PhysicsEditor]);
                        ui.close_menu();
                    }
                    if ui.button("UI Layout Editor").clicked() {
                        self.dock_state.add_window(vec![Tab::UiLayoutEditor]);
                        ui.close_menu();
                    }
                    if ui.button("Preferences").clicked() {
                        self.dock_state.add_window(vec![Tab::Preferences]);
                        ui.close_menu();
                    }
                    if ui.button("Version Control").clicked() {
                        self.dock_state.add_window(vec![Tab::VersionControl]);
                        ui.close_menu();
                    }
                    if ui.button("Plugin Manager").clicked() {
                        self.dock_state.add_window(vec![Tab::PluginManagerPanel]);
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
                    if ui.button("Tilemap Editor").clicked() {
                        self.dock_state.add_window(vec![Tab::TilemapEditor]);
                        ui.close_menu();
                    }
                    if ui.button("Hierarchy").clicked() {
                        self.dock_state.add_window(vec![Tab::Hierarchy]);
                        ui.close_menu();
                    }
                    if ui.button("Inspector").clicked() {
                        self.dock_state.add_window(vec![Tab::Inspector]);
                        ui.close_menu();
                    }
                });

                ui.menu_button("Project", |ui| {
                    if ui.button("Build Settings").clicked() {
                        self.dock_state.add_window(vec![Tab::Settings]);
                        ui.close_menu();
                    }
                    if ui.button("Package").clicked() {
                        menu_actions.push(MenuAction::PackageProject);
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
                        menu_actions.push(MenuAction::OpenDocs);
                        ui.close_menu();
                    }
                    if ui.button("About Kiln").clicked() {
                        menu_actions.push(MenuAction::About);
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
                                    if ui
                                        .add_sized([200.0, 40.0], egui::Button::new("New Project"))
                                        .clicked()
                                    {
                                        self.show_welcome = false;
                                    }
                                    ui.add_space(16.0);
                                    if ui
                                        .add_sized([200.0, 40.0], egui::Button::new("Open Project"))
                                        .clicked()
                                    {
                                        self.show_welcome = false;
                                        self.play_toggle_requested = false;
                                        self.open_project_requested = true;
                                    }
                                });

                                ui.add_space(16.0);
                                ui.horizontal(|ui| {
                                    ui.label("Project file:");
                                    ui.add_sized(
                                        [320.0, 24.0],
                                        egui::TextEdit::singleline(&mut self.project_path_input),
                                    );
                                });
                                ui.add_space(8.0);
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
        }

        // The about window tells what the editor is and which version it is.
        if self.show_about {
            egui::Window::new("About Kiln")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ctx, |ui| {
                    ui.heading("Kiln");
                    ui.label("A lightweight game engine for indie developers.");
                    ui.add_space(8.0);
                    ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                    ui.label("Renderer: wgpu");
                    ui.add_space(8.0);
                    if ui.button("Close").clicked() {
                        self.show_about = false;
                    }
                });
        }

        if !self.show_welcome {
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
                    pass_timings: &pass_timings,
                    profiler_supported,
                    ui_designer: &mut self.ui_designer,
                    prefab_editor: &mut self.prefab_editor,
                    gameplay_editor: &mut self.gameplay_editor,
                    asset_browser: &mut self.asset_browser,
                    tilemap_editor: &mut self.tilemap_editor,
                    hierarchy_panel: &mut self.hierarchy_panel,
                    inspector_panel: &mut self.inspector_panel,
                    graph_editor: &mut self.graph_editor,
                    current_project: &mut self.current_project,
                    play_toggle_requested: &mut self.play_toggle_requested,
                    scene_save_requested: &mut self.scene_save_requested,
                    scene_load_requested: &mut self.scene_load_requested,
                    play: self.play.as_ref(),
                    fps,
                    history: &mut self.history,
                    entity_textures: &self.entity_textures,
                    entity_roles: &self.entity_roles,
                    console_log: &mut self.console_log,
                    scene_view_rect: &mut self.scene_view_rect,
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

        // M28: default 2D frame graph (tier-filtered) with GPU timestamps, then egui.
        let (w, h) = gpu.renderer.size();
        let frame_graph = Frame2dGraph::build(w, h, tier, gpu.renderer.pass_config());
        let compiled = frame_graph.compile().expect("frame graph");
        {
            let profiler = gpu.renderer.profiler_mut();
            execute_passes(
                &compiled,
                &mut encoder,
                &view,
                tier,
                Some(profiler),
                |ctx| match ctx.name {
                    "main" => clear_view(
                        ctx.encoder,
                        ctx.output_view,
                        wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.12,
                            a: 1.0,
                        },
                    ),
                    _ => {
                        let _ = ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some(ctx.name),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: ctx.output_view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Load,
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: None,
                            timestamp_writes: None,
                            occlusion_query_set: None,
                        });
                    }
                },
            );
        }

        let egui_begin = gpu.renderer.profiler_mut().begin_pass(&mut encoder, "egui");
        {
            let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
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
        }
        gpu.renderer
            .profiler_mut()
            .end_pass(&mut encoder, egui_begin);
        gpu.renderer.profiler_mut().resolve(&mut encoder);

        // Render the scene view with the engine's sprite renderer.
        //
        // The scene is drawn into the scene-view panel: the panel's rectangle
        // becomes a viewport, the world is projected with the same camera the
        // player uses, and the HUD is drawn on top in screen space. The
        // engine owns every pixel of the panel, so the editor shows what the
        // player shows.
        let panel = scene_view_rect
            .map(|rect| scene_view::rect_to_pixels(rect, full_output.pixels_per_point));
        if let Some(panel) = panel {
            let viewport = scene_view::letterbox(panel.width, panel.height, world.0, world.1);

            if !viewport.is_empty() {
                let queue = gpu.renderer.queue();
                let origin_x = panel.x + viewport.x;
                let origin_y = panel.y + viewport.y;

                // Queue the world: background, grid, tilemaps, sprites.
                gpu.scene_view_sprite_renderer.begin_frame();
                gpu.scene_view_sprite_renderer.push(scene_view::background(
                    world.0, world.1, BACKGROUND_COLOR,
                ));
                for line in scene_view::grid_lines(world.0, world.1, GRID_SPACING, GRID_COLOR) {
                    gpu.scene_view_sprite_renderer.push(line);
                }
                let world_camera = scene_view::world_camera(world.0, world.1);
                let view_bounds = world_camera.view_bounds();
                for (_, tilemap) in self.world.world().query::<(&Transform, &Tilemap)>().iter() {
                    for (texture, instance) in tilemap.collect_visible(view_bounds) {
                        gpu.scene_view_sprite_renderer.push_textured(texture, instance);
                    }
                }
                for (transform, sprite) in
                    self.world.world().query::<(&Transform, &Sprite)>().iter()
                {
                    gpu.scene_view_sprite_renderer.push_textured(
                        sprite.texture,
                        SpriteInstance {
                            position: [transform.position.x, transform.position.y],
                            size: sprite.size,
                            color: sprite.color,
                            uv_rect: sprite.uv_rect,
                        },
                    );
                }

                // Determine render target: post framebuffer if available, else main view.
                let (scene_target_view, use_post) = if let Some(fb) = gpu.scene_view_post_framebuffer.as_ref() {
                    (fb.color_view(), true)
                } else {
                    (&view, false)
                };

                // Render scene to target (post framebuffer or main view).
                {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("scene_view_pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: scene_target_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: BACKGROUND_COLOR[0] as f64,
                                    g: BACKGROUND_COLOR[1] as f64,
                                    b: BACKGROUND_COLOR[2] as f64,
                                    a: BACKGROUND_COLOR[3] as f64,
                                }),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    pass.set_viewport(origin_x as f32, origin_y as f32,
                        viewport.width as f32, viewport.height as f32, 0.0, 1.0);
                    pass.set_scissor_rect(origin_x, origin_y, viewport.width, viewport.height);
                    gpu.scene_view_sprite_renderer.flush(&mut pass, &world_camera, queue);
                }

                // Apply post-processing if available.
                if use_post {
                    if let Some(pipeline) = gpu.scene_view_post_pipeline.as_mut() {
                        if let Some(fb) = gpu.scene_view_post_framebuffer.as_ref() {
                            pipeline.apply(
                                gpu.renderer.device(),
                                gpu.renderer.queue(),
                                &mut encoder,
                                fb,
                                &view,
                            );
                        }
                    }
                }

                // Render HUD overlay on top.
                gpu.scene_view_overlay_renderer.begin_frame();
                if let Some(play) = &self.play {
                    let text = kiln_ui::TextRenderer::new();
                    let mut lines = play.slice.hud_lines();
                    if let Some(error) = &play.error {
                        lines.push(format!("ERROR: {error}"));
                    }
                    for (index, line) in lines.iter().enumerate() {
                        let y = 16.0 + index as f32 * 20.0;
                        let color = if line.starts_with("ERROR:") {
                            [1.0, 0.35, 0.35, 1.0]
                        } else {
                            [1.0, 1.0, 1.0, 1.0]
                        };
                        let quads = text.render(line, 16.0, y, 16.0, color);
                        for quad in quads {
                            gpu.scene_view_overlay_renderer.push(quad);
                        }
                    }
                }

                let overlay_camera = scene_view::overlay_camera(ViewportRect {
                    x: origin_x, y: origin_y, width: viewport.width, height: viewport.height,
                });
                {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("scene_view_overlay_pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    pass.set_viewport(origin_x as f32, origin_y as f32,
                        viewport.width as f32, viewport.height as f32, 0.0, 1.0);
                    pass.set_scissor_rect(origin_x, origin_y, viewport.width, viewport.height);
                    gpu.scene_view_overlay_renderer.flush(&mut pass, &overlay_camera, queue);
                }
            }
        }

            }
        }

        gpu.renderer.queue().submit([encoder.finish()]);

        // Execute menu actions after UI pass (borrows are released)
        for action in menu_actions {
            match action {
                MenuAction::NewProject => {
                    self.show_welcome = true;
                }
                MenuAction::OpenProject => {
                    let path = PathBuf::from(self.project_path_input.trim());
                    match self.load_project(path) {
                        Ok(()) => self.console_log.push("[INFO] Project loaded".to_string()),
                        Err(e) => {
                            let message = format!("[ERROR] Could not load the project: {e}");
                            log::error!("{message}");
                            self.console_log.push(message);
                        }
                    }
                }
                MenuAction::SaveProject => {
                    if let Err(e) = self.save_project() {
                        eprintln!("Failed to save project: {}", e);
                    }
                }
                MenuAction::SaveProjectAs => {
                    if let Err(e) = self.save_project_as(PathBuf::from("kiln.toml")) {
                        eprintln!("Failed to save project as: {}", e);
                    }
                }
                MenuAction::SaveScene => {
                    if let Err(e) = self.save_scene() {
                        eprintln!("Failed to save scene: {}", e);
                    }
                }
                MenuAction::LoadScene => {
                    if let Err(e) = self.load_scene() {
                        eprintln!("Failed to load scene: {}", e);
                    }
                }
                MenuAction::Undo => {
                    if let Err(e) = self.undo() {
                        self.console_log.push(format!("[INFO] {e}"));
                    }
                }
                MenuAction::Redo => {
                    if let Err(e) = self.redo() {
                        self.console_log.push(format!("[INFO] {e}"));
                    }
                }
                MenuAction::ExportProject => {
                    self.export_project();
                }
                MenuAction::PackageProject => {
                    self.package_project();
                }
                MenuAction::OpenDocs => {
                    self.open_docs();
                }
                MenuAction::About => {
                    self.show_about = true;
                }
            }
        }

        // Save and load the running game, like the standalone player does.
        if self.play.is_some() {
            let ctx = self.egui_winit.egui_ctx();
            ctx.input(|i| {
                if i.key_pressed(egui::Key::F5) {
                    save_requested = true;
                }
                if i.key_pressed(egui::Key::F9) {
                    load_requested = true;
                }
            });
        }
        if save_requested {
            self.save_game();
        }
        if load_requested {
            self.load_game();
        }

        // Handle the open-project request from the welcome screen.
        if std::mem::take(&mut self.open_project_requested) {
            let path = PathBuf::from(self.project_path_input.trim());
            match self.load_project(path) {
                Ok(()) => self.console_log.push("[INFO] Project loaded".to_string()),
                Err(e) => {
                    let message = format!("[ERROR] Could not load the project: {e}");
                    log::error!("{message}");
                    self.console_log.push(message);
                }
            }
        }

        // Handle the play/pause request from the scene toolbar.
        if std::mem::take(&mut self.play_toggle_requested) {
            if self.is_playing {
                self.is_playing = false;
                self.stop_play();
            } else {
                // Playing needs a project with a scripted scene.
                match self.start_play() {
                    Ok(()) => self.is_playing = true,
                    Err(message) => {
                        self.console_log.push(format!("[ERROR] {message}"));
                    }
                }
            }
        }

        // Handle scene save/load requests
        if std::mem::take(&mut self.scene_save_requested) {
            if let Err(e) = self.save_scene() {
                eprintln!("Failed to save scene: {}", e);
            }
        }
        if std::mem::take(&mut self.scene_load_requested) {
            if let Err(e) = self.load_scene() {
                eprintln!("Failed to load scene: {}", e);
            }
        }

        // Update the frame counter and the measured frame rate.
        self.frame_count += 1;
        let elapsed = self.last_frame.elapsed();
        if elapsed.as_secs_f32() > 0.0 {
            self.fps = elapsed.as_secs_f32().recip();
        }
        self.last_frame = Instant::now();
    }
}

// Standalone function for status bar (no self borrow issues)
fn show_status_bar(
    ctx: &egui::Context,
    fps: f32,
    entity_count: u32,
    tier: kiln_render::RenderTier,
    is_playing: bool,
) {
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
    pass_timings: &'a [PassTiming],
    profiler_supported: bool,
    ui_designer: &'a mut UIDesignerState,
    prefab_editor: &'a mut PrefabEditorState,
    gameplay_editor: &'a mut GameplayEditorState,
    asset_browser: &'a mut AssetBrowser,
    tilemap_editor: &'a mut TilemapEditorState,
    hierarchy_panel: &'a mut HierarchyPanel,
    inspector_panel: &'a mut InspectorPanel,
    graph_editor: &'a mut GraphEditorState,
    current_project: &'a mut Option<Project>,
    play_toggle_requested: &'a mut bool,
    scene_save_requested: &'a mut bool,
    scene_load_requested: &'a mut bool,
    /// The running game session, drawn over the scene view.
    play: Option<&'a PlaySession>,
    /// The measured frame rate, in frames per second.
    fps: f32,
    /// The history of scene edits, used by undo and redo.
    history: &'a mut SceneHistory,
    /// Asset path of each scene entity's texture, keyed by entity id.
    entity_textures: &'a HashMap<String, String>,
    /// Gameplay role of each scene entity, keyed by entity id.
    entity_roles: &'a HashMap<String, String>,
    /// Lines shown in the console panel.
    console_log: &'a mut Vec<String>,
    /// Screen-space rectangle of the scene view panel, set by the panel each
    /// frame so the render loop can draw the scene into it.
    scene_view_rect: &'a mut Option<egui::Rect>,
}

impl TabViewer<'_> {
    /// Record the current scene, so the edit can be undone.
    fn record_scene(&mut self) {
        let scene = scene_from_world(
            &*self.world,
            self.entity_textures,
            self.entity_roles,
            self.current_project.as_ref(),
        );
        self.history.record(scene);
    }

    /// Add an entity to the middle of the scene.
    ///
    /// The scene is recorded first, so the new entity can be undone.
    fn add_entity(&mut self) {
        if self.editing_locked() {
            return;
        }
        self.record_scene();

        let existing: Vec<String> = self.entity_textures.keys().cloned().collect();
        let size = self.world_size();
        let position = [size.0 / 2.0, size.1 / 2.0];
        let (_, entity) = crate::editing::add_entity(
            self.world,
            &existing,
            position,
            crate::editing::EntityDefaults::default(),
        );
        *self.selected_entity = Some(entity);
    }

    /// The size of the world the scene view shows, in world units.
    fn world_size(&self) -> (f32, f32) {
        match self.current_project.as_ref() {
            Some(project) => (project.window.width as f32, project.window.height as f32),
            None => (DEFAULT_WORLD_SIZE.0, DEFAULT_WORLD_SIZE.1),
        }
    }

    /// Remove a component from an entity.
    fn remove_component(&mut self, entity: hecs::Entity, kind: ComponentKind) {
        if self.editing_locked() {
            return;
        }
        // The scene is recorded before the change, so the change can be
        // undone. It is only kept when the component was really there.
        let before = scene_from_world(
            &*self.world,
            self.entity_textures,
            self.entity_roles,
            self.current_project.as_ref(),
        );
        let removed = match kind {
            ComponentKind::Transform => crate::editing::remove_transform(self.world, entity),
            ComponentKind::Sprite => crate::editing::remove_sprite(self.world, entity),
            ComponentKind::Role => crate::editing::remove_role(self.world, entity),
            ComponentKind::Tilemap => crate::editing::remove_tilemap(self.world, entity),
        };
        if !removed {
            return;
        }
        self.history.record(before);
        *self.selected_entity = None;
    }

    /// Add a component to an entity.
    fn add_component(&mut self, entity: hecs::Entity, kind: ComponentKind) {
        if self.editing_locked() {
            return;
        }
        let before = scene_from_world(
            &*self.world,
            self.entity_textures,
            self.entity_roles,
            self.current_project.as_ref(),
        );
        let added = match kind {
            ComponentKind::Transform => crate::editing::add_transform(self.world, entity),
            ComponentKind::Sprite => crate::editing::add_sprite(
                self.world,
                entity,
                crate::editing::EntityDefaults::default(),
            ),
            ComponentKind::Role => crate::editing::add_role(self.world, entity),
            ComponentKind::Tilemap => crate::editing::add_tilemap(self.world, entity),
        };
        if !added {
            return;
        }
        self.history.record(before);
    }

    /// Whether the scene may not be edited right now.
    ///
    /// While the game is playing the world belongs to the running game, so
    /// editing it would change the game instead of the scene.
    fn editing_locked(&self) -> bool {
        *self.is_playing
    }

    /// Create a prefab from the selected entity.
    ///
    /// The prefab keeps the entity's components, so an instance of it looks
    /// like the entity it was made from.
    fn create_prefab_from_selection(&mut self) {
        let Some(entity) = *self.selected_entity else {
            self.log("[INFO] Select an entity to make a prefab of");
            return;
        };

        // The source id is the identity of the entity in the scene data, so
        // the prefab is made from the entity the scene holds.
        let source_id = {
            let world = self.world.world();
            world.get::<&SourceId>(entity).ok().map(|id| id.0.clone())
        };
        let Some(source_id) = source_id else {
            self.log("[INFO] The selected entity has no scene data");
            return;
        };

        let scene = scene_from_world(
            self.world,
            self.entity_textures,
            self.entity_roles,
            self.current_project.as_ref(),
        );
        let Some(source) = scene.entities.iter().find(|e| e.id == source_id).cloned() else {
            self.log("[INFO] The selected entity has no scene data");
            return;
        };

        let name = format!("Prefab_{}", source.id);
        let prefab = crate::editing::prefab_from_entity(&source, &name);
        let id = prefab.id.clone();
        self.prefab_editor.add_prefab(prefab);
        self.prefab_editor.select_prefab(&id);
        self.log(&format!("[INFO] Created the prefab {name}"));
    }

    /// Add an instance of the selected prefab to the scene.
    fn instantiate_selected_prefab(&mut self) {
        let Some(prefab) = self.prefab_editor.selected_prefab().cloned() else {
            self.log("[INFO] Select a prefab to instantiate it");
            return;
        };

        let before = scene_from_world(
            self.world,
            self.entity_textures,
            self.entity_roles,
            self.current_project.as_ref(),
        );
        let existing: Vec<String> = self.entity_textures.keys().cloned().collect();
        let size = self.world_size();
        let position = [size.0 / 2.0, size.1 / 2.0];
        let id = crate::editing::instantiate_prefab(self.world, &prefab, &existing, position);
        match id {
            Some(id) => {
                self.history.record(before);
                self.log(&format!("[INFO] Instantiated the prefab as {id}"));
            }
            None => self.log("[ERROR] The prefab could not be instantiated"),
        }
    }

    /// Create an empty scene in the project.
    fn create_scene(&mut self) {
        let Some(project) = self.current_project.as_mut() else {
            self.log("[INFO] Open a project to add a scene to it");
            return;
        };
        let name = "New Scene".to_string();
        let id = crate::editing::create_scene(project, &name);
        self.log(&format!("[INFO] Created the scene {id}"));
    }

    /// Add another scene as a nested scene of the current one.
    fn nest_scene(&mut self) {
        let Some(project) = self.current_project.as_mut() else {
            self.log("[INFO] Open a project to nest a scene in it");
            return;
        };
        let Some(parent_id) = project.default_scene_id() else {
            self.log("[INFO] The project has no scene to nest in");
            return;
        };
        // The nested scene is the first scene that is not the parent.
        let Some(child_id) = project.scenes.keys().find(|id| **id != parent_id).cloned() else {
            self.log("[INFO] The project has no other scene to nest");
            return;
        };

        match crate::editing::nest_scene(project, &parent_id, &child_id) {
            Ok(()) => self.log(&format!(
                "[INFO] Nested the scene {child_id} in {parent_id}"
            )),
            Err(message) => self.log(&format!("[ERROR] {message}")),
        }
    }

    /// Import the assets of the project into the asset cache.
    ///
    /// The cache holds the hash of every imported file, so a later import
    /// only picks up the files that changed.
    fn import_assets(&mut self) {
        let Some(dir) = self.asset_browser.root().map(|p| p.to_path_buf()) else {
            self.log("[INFO] Open a project to import its assets");
            return;
        };
        let cache_dir = dir.join(".kiln-cache");
        let mut cache = kiln_assets::AssetCache::new(cache_dir);
        if let Err(e) = cache.load() {
            self.log(&format!("[ERROR] The asset cache could not be read: {e}"));
            return;
        }
        let results = match cache.import_dir(&dir) {
            Ok(results) => results,
            Err(e) => {
                self.log(&format!("[ERROR] The assets could not be imported: {e}"));
                return;
            }
        };
        if let Err(e) = cache.save() {
            self.log(&format!("[ERROR] The asset cache could not be saved: {e}"));
            return;
        }

        let imported = results
            .iter()
            .filter(|r| matches!(r, kiln_assets::ImportResult::Imported { .. }))
            .count();
        let skipped = results.len() - imported;
        self.log(&format!(
            "[INFO] Imported {imported} assets, {skipped} were unchanged"
        ));
    }

    /// Add a line to the console.
    fn log(&mut self, message: &str) {
        self.console_log.push(message.to_string());
    }
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
                            ui.add_sized([100.0, 28.0], egui::Button::new("■ Stop"))
                        } else {
                            ui.add_sized([100.0, 28.0], egui::Button::new("▶ Play"))
                        };
                        if play_btn.clicked() {
                            *self.play_toggle_requested = true;
                        }

                        ui.separator();

                        if ui.button("Save Scene").clicked() {
                            *self.scene_save_requested = true;
                        }
                        if ui.button("Load Scene").clicked() {
                            *self.scene_load_requested = true;
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // While playing, the toolbar shows the scene that is
                            // running, so it is clear what the view shows.
                            if let Some(play) = &self.play {
                                ui.label(format!("Playing: {}", play.scene_id));
                            }
                            ui.label(format!("FPS: {:.0}", self.fps));
                        });
                    });

                    // Scene tabs bar
                    if !self.scene_tabs.is_empty() {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);
                            
                            // Add scene button
                            if ui.button("+").clicked() {
                                let scene_id = self.next_scene_id();
                                self.add_scene_tab(scene_id.clone());
                                self.active_scene_tab = Some(self.scene_tabs.len() - 1);
                            }
                            
                            for (idx, tab) in self.scene_tabs.iter().enumerate() {
                                let is_active = self.active_scene_tab == Some(idx);
                                let mut selected = is_active;
                                
                                let mut tab_ui = ui.add(egui::SelectableLabel::new(
                                    selected, 
                                    format!("{}{}", if tab.dirty { "*" } else { "" }, tab.scene_id)
                                ));
                                
                                if tab_ui.clicked() {
                                    self.active_scene_tab = Some(idx);
                                    self.switch_scene_tab(idx);
                                }
                                
                                // Close on right-click
                                if tab_ui.secondary_clicked() {
                                    self.close_scene_tab(idx);
                                }
                            }
                        });
                    }

                    ui.add_space(8.0);

                    // The engine draws the scene here, so the panel must not
                    // paint over it. The rectangle is handed to the render
                    // loop, which converts it to pixels and draws the scene
                    // with the engine's sprite renderer.
                    *self.scene_view_rect = Some(rect);

                    // The grid is drawn by the engine as world-space quads, so
                    // it lines up with the world instead of the panel.

                    // The sprites and the HUD are drawn by the engine too, so
                    // the editor shows the same pixels the player shows. The
                    // panel itself draws nothing over the canvas.

                    // Entity list for selection
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.heading("Entities");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add_enabled(
                                    !self.editing_locked(),
                                    egui::Button::new("+ Add Entity"),
                                )
                                .clicked()
                            {
                                self.add_entity();
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

                    // The selected entity is copied out, so the world can be
                    // borrowed while the inspector edits it.
                    let selected = *self.selected_entity;
                    if let Some(entity) = selected {
                        ui.label(format!("Entity ID: {:?}", entity.id()));
                        ui.add_space(8.0);

                        // Read current values once
                        let world = self.world.world();
                        let transform = world.get::<&Transform>(entity).ok().map(|t| *t);
                        let sprite = world.get::<&Sprite>(entity).ok().map(|s| *s);

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
                                                ui.menu_button("⋮", |ui| {
                                                    if ui.button("Remove Component").clicked() {
                                                        self.remove_component(
                                                            entity,
                                                            ComponentKind::Transform,
                                                        );
                                                        ui.close_menu();
                                                    }
                                                });
                                            },
                                        );
                                    });
                                    ui.add_space(8.0);

                                    ui.horizontal(|ui| {
                                        ui.label("Position");
                                        ui.add_space(16.0);
                                        ui.label("X:");
                                        let response = ui.add_enabled(
                                            !self.editing_locked(),
                                            egui::DragValue::new(&mut transform.position.x)
                                                .speed(1.0),
                                        );
                                        // A drag is one edit, so the scene is
                                        // recorded when the drag starts.
                                        if response.drag_started() {
                                            self.record_scene();
                                        }
                                        if response.changed() {
                                            let world = self.world.world_mut();
                                            if let Ok(mut t) = world.get::<&mut Transform>(entity) {
                                                t.position.x = transform.position.x;
                                            }
                                        }
                                        ui.add_space(8.0);
                                        ui.label("Y:");
                                        let response = ui.add_enabled(
                                            !self.editing_locked(),
                                            egui::DragValue::new(&mut transform.position.y)
                                                .speed(1.0),
                                        );
                                        if response.drag_started() {
                                            self.record_scene();
                                        }
                                        if response.changed() {
                                            let world = self.world.world_mut();
                                            if let Ok(mut t) = world.get::<&mut Transform>(entity) {
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
                                                ui.menu_button("⋮", |ui| {
                                                    if ui.button("Remove Component").clicked() {
                                                        self.remove_component(
                                                            entity,
                                                            ComponentKind::Sprite,
                                                        );
                                                        ui.close_menu();
                                                    }
                                                });
                                            },
                                        );
                                    });
                                    ui.add_space(8.0);

                                    ui.horizontal(|ui| {
                                        ui.label("Color");
                                        ui.add_space(16.0);
                                        ui.label("R:");
                                        let response = ui.add_enabled(
                                            !self.editing_locked(),
                                            egui::DragValue::new(&mut sprite.color[0])
                                                .speed(0.01)
                                                .range(0.0..=1.0),
                                        );
                                        if response.drag_started() {
                                            self.record_scene();
                                        }
                                        if response.changed() {
                                            let world = self.world.world_mut();
                                            if let Ok(mut s) = world.get::<&mut Sprite>(entity) {
                                                s.color[0] = sprite.color[0];
                                            }
                                        }
                                        ui.add_space(8.0);
                                        ui.label("G:");
                                        let response = ui.add_enabled(
                                            !self.editing_locked(),
                                            egui::DragValue::new(&mut sprite.color[1])
                                                .speed(0.01)
                                                .range(0.0..=1.0),
                                        );
                                        if response.drag_started() {
                                            self.record_scene();
                                        }
                                        if response.changed() {
                                            let world = self.world.world_mut();
                                            if let Ok(mut s) = world.get::<&mut Sprite>(entity) {
                                                s.color[1] = sprite.color[1];
                                            }
                                        }
                                        ui.add_space(8.0);
                                        ui.label("B:");
                                        let response = ui.add_enabled(
                                            !self.editing_locked(),
                                            egui::DragValue::new(&mut sprite.color[2])
                                                .speed(0.01)
                                                .range(0.0..=1.0),
                                        );
                                        if response.drag_started() {
                                            self.record_scene();
                                        }
                                        if response.changed() {
                                            let world = self.world.world_mut();
                                            if let Ok(mut s) = world.get::<&mut Sprite>(entity) {
                                                s.color[2] = sprite.color[2];
                                            }
                                        }
                                        ui.add_space(8.0);
                                        ui.label("A:");
                                        let response = ui.add_enabled(
                                            !self.editing_locked(),
                                            egui::DragValue::new(&mut sprite.color[3])
                                                .speed(0.01)
                                                .range(0.0..=1.0),
                                        );
                                        if response.drag_started() {
                                            self.record_scene();
                                        }
                                        if response.changed() {
                                            let world = self.world.world_mut();
                                            if let Ok(mut s) = world.get::<&mut Sprite>(entity) {
                                                s.color[3] = sprite.color[3];
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
                        // The components that are missing can be added, so
                        // an entity can be built up piece by piece.
                        let has_transform = transform.is_some();
                        let has_sprite = sprite.is_some();
                        let has_role = world.get::<&Role>(entity).is_ok();
                        let has_tilemap = world.get::<&Tilemap>(entity).is_ok();
                        if !has_transform || !has_sprite || !has_role || !has_tilemap {
                            ui.horizontal(|ui| {
                                if !has_transform && ui.button("+ Transform").clicked() {
                                    self.add_component(entity, ComponentKind::Transform);
                                }
                                if !has_sprite && ui.button("+ Sprite").clicked() {
                                    self.add_component(entity, ComponentKind::Sprite);
                                }
                                if !has_role && ui.button("+ Role").clicked() {
                                    self.add_component(entity, ComponentKind::Role);
                                }
                                if !has_tilemap && ui.button("+ Tilemap").clicked() {
                                    self.add_component(entity, ComponentKind::Tilemap);
                                }
                            });
                        } else {
                            ui.label("The entity has every component.");
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
                                self.import_assets();
                            }
                            if ui.button("Refresh").clicked() {
                                if let Some(dir) =
                                    self.asset_browser.root().map(|p| p.to_path_buf())
                                {
                                    self.asset_browser.scan(&dir);
                                }
                            }
                        });
                    });

                    ui.add_space(8.0);

                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(35, 35, 40))
                        .rounding(4.0)
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            let entries = self.asset_browser.entries().to_vec();
                            if entries.is_empty() {
                                ui.centered_and_justified(|ui| {
                                    ui.label("No assets found");
                                    ui.add_space(8.0);
                                    ui.label("Open a project to browse its files");
                                });
                            } else {
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    for entry in &entries {
                                        let name = entry
                                            .path
                                            .rsplit('/')
                                            .next()
                                            .unwrap_or(&entry.path)
                                            .to_string();
                                        let selected = self.asset_browser.preview_source()
                                            == Some(&entry.path);

                                        let response = ui
                                            .selectable_label(selected, &name)
                                            .on_hover_text(&entry.path);

                                        if response.clicked() && entry.kind == AssetKind::Image {
                                            if let Err(e) = self.asset_browser.select(&entry.path) {
                                                log::warn!("asset '{}': {e}", entry.path);
                                            }
                                        }
                                    }
                                });
                            }
                        });

                    // PNG preview of the selected asset.
                    if let Some(source) = self.asset_browser.preview_source() {
                        ui.add_space(8.0);
                        ui.label(format!("Preview: {source}"));
                        if let Some(image) = self.asset_browser.preview_color_image() {
                            let texture_id = ui.ctx().load_texture(
                                "preview",
                                image,
                                egui::TextureOptions::NEAREST,
                            );
                            let size = self.asset_browser.preview_size().unwrap_or((0, 0));
                            let scale = if size.0 > 0 {
                                96.0 / size.0 as f32
                            } else {
                                1.0
                            };
                            ui.add(egui::Image::new(&texture_id).fit_to_exact_size(egui::vec2(
                                size.0 as f32 * scale,
                                size.1 as f32 * scale,
                            )));
                            if let Some((w, h)) = self.asset_browser.preview_size() {
                                ui.label(format!("{w} x {h} pixels"));
                            }
                        }
                    }

                    // Decode failures are shown in the panel, not only logged.
                    let errors = self.asset_browser.errors().to_vec();
                    if !errors.is_empty() {
                        ui.add_space(8.0);
                        ui.colored_label(
                            egui::Color32::LIGHT_RED,
                            format!("{} asset(s) failed to load", errors.len()),
                        );
                        for (path, message) in &errors {
                            ui.label(format!("{path}: {message}")).on_hover_text(path);
                        }
                    }
                });
            }
            Tab::Console => {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Console");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Clear").clicked() {
                                // The console starts over, so the log only
                                // shows what happened since it was cleared.
                                self.console_log.clear();
                            }
                        });
                    });

                    ui.add_space(8.0);

                    // Console log
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(35, 35, 40))
                        .rounding(4.0)
                        .inner_margin(16.0)
                        .show(ui, |ui| {
                            if self.console_log.is_empty() {
                                ui.label("[INFO] System initialized");
                                ui.label("[INFO] Editor ready");
                                ui.label("[INFO] WGPU backend initialized");
                            } else {
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    for line in self.console_log.iter() {
                                        ui.label(line.as_str());
                                    }
                                });
                            }
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

                    // GPU profiling (M28)
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(35, 35, 40))
                        .rounding(4.0)
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.strong("GPU Pass Timings");
                            ui.add_space(8.0);
                            if !self.profiler_supported {
                                ui.label("TIMESTAMP_QUERY not supported on this adapter.");
                            } else if self.pass_timings.is_empty() {
                                ui.label("Waiting for first resolved frame…");
                            } else {
                                for t in self.pass_timings {
                                    ui.label(format!(
                                        "  {}: {:.2} µs ({:.3} ms)",
                                        t.name,
                                        t.duration_us(),
                                        t.duration_ms()
                                    ));
                                }
                            }
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
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.button("Refresh").clicked() {
                                            // Refresh hierarchy
                                        }
                                    },
                                );
                            });

                            ui.add_space(4.0);

                            egui::ScrollArea::vertical()
                                .max_height(ui.available_height())
                                .show(ui, |ui| {
                                    let root_ids: Vec<WidgetId> =
                                        self.ui_designer.root_nodes.clone();
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

                            let desired_size =
                                egui::vec2(ui.available_width(), ui.available_height());
                            let (rect, _response) =
                                ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());

                            // Draw canvas background
                            ui.painter().rect_filled(
                                rect,
                                0.0,
                                egui::Color32::from_rgb(18, 18, 20),
                            );

                            // Draw grid
                            let grid_size = 50.0;
                            let painter = ui.painter();
                            for x in
                                (rect.min.x as i32..rect.max.x as i32).step_by(grid_size as usize)
                            {
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
                            for y in
                                (rect.min.y as i32..rect.max.y as i32).step_by(grid_size as usize)
                            {
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
                            self.create_prefab_from_selection();
                        }

                        ui.separator();

                        if ui.button("Instantiate Prefab").clicked() {
                            self.instantiate_selected_prefab();
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!("Prefabs: {}", self.prefab_editor.prefab_list().len()));
                        });
                    });

                    ui.add_space(8.0);

                    ui.add_space(8.0);

                    // Render the prefab editor panel (list + override editor).
                    // Returns true if overrides were applied, in which case we
                    // update all ECS instances to match the edited prefab.
                    if self.prefab_editor.render(ui) {
                        let prefabs: Vec<_> = self
                            .prefab_editor
                            .state
                            .prefab_list()
                            .iter()
                            .cloned()
                            .collect();
                        for prefab in prefabs {
                            let updated =
                                crate::editing::update_prefab_instances(&mut self.world, &prefab);
                            if updated > 0 {
                                self.log(&format!(
                                    "[INFO] Updated {} instances of prefab '{}'",
                                    updated, prefab.name
                                ));
                            }
                        }
                    }
                };

            }
            Tab::SceneEditor => {
                ui.vertical(|ui| {
                    ui.heading("Scene Editor");
                    ui.add_space(8.0);

                    // Toolbar
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);

                        if ui.button("Create New Scene").clicked() {
                            self.create_scene();
                        }

                        ui.separator();

                        if ui.button("Add Nested Scene").clicked() {
                            self.nest_scene();
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
                            let dialogue =
                                Dialogue::new("new_dialogue".to_string(), "start".to_string());
                            self.gameplay_editor.add_dialogue(dialogue);
                        }
                        if ui.button("New Quest").clicked() {
                            let quest = Quest::new(
                                "new_quest".to_string(),
                                "quest_name".to_string(),
                                "quest_desc".to_string(),
                            );
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
                            for id in self.gameplay_editor.dialogues.keys() {
                                let is_selected =
                                    self.gameplay_editor.selected_dialogue.as_ref() == Some(id);
                                if ui.selectable_label(is_selected, id).clicked() {
                                    self.gameplay_editor.selected_dialogue = Some(id.clone());
                                }
                            }

                            ui.add_space(8.0);
                            ui.label("Quests");
                            for id in self.gameplay_editor.quests.keys() {
                                let is_selected =
                                    self.gameplay_editor.selected_quest.as_ref() == Some(id);
                                if ui.selectable_label(is_selected, id).clicked() {
                                    self.gameplay_editor.selected_quest = Some(id.clone());
                                }
                            }

                            ui.add_space(8.0);
                            ui.label("Data Tables");
                            for id in self.gameplay_editor.data_tables.keys() {
                                let is_selected =
                                    self.gameplay_editor.selected_table.as_ref() == Some(id);
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
                                ui.label(
                                    "Select a dialogue, quest, or data table to view details.",
                                );
                            }
                        });
                    });
                });
            }
            Tab::TilemapEditor => {
                ui.vertical(|ui| {
                    if let Some(project) = self.current_project.as_mut() {
                        crate::tilemap_editor::show(ui, self.tilemap_editor, project);
                    } else {
                        ui.heading("Tilemap Editor");
                        ui.add_space(8.0);
                        ui.label("Open a project to author tilemaps.");
                    }
                });
            }
            Tab::GraphEditor => {
                ui.vertical(|ui| {
                    crate::graph_editor::render_graph_editor(ui, &mut self.graph_editor);
                });
            }
            Tab::AnimationEditor => {
                ui.vertical(|ui| {
                    if let Some(project) = self.current_project.as_mut() {
                        crate::anim_editor::render_animation_editor(
                            ui,
                            &mut self.anim_editor,
                            project,
                            1.0 / 60.0, // dt
                        );
                    } else {
                        ui.heading("Animation Editor");
                        ui.add_space(8.0);
                        ui.label("Open a project to author animations.");
                    }
                });
            }
            Tab::BuildPanel => {
                ui.vertical(|ui| {
                    if let Some(project) = self.current_project.as_mut() {
                        crate::build_panel::BuildPanel::render(
                            &mut self.build_panel,
                            ui,
                            project,
                        );
                    } else {
                        ui.heading("Build / Export");
                        ui.add_space(8.0);
                        ui.label("Open a project to build and export.");
                    }
                });
            }
            Tab::ProjectHub => {
                ui.vertical(|ui| {
                    crate::project_hub::render_project_hub(
                        ui,
                        &mut self.project_hub,
                        &mut |path| {
                            // Open project callback
                            if let Err(e) = self.load_project(path.clone()) {
                                eprintln!("Failed to load project: {}", e);
                            } else {
                                // Add to recent projects
                                self.project_hub.add_recent(path, self.current_project.as_ref().map(|p| p.name.clone()).unwrap_or_else(|| "Unknown".to_string()));
                            }
                        },
                        &mut |name, path, template| {
                            // Create project callback
                            if let Err(e) = self.create_project(name, path, template) {
                                eprintln!("Failed to create project: {}", e);
                            }
                        },
                    );
                });
            }
            Tab::ProfilerPanel => {
                ui.vertical(|ui| {
                    if let Some(gpu) = self.gpu.as_ref() {
                        self.profiler_panel.update(&gpu.renderer.profiler());
                        self.profiler_panel.render(ui, &gpu.renderer.profiler());
                    } else {
                        ui.heading("Profiler");
                        ui.add_space(8.0);
                        ui.label("GPU not initialized. Open a project to view profiler data.");
                    }
                });
            }
            Tab::DebuggerPanel => {
                ui.vertical(|ui| {
                    if let Some(gpu) = self.gpu.as_ref() {
                        // Debugger would need access to the SliceSession/VM
                        // For now, show placeholder
                        ui.heading("KilnScript Debugger");
                        ui.add_space(8.0);
                        ui.label("Debugger panel - requires SliceSession integration");
                        ui.label("Breakpoints, stepping, and variable inspection coming soon");
                    } else {
                        ui.heading("Debugger");
                        ui.add_space(8.0);
                        ui.label("GPU not initialized. Open a project to use debugger.");
                    }
                });
            }
            Tab::VfxEditor => {
                ui.vertical(|ui| {
                    self.vfx_editor.render(ui);
                });
            }
            Tab::AudioEditor => {
                ui.vertical(|ui| {
                    self.audio_editor.render(ui);
                });
            }
            Tab::PhysicsEditor => {
                ui.vertical(|ui| {
                    self.physics_editor.render(ui);
                });
            }
            Tab::UiLayoutEditor => {
                ui.vertical(|ui| {
                    self.ui_layout_editor.render(ui);
                });
            }
            Tab::Preferences => {
                ui.vertical(|ui| {
                    self.preferences.render(ui);
                });
            }
            Tab::PluginManagerPanel => {
                ui.vertical(|ui| {
                    self.plugin_manager.render(ui, context);
                });
            }
            Tab::VersionControl => {
                ui.vertical(|ui| {
                    self.version_control.render(ui);
                });
            }
            Tab::Hierarchy => {
                ui.vertical(|ui| {
                    if let Some(project) = self.current_project.as_mut() {
                        let scene_id = project
                            .scenes
                            .keys()
                            .next()
                            .map(|s| s.clone())
                            .unwrap_or_default();
                        // Render hierarchy panel and handle actions.
                        if let Some(action) = crate::hierarchy::render_hierarchy_panel(
                            ui,
                            &mut self.hierarchy_panel,
                            project,
                            &scene_id,
                        ) {
                            match action {
                                crate::hierarchy::HierarchyAction::SelectEntity(project_id) => {
                                    // Sync selection: project entity ID -> hecs Entity
                                    if let Some(&hecs_entity) = self.entity_id_to_hecs.get(&project_id) {
                                        self.selected_entity = Some(hecs_entity);
                                    }
                                }
                                crate::hierarchy::HierarchyAction::AddEntity(name) => {
                                    self.record_scene();
                                    // Use all known project entity IDs for unique naming.
                                    let existing: Vec<String> = self.entity_id_to_hecs.keys().cloned().collect();
                                    let size = self.world_size();
                                    let position = [size.0 / 2.0, size.1 / 2.0];
                                    let (source_id, entity) = crate::editing::add_entity(
                                        self.world,
                                        &existing,
                                        position,
                                        crate::editing::EntityDefaults::default(),
                                    );
                                    // Update the project with the new entity
                                    if let Some(scene) = project.scenes.get_mut(&scene_id) {
                                        scene.entities.push(kiln_core::project::Entity {
                                            id: source_id.clone(),
                                            name: name,
                                            components: vec![
                                                kiln_core::project::Component::Transform {
                                                    x: position[0],
                                                    y: position[1],
                                                    rotation: 0.0,
                                                    scale: 1.0,
                                                },
                                                kiln_core::project::Component::Sprite {
                                                    color: [1.0, 1.0, 1.0, 1.0],
                                                    texture: None,
                                                },
                                                kiln_core::project::Component::Role {
                                                    role: "".to_string(),
                                                },
                                            ],
                                        });
                                    }
                                    *self.selected_entity = Some(entity);
                                    self.entity_id_to_hecs.insert(source_id, entity);
                                    self.hierarchy_panel.rebuild(project, &scene_id);
                                }
                                crate::hierarchy::HierarchyAction::RemoveEntity(project_id) => {
                                    if let Some(&hecs_entity) = self.entity_id_to_hecs.get(&project_id) {
                                        self.record_scene();
                                        if crate::editing::remove_entity(self.world, hecs_entity) {
                                            *self.selected_entity = None;
                                        }
                                        // Remove from project
                                        if let Some(scene) = project.scenes.get_mut(&scene_id) {
                                            scene.entities.retain(|e| e.id != project_id);
                                        }
                                        self.entity_id_to_hecs.remove(&project_id);
                                        self.entity_textures.remove(&project_id);
                                        self.entity_roles.remove(&project_id);
                                        self.hierarchy_panel.rebuild(project, &scene_id);
                                    }
                                }
                            }
                        }
                    } else {
                        ui.heading("Hierarchy");
                        ui.add_space(8.0);
                        ui.label("Open a project to view entities.");
                    }
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
            Tab::TilemapEditor => "Tilemap Editor".into(),
            Tab::Hierarchy => "Hierarchy".into(),
            Tab::AnimationEditor => "Animation Editor".into(),
            Tab::BuildPanel => "Build / Export".into(),
            Tab::ProjectHub => "Project Hub".into(),
            Tab::ProfilerPanel => "Profiler".into(),
            Tab::DebuggerPanel => "Debugger".into(),
            Tab::VfxEditor => "VFX Editor".into(),
            Tab::AudioEditor => "Audio Editor".into(),
            Tab::PhysicsEditor => "Physics Editor".into(),
            Tab::UiLayoutEditor => "UI Layout Editor".into(),
            Tab::Preferences => "Preferences".into(),
            Tab::PluginManagerPanel => "Plugin Manager".into(),
            Tab::VersionControl => "Version Control".into(),
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
            let screen_pos = egui::pos2(canvas_rect.min.x + bounds.x, canvas_rect.min.y + bounds.y);
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
    

    /// Generate a unique scene ID.
    fn next_scene_id(&self) -> String {
        let mut index = 1;
        loop {
            let candidate = format!("scene_{}", index);
            if !self.scene_tabs.iter().any(|t| t.scene_id == candidate) {
                return candidate;
            }
            index += 1;
        }
    }
    
    /// Add a new scene tab.
    fn add_scene_tab(&mut self, scene_id: String) {
        self.scene_tabs.push(SceneTab {
            scene_id,
            is_active: false,
            dirty: false,
        });
    }
    
    /// Switch to a scene tab.
    fn switch_scene_tab(&mut self, index: usize) {
        if index < self.scene_tabs.len() {
            for (i, tab) in self.scene_tabs.iter_mut().enumerate() {
                tab.is_active = i == index;
            }
            self.active_scene_tab = Some(index);
            // TODO: Load the scene data for this tab
        }
    }
    
    /// Close a scene tab.
    fn close_scene_tab(&mut self, index: usize) {
        if index < self.scene_tabs.len() {
            // TODO: Check for unsaved changes
            self.scene_tabs.remove(index);
            if let Some(active) = self.active_scene_tab {
                if active >= index && active > 0 {
                    self.active_scene_tab = Some(active - 1);
                } else if active >= index {
                    self.active_scene_tab = self.scene_tabs.first().map(|_| 0);
                }
            } else if !self.scene_tabs.is_empty() {
                self.active_scene_tab = Some(0);
            } else {
                self.active_scene_tab = None;
            }
        }
    }
}

}
}
