//! Main editor application.

use hecs;
use kiln_core::{
    project::{Project, Scene},
    Transform,
};
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
    event::{ElementState, KeyEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, NamedKey},
    window::Window,
};

use crate::anim_editor::AnimationEditorState;
use crate::assets::{AssetBrowser, AssetKind};
use crate::audio_editor::AudioEditorState;
use crate::build_panel::BuildPanel;
use crate::component_command::{ComponentCommand, ComponentEdit, ComponentKind};
use crate::debugger_panel::DebuggerPanel;
use crate::hierarchy::HierarchyPanel;
use crate::input_rebind_panel::InputRebindPanel;
use crate::physics_editor::PhysicsEditorState;
use crate::plugin_manager_panel::PluginManagerPanel;
use crate::prefab_editor_panel::PrefabEditorPanel;
use crate::preferences::PreferencesPanel;
use crate::profiler_panel::ProfilerPanel;
use crate::project_hub::ProjectHubState;
use crate::project_lifecycle::{ProjectAction, ProjectLifecycle, SaveDecision};
use crate::property_edit::SetSpriteVisual;
use crate::scene_view::{self, SceneTextures, ViewportRect};
use crate::scene_workspace::SceneWorkspace;
use crate::shader_editor::ShaderEditorState;
use crate::sprite_editor::SpriteEditorState;
use crate::tilemap_editor::TilemapEditorState;
use crate::transform_edit::{SceneTransaction, SetTransform};
use crate::transform_gizmo::{hits_handle, GizmoDrag, GizmoMode};
use crate::ui_layout_editor::UiLayoutEditorState;
use crate::version_control_panel::VersionControlPanel;
use crate::vfx_editor::VfxEditorState;

#[derive(Debug, Clone)]
enum SceneAction {
    Open(String),
    Create,
    Switch(usize),
    Close(usize),
}

#[derive(Debug, Clone)]
struct SceneClosePrompt {
    index: usize,
    save_error: Option<String>,
}

struct RecoveryPrompt {
    project_path: PathBuf,
    project: Project,
}

enum ProjectRequest {
    Component(ComponentCommand),
    Entity(crate::entity_command::EntityCommand),
    CancelTransform,
    Leave(ProjectAction),
    Resolve(SaveDecision),
    ResolveSceneClose(SaveDecision),
    ResolveRecovery(bool),
    Scene(SceneAction),
    Open(PathBuf),
    SaveAs(PathBuf),
    CreateDialog,
    Create(String, PathBuf, String),
}

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

impl PlaySession {
    /// Play an audio preview by name.
    #[cfg(feature = "audio")]
    pub fn play_audio_preview(&mut self, name: &str) {
        self.slice.host_mut().play_audio_preview(name);
    }

    /// Stop audio playback preview.
    #[cfg(feature = "audio")]
    pub fn stop_audio_preview(&mut self) {
        self.slice.host_mut().stop_audio_preview();
    }

    /// Register an audio asset with the slice session.
    #[cfg(feature = "audio")]
    pub fn register_audio(&mut self, name: &str, path: &str) {
        self.slice.host_mut().register_audio(name, path);
    }
}

/// The world size the scene view falls back to when no project is open.
const DEFAULT_WORLD_SIZE: (f32, f32) = (1280.0, 720.0);

/// The spacing of the scene view's grid, in world units.
const GRID_SPACING: f32 = 50.0;

/// The colour of the scene view's grid lines.
const GRID_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 0.06];

/// The colour behind the scene view.
const BACKGROUND_COLOR: [f32; 4] = [0.07, 0.07, 0.08, 1.0];

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
    Exit,
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
    rename_input: Option<(String, String)>,
    role_input: Option<(String, String)>,
    sprite_reference_input: Option<(String, SpriteReferenceDraft)>,
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
    scenes: SceneWorkspace,
    ui_designer: UIDesignerState,
    gameplay_editor: GameplayEditorState,
    tilemap_editor: TilemapEditorState,
    hierarchy_panel: HierarchyPanel,
    anim_editor: AnimationEditorState,
    build_panel: BuildPanel,
    project_hub: ProjectHubState,
    profiler_panel: ProfilerPanel,
    shader_editor: ShaderEditorState,
    debugger_panel: DebuggerPanel,
    sprite_editor: SpriteEditorState,
    vfx_editor: VfxEditorState,
    audio_editor: AudioEditorState,
    physics_editor: PhysicsEditorState,
    ui_layout_editor: UiLayoutEditorState,
    preferences: PreferencesPanel,
    prefab_editor: PrefabEditorPanel,
    version_control: VersionControlPanel,
    plugin_manager: PluginManagerPanel,
    input_rebind: InputRebindPanel,
    /// Mapping from project entity ID (source_id) to hecs Entity for selection sync.
    entity_id_to_hecs: HashMap<String, hecs::Entity>,
    project_path: Option<PathBuf>,
    project_requests: Vec<ProjectRequest>,
    project_error: Option<String>,
    lifecycle: ProjectLifecycle,
    exit_requested: bool,
    show_save_as: bool,
    save_as_path: String,
    current_project: Option<Project>,
    asset_browser: AssetBrowser,
    /// Filter text for the asset browser.
    asset_filter: String,
    /// Transform gizmo state for visual entity manipulation.
    gizmo: GizmoState,
    transform_edit: Option<SceneTransaction>,
    /// Set by the welcome screen, handled after the UI pass.
    open_project_requested: bool,
    /// Set by the scene toolbar, handled after the UI pass.
    play_toggle_requested: bool,
    /// Set by the scene toolbar, handled after the UI pass.
    scene_save_requested: bool,
    /// Set by the scene toolbar, handled after the UI pass.
    scene_load_requested: bool,
    scene_close_prompt: Option<SceneClosePrompt>,
    pending_recovery: Option<RecoveryPrompt>,
    last_autosave: Instant,
    /// Camera state for the scene view.
    camera: CameraState,
    /// Grid snap settings for the scene editor.
    grid_snap_size: f32,
    /// Whether grid snapping is enabled.
    grid_snap_enabled: bool,
}

#[derive(Clone, Default)]
struct SpriteReferenceDraft {
    texture: String,
    material: String,
    animation_clip: String,
}

/// Camera state for the scene editor viewport.
#[derive(Debug, Clone)]
pub struct CameraState {
    /// Entity to follow (if any).
    target: Option<String>,
    /// Camera position in world space.
    position: [f32; 2],
    /// Camera zoom level.
    zoom: f32,
    /// Camera bounds (min_x, min_y, max_x, max_y). None means no bounds.
    bounds: Option<[f32; 4]>,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            target: None,
            position: [0.0, 0.0],
            zoom: 1.0,
            bounds: None,
        }
    }
}

/// Transform gizmo state for dragging entities in the scene view.
#[derive(Debug, Clone, Default)]
struct GizmoState {
    mode: GizmoMode,
    drag: Option<GizmoDragSession>,
}

#[derive(Debug, Clone)]
struct GizmoDragSession {
    source: String,
    drag: GizmoDrag,
    selected: Vec<(String, Transform)>,
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
    ShaderEditor,
    UiLayoutEditor,
    Preferences,
    PluginManagerPanel,
    PrefabEditorPanel,
    VersionControl,
    InputRebind,
    SpriteEditor,
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

    let slice = kiln_runtime::SliceSession::with_action_map(&source, project.action_map.clone())
        .map_err(|d| {
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
    crate::scene_document::snapshot(world, entity_textures, entity_roles, project)
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
            rename_input: None,
            role_input: None,
            sprite_reference_input: None,
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
            scenes: SceneWorkspace::default(),
            ui_designer: UIDesignerState::new(),
            gameplay_editor: GameplayEditorState::new(),
            tilemap_editor: TilemapEditorState::new(),
            hierarchy_panel: HierarchyPanel::new(),
            anim_editor: AnimationEditorState::new(),
            build_panel: BuildPanel::new(),
            project_hub: ProjectHubState::new(),
            profiler_panel: ProfilerPanel::new(),
            shader_editor: ShaderEditorState::new(),
            debugger_panel: DebuggerPanel::new(),
            sprite_editor: SpriteEditorState::new(),
            vfx_editor: VfxEditorState::new(),
            audio_editor: AudioEditorState::new(),
            physics_editor: PhysicsEditorState::new(),
            ui_layout_editor: UiLayoutEditorState::new(),
            preferences: PreferencesPanel::new(),
            prefab_editor: PrefabEditorPanel::new(),
            version_control: VersionControlPanel::new(),
            plugin_manager: PluginManagerPanel::new(),
            input_rebind: InputRebindPanel::new(),
            entity_id_to_hecs: HashMap::new(),
            project_path: None,
            project_requests: Vec::new(),
            project_error: None,
            lifecycle: ProjectLifecycle::default(),
            exit_requested: false,
            show_save_as: false,
            save_as_path: String::new(),
            current_project: None,
            asset_browser: AssetBrowser::new(),
            asset_filter: String::new(),
            camera: CameraState::default(),
            grid_snap_size: 32.0,
            grid_snap_enabled: false,
            open_project_requested: false,
            play_toggle_requested: false,
            scene_save_requested: false,
            scene_load_requested: false,
            scene_close_prompt: None,
            pending_recovery: None,
            last_autosave: Instant::now(),
            gizmo: GizmoState::default(),
            transform_edit: None,
        })
    }

    async fn init_gpu(&mut self, window: Arc<Window>) {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::util::backend_bits_from_env().unwrap_or(wgpu::Backends::all()),
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
        let (scene_view_post_framebuffer, scene_view_post_pipeline) = if renderer.tier()
            != RenderTier::Low
        {
            let _config = PostConfig::for_tier(renderer.tier());
            let fb =
                PostFramebuffer::new(renderer.device(), size.width, size.height, renderer.tier());
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
        self.finish_transform_edit();
        let path = self
            .project_path
            .clone()
            .ok_or("No project file open. Use Save As first.")?;
        let project = self.project_to_save()?;
        let path = crate::project_files::save(&project, &path)?;
        let _ = crate::recovery::clear(&path);
        self.project_path = Some(path);
        self.scenes.mark_saved(&project);
        self.lifecycle.mark_saved(&project);
        self.current_project = Some(project);
        self.project_error = None;
        Ok(())
    }

    fn project_to_save(&self) -> Result<Project, String> {
        if self.play.is_some() {
            return Err("Stop play mode before saving scene edits".into());
        }
        let mut project = self.current_project.clone().ok_or("No project loaded")?;
        if self.scenes.active_id().is_some() {
            self.scenes.capture(&mut project, self.scene_from_world())?;
        }
        Ok(project)
    }

    /// Save another manifest in the same asset directory, committing state only on success.
    pub fn save_project_as(&mut self, path: PathBuf) -> Result<(), String> {
        self.finish_transform_edit();
        let source = self
            .project_path
            .as_ref()
            .ok_or("Open or create a project before Save As")?;
        let previous_path = source.clone();
        let project = self.project_to_save()?;
        let path = crate::project_files::save_as(&project, source, &path)?;
        let _ = crate::recovery::clear(&previous_path);
        let _ = crate::recovery::clear(&path);
        self.project_path = Some(path.clone());
        self.project_path_input = path.to_string_lossy().into_owned();
        self.scenes.mark_saved(&project);
        self.lifecycle.mark_saved(&project);
        self.current_project = Some(project);
        self.project_error = None;
        Ok(())
    }

    /// Load a project from a file path.
    pub fn load_project(&mut self, path: PathBuf) -> Result<(), String> {
        let loaded = crate::project_files::load(&path)?;
        self.finish_transform_edit();
        if self.play.is_some() {
            self.stop_play();
        }
        self.is_playing = false;
        let project = loaded.project;
        let path = loaded.path;
        let recovery = match crate::recovery::load_if_newer(&path) {
            Ok(recovery) => recovery.map(|(_, project)| project),
            Err(error) => {
                self.console_log.push(format!(
                    "[WARN] Ignoring unreadable recovery snapshot: {error}"
                ));
                None
            }
        };
        self.scenes = SceneWorkspace::open(&project);
        self.lifecycle.mark_saved(&project);
        self.selected_entity = None;
        self.project_path_input = path.to_string_lossy().into_owned();
        self.show_welcome = false;
        self.project_error = None;
        self.project_hub
            .add_recent(path.clone(), project.name.clone());
        self.project_path = Some(path.clone());
        self.current_project = Some(project);
        self.pending_recovery = recovery.map(|project| RecoveryPrompt {
            project_path: path.clone(),
            project,
        });
        self.last_autosave = Instant::now();

        // Scan the project directory so the asset browser lists its files.
        if let Some(dir) = path.parent() {
            self.project_root = Some(dir.to_path_buf());
            self.asset_browser.scan(dir);
        }

        // Upload the project's textures to the GPU, so the scene view can
        // draw the same art the player draws.
        self.upload_scene_textures();

        // Clear transient selection and project the active scene.
        self.reset_scene_view();

        Ok(())
    }

    /// Create a new project from a template.
    pub fn create_project(
        &mut self,
        name: &str,
        path: &Path,
        template: &str,
    ) -> Result<(), String> {
        let loaded = crate::project_files::create(name, path, template)?;
        self.load_project(loaded.path)
    }

    fn finish_transform_edit(&mut self) {
        if let Some(edit) = self.transform_edit.take() {
            edit.commit(&mut self.scenes.history);
        }
        self.gizmo.drag = None;
    }

    fn cancel_transform_edit(&mut self) {
        if let Some(edit) = self.transform_edit.take() {
            let before = edit.cancel();
            let _ = self.apply_scene(before);
        }
        self.gizmo.drag = None;
    }

    fn authoring_document(&self) -> Result<Option<Project>, String> {
        if self.current_project.is_none() || self.play.is_some() {
            return Ok(self.current_project.clone());
        }
        self.project_to_save().map(Some)
    }

    fn request_project_action(&mut self, action: ProjectAction) -> Result<(), String> {
        if self.lifecycle.pending().is_some() {
            return Ok(());
        }
        if self.play.is_some() {
            self.stop_play();
        }
        self.finish_transform_edit();
        self.gizmo.drag = None;
        let document = self.authoring_document()?;
        if let Some(action) = self.lifecycle.request(action, document.as_ref())? {
            self.execute_project_action(action)?;
        }
        Ok(())
    }

    fn resolve_project_action(&mut self, decision: SaveDecision) -> Result<(), String> {
        if self.lifecycle.pending().is_none() {
            return Ok(());
        }
        let saved = if decision == SaveDecision::Save {
            self.save_project()
        } else {
            Ok(())
        };
        if let Some(action) = self.lifecycle.resolve(decision, saved)? {
            self.execute_project_action(action)?;
        }
        Ok(())
    }

    fn execute_project_action(&mut self, action: ProjectAction) -> Result<(), String> {
        match action {
            ProjectAction::Open(path) => self.load_project(path),
            ProjectAction::Create {
                name,
                path,
                template,
            } => self.create_project(&name, &path, &template),
            ProjectAction::Exit => {
                self.exit_requested = true;
                Ok(())
            }
        }
    }

    fn begin_new_project(&mut self) {
        self.show_welcome = false;
        self.project_hub.show_new_project_dialog = true;
        self.project_hub.new_project_name.clear();
        self.project_hub.new_project_dir =
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        self.project_hub.new_project_template = "empty_2d".into();
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
        self.finish_transform_edit();
        self.capture_active_scene()?;
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
        self.selected_entity = None;
        self.hierarchy_panel.clear_selection();
        self.gizmo = GizmoState::default();
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
        self.reset_scene_view();
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

    fn scene_camera(&self) -> kiln_render::camera::Camera {
        let world = self.world_size();
        let mut camera = scene_view::world_camera(world.0, world.1);
        if let Some(target_id) = &self.camera.target {
            if let Some(&hecs_entity) = self.entity_id_to_hecs.get(target_id) {
                if let Ok(transform) = self
                    .world
                    .world_ref()
                    .get::<&kiln_runtime::GlobalTransform>(hecs_entity)
                {
                    camera.position = transform.position.into();
                }
            }
        }
        camera.position[0] += self.camera.position[0];
        camera.position[1] += self.camera.position[1];
        camera.zoom *= self.camera.zoom;
        // Apply bounds if set
        if let Some([min_x, min_y, max_x, max_y]) = self.camera.bounds {
            let view = camera.view_bounds();
            let mut pos = camera.position;
            if view[0] < min_x {
                pos[0] = min_x;
            }
            if view[1] < min_y {
                pos[1] = min_y;
            }
            if view[2] > max_x {
                pos[0] = max_x - (view[2] - view[0]);
            }
            if view[3] > max_y {
                pos[1] = max_y - (view[3] - view[1]);
            }
            camera.position = pos;
        }
        camera
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
        let Some(scene_id) = self.scenes.active_id().map(str::to_owned) else {
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
        let world_ref = world.world_ref();
        for entity in world_ref.iter() {
            let eid = entity.entity();
            if let Ok(source_id) = world_ref.get::<&SourceId>(eid) {
                self.entity_id_to_hecs.insert(source_id.0.clone(), eid);
            }
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
        crate::scene_document::snapshot_for_scene(
            &self.world,
            &self.entity_textures,
            &self.entity_roles,
            self.current_project.as_ref(),
            self.scenes.active_id(),
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
        self.finish_transform_edit();
        self.require_edit_mode()?;
        let current = self.scene_from_world();
        let Some(previous) = self.scenes.history.undo(current) else {
            return Err("Nothing to undo".to_string());
        };
        self.apply_scene(previous)
    }

    /// Redo the last undone scene edit.
    pub fn redo(&mut self) -> Result<(), String> {
        self.finish_transform_edit();
        self.require_edit_mode()?;
        let current = self.scene_from_world();
        let Some(next) = self.scenes.history.redo(current) else {
            return Err("Nothing to redo".to_string());
        };
        self.apply_scene(next)
    }

    fn apply_component_command(&mut self, command: ComponentCommand) -> Result<(), String> {
        self.require_edit_mode()?;
        self.finish_transform_edit();
        if self.scenes.active_id() != Some(command.scene_id.as_str()) {
            return Err("Component command targets an inactive scene".into());
        }
        let count = self
            .world
            .world_ref()
            .query::<(hecs::Entity, &SourceId)>()
            .iter()
            .filter(|(_, source)| source.0 == command.entity_id)
            .count();
        if count != 1 {
            return Err("Entity identity is missing or ambiguous".into());
        }
        let mut scene = self.scene_from_world();
        let project = self.current_project.as_ref().ok_or("No project loaded")?;
        let refresh_textures = matches!(&command.edit, ComponentEdit::SetSpriteReferences { .. });
        if command.execute(&mut scene, project, &mut self.scenes.history)? {
            if refresh_textures {
                self.apply_scene_with_texture_refresh(scene)?;
            } else {
                self.apply_scene(scene)?;
            }
        }
        Ok(())
    }

    fn apply_entity_command(
        &mut self,
        command: crate::entity_command::EntityCommand,
    ) -> Result<(), String> {
        self.require_edit_mode()?;
        self.finish_transform_edit();
        if self.scenes.active_id() != Some(command.scene_id.as_str()) {
            return Err("Hierarchy command targets an inactive scene".into());
        }
        let mut scene = self.scene_from_world();
        let project = self.current_project.as_ref().ok_or("No project loaded")?;
        let deleted = match &command.edit {
            crate::entity_command::EntityEdit::Delete { entity_id } => Some(entity_id.as_str()),
            _ => None,
        };
        let selected = command.execute(&mut scene, project, &mut self.scenes.history)?;
        self.apply_scene(scene)?;
        if deleted.is_some_and(|id| self.hierarchy_panel.selected_entity_id() == Some(id)) {
            self.hierarchy_panel.clear_selection();
        }
        if let Some(id) = selected {
            self.selected_entity = self.entity_id_to_hecs.get(&id).copied();
            self.hierarchy_panel.select(&id);
        }
        Ok(())
    }

    /// Apply a scene to the project and the editor world.
    fn apply_scene(&mut self, scene: Scene) -> Result<(), String> {
        let selected_source = self.selected_entity.and_then(|entity| {
            self.world
                .world_ref()
                .get::<&SourceId>(entity)
                .ok()
                .map(|id| id.0.clone())
        });
        let scene_id = scene.id.clone();
        if let Some(project) = &mut self.current_project {
            project.scenes.insert(scene_id, scene);
        }
        self.load_project_scene();
        self.selected_entity =
            selected_source.and_then(|id| self.entity_id_to_hecs.get(&id).copied());
        Ok(())
    }

    fn apply_scene_with_texture_refresh(&mut self, scene: Scene) -> Result<(), String> {
        let selected_source = self.selected_entity.and_then(|entity| {
            self.world
                .world_ref()
                .get::<&SourceId>(entity)
                .ok()
                .map(|id| id.0.clone())
        });
        let scene_id = scene.id.clone();
        if let Some(project) = &mut self.current_project {
            project.scenes.insert(scene_id, scene);
        }
        self.upload_scene_textures();
        self.load_project_scene();
        self.selected_entity =
            selected_source.and_then(|id| self.entity_id_to_hecs.get(&id).copied());
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
        self.finish_transform_edit();
        self.require_edit_mode()?;
        self.capture_active_scene()?;
        self.reset_scene_view();
        Ok(())
    }

    fn require_edit_mode(&self) -> Result<(), String> {
        if self.play.is_some() {
            return Err("Stop play mode before changing scene documents".into());
        }
        Ok(())
    }

    fn capture_active_scene(&mut self) -> Result<(), String> {
        let scene = self.scene_from_world();
        let project = self.current_project.as_mut().ok_or("No project loaded")?;
        self.scenes.capture(project, scene)
    }

    fn reset_scene_view(&mut self) {
        self.selected_entity = None;
        self.rename_input = None;
        self.role_input = None;
        self.sprite_reference_input = None;
        self.hierarchy_panel.clear_selection();
        self.gizmo = GizmoState::default();
        self.load_project_scene();
    }

    fn change_scene(&mut self, action: SceneAction) -> Result<(), String> {
        self.finish_transform_edit();
        self.require_edit_mode()?;
        let current = self.scenes.active_id().map(|_| self.scene_from_world());
        if let SceneAction::Close(index) = &action {
            if *index >= self.scenes.tabs.len() {
                return Err("Unknown scene tab".into());
            }
            if self.scenes.tabs.len() == 1 {
                return Err("Keep at least one scene tab open".into());
            }
            if let Some(project) = self.current_project.as_ref() {
                self.scenes.refresh_dirty(project, current.as_ref());
            }
            if self.scenes.tabs[*index].dirty {
                self.scene_close_prompt = Some(SceneClosePrompt {
                    index: *index,
                    save_error: None,
                });
                return Ok(());
            }
        }
        let project = self.current_project.as_mut().ok_or("No project loaded")?;
        match action {
            SceneAction::Open(id) => {
                self.scenes
                    .open_scene(project, current.ok_or("No active scene")?, &id)?
            }
            SceneAction::Create => self.scenes.create(project, current)?,
            SceneAction::Switch(index) => {
                self.scenes
                    .switch(project, current.ok_or("No active scene")?, index)?
            }
            SceneAction::Close(index) => {
                self.scenes
                    .close(project, current.ok_or("No active scene")?, index)?
            }
        }
        self.reset_scene_view();
        Ok(())
    }

    fn resolve_scene_close(&mut self, decision: SaveDecision) -> Result<(), String> {
        let Some(prompt) = self.scene_close_prompt.clone() else {
            return Ok(());
        };
        match decision {
            SaveDecision::Cancel => {
                self.scene_close_prompt = None;
                Ok(())
            }
            SaveDecision::Save => {
                if let Err(error) = self.save_project() {
                    if let Some(prompt) = self.scene_close_prompt.as_mut() {
                        prompt.save_error = Some(error);
                    }
                    return Ok(());
                }
                self.scene_close_prompt = None;
                self.change_scene(SceneAction::Close(prompt.index))
            }
            SaveDecision::Discard => {
                self.finish_transform_edit();
                self.require_edit_mode()?;
                let current = self.scene_from_world();
                let project = self.current_project.as_mut().ok_or("No project loaded")?;
                self.scenes
                    .discard_and_close(project, current, prompt.index)?;
                self.scene_close_prompt = None;
                self.reset_scene_view();
                Ok(())
            }
        }
    }

    fn resolve_recovery(&mut self, restore: bool) -> Result<(), String> {
        let Some(prompt) = self.pending_recovery.as_ref() else {
            return Ok(());
        };
        if !restore {
            crate::recovery::clear(&prompt.project_path)?;
            self.pending_recovery = None;
            self.last_autosave = Instant::now();
            return Ok(());
        }
        let Some(prompt) = self.pending_recovery.take() else {
            return Ok(());
        };
        {
            let saved_project = self.current_project.clone().ok_or("No project loaded")?;
            self.current_project = Some(prompt.project.clone());
            self.scenes = SceneWorkspace::open(&prompt.project);
            self.scenes.set_saved_baselines(&saved_project);
            self.lifecycle.mark_saved(&saved_project);
            self.reset_scene_view();
            self.console_log
                .push("[INFO] Recovered unsaved project changes".into());
        }
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
            if let Some(last_project) = self.project_hub.last_project().cloned() {
                let path = last_project.path.clone();
                if path.exists() {
                    if let Err(e) = self.load_project(path.to_path_buf()) {
                        eprintln!("Failed to auto-reopen last project: {}", e);
                    } else {
                        // Add back to recent projects since we just opened it
                        self.project_hub
                            .add_recent(last_project.path.clone(), last_project.name.clone());
                    }
                }
            }
        }
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        use winit::event::WindowEvent;

        let gpu = self.gpu.as_mut().unwrap();
        let response = self.egui_winit.on_window_event(&gpu.window, &event);
        // egui still receives prompt input; raw gizmo input cannot mutate behind it.
        if self.lifecycle.pending().is_some()
            && matches!(
                event,
                WindowEvent::MouseInput { .. }
                    | WindowEvent::CursorMoved { .. }
                    | WindowEvent::KeyboardInput { .. }
            )
        {
            return;
        }

        #[allow(clippy::collapsible_match)]
        match event {
            WindowEvent::CloseRequested => {
                self.project_requests
                    .push(ProjectRequest::Leave(ProjectAction::Exit));
            }
            WindowEvent::RedrawRequested => self.render_frame(),
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    gpu.renderer.resize(&gpu.surface, size.width, size.height);
                    // Also resize the scene view post framebuffer if it exists.
                    if let Some(fb) = gpu.scene_view_post_framebuffer.as_mut() {
                        fb.resize(
                            gpu.renderer.device(),
                            size.width,
                            size.height,
                            gpu.renderer.tier(),
                        );
                    }
                }
            }
            WindowEvent::Focused(false) => self.finish_transform_edit(),
            _ => {}
        }

        // Gizmo mode keyboard shortcuts (W/E/R like Unity)
        if let WindowEvent::KeyboardInput {
            event:
                KeyEvent {
                    logical_key: key,
                    state: ElementState::Pressed,
                    ..
                },
            ..
        } = event
        {
            if !response.consumed {
                match key {
                    Key::Character(key) if ["w", "e", "r"].contains(&key.as_str()) => {
                        self.finish_transform_edit();
                        self.gizmo.mode = match key.as_str() {
                            "w" => GizmoMode::Translate,
                            "e" => GizmoMode::Rotate,
                            _ => GizmoMode::Scale,
                        };
                    }
                    Key::Named(NamedKey::Escape) if self.transform_edit.is_none() => {
                        self.gizmo.mode = GizmoMode::None
                    }
                    _ => {}
                }
            }
        }

        // Early return if egui handled the event
        #[allow(clippy::needless_return)]
        if response.consumed {
            return;
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.exit_requested {
            event_loop.exit();
            return;
        }
        if let Some(gpu) = &self.gpu {
            gpu.window.request_redraw();
        }
    }
}

impl EditorApp {
    fn render_frame(&mut self) {
        if !self
            .project_requests
            .iter()
            .any(|request| matches!(request, ProjectRequest::CancelTransform))
            && self.gizmo.drag.is_none()
            && !self.egui_winit.egui_ctx().wants_keyboard_input()
            && !self
                .egui_winit
                .egui_ctx()
                .input(|input| input.pointer.primary_down())
        {
            self.finish_transform_edit();
        }
        for request in std::mem::take(&mut self.project_requests) {
            if self.lifecycle.pending().is_some() && !matches!(request, ProjectRequest::Resolve(_))
            {
                continue;
            }
            if self.scene_close_prompt.is_some()
                && !matches!(request, ProjectRequest::ResolveSceneClose(_))
            {
                continue;
            }
            if self.pending_recovery.is_some()
                && !matches!(request, ProjectRequest::ResolveRecovery(_))
            {
                continue;
            }
            let result = match request {
                ProjectRequest::Component(command) => self.apply_component_command(command),
                ProjectRequest::Entity(command) => self.apply_entity_command(command),
                ProjectRequest::CancelTransform => {
                    self.cancel_transform_edit();
                    Ok(())
                }
                ProjectRequest::Leave(action) => self.request_project_action(action),
                ProjectRequest::Resolve(decision) => self.resolve_project_action(decision),
                ProjectRequest::ResolveSceneClose(decision) => self.resolve_scene_close(decision),
                ProjectRequest::ResolveRecovery(restore) => self.resolve_recovery(restore),
                ProjectRequest::Scene(action) => self.change_scene(action),
                ProjectRequest::Open(path) => {
                    self.request_project_action(ProjectAction::Open(path))
                }
                ProjectRequest::SaveAs(path) => self.save_project_as(path),
                ProjectRequest::CreateDialog => {
                    self.begin_new_project();
                    Ok(())
                }
                ProjectRequest::Create(name, path, template) => {
                    self.request_project_action(ProjectAction::Create {
                        name,
                        path,
                        template,
                    })
                }
            };
            if let Err(error) = result {
                self.console_log.push(format!("[ERROR] {error}"));
                if self.lifecycle.pending().is_none() {
                    self.project_error = Some(error);
                }
            }
            if self.lifecycle.pending().is_some() || self.exit_requested {
                break;
            }
        }

        if self.play.is_none() {
            let current = self.scenes.active_id().map(|_| self.scene_from_world());
            if let Some(project) = &self.current_project {
                self.scenes.refresh_dirty(project, current.as_ref());
            }
        }

        let project_blocked = self.lifecycle.pending().is_some();
        let project_dirty = self
            .authoring_document()
            .and_then(|document| self.lifecycle.is_dirty(document.as_ref()))
            .unwrap_or(true);
        if self.pending_recovery.is_none() {
            if let Some(path) = self.project_path.clone() {
                if self.play.is_none() && project_dirty {
                    if self.last_autosave.elapsed() >= std::time::Duration::from_secs(30) {
                        if let Ok(Some(project)) = self.authoring_document() {
                            if let Err(error) = crate::recovery::save(&path, &project) {
                                self.console_log.push(format!(
                                    "[WARN] Could not write recovery snapshot: {error}"
                                ));
                            }
                        }
                        self.last_autosave = Instant::now();
                    }
                } else if !project_dirty {
                    let _ = crate::recovery::clear(&path);
                }
            }
        }

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
        let scene_camera = self.scene_camera();
        self.scene_view_rect = None;

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
            if project_blocked {
                ui.disable();
            }
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
                        menu_actions.push(MenuAction::Exit);
                        ui.close_menu();
                    }
                });

                ui.menu_button("Edit", |ui| {
                    if ui
                        .add_enabled(self.scenes.history.can_undo(), egui::Button::new("Undo"))
                        .clicked()
                    {
                        menu_actions.push(MenuAction::Undo);
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(self.scenes.history.can_redo(), egui::Button::new("Redo"))
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
                        if let Some(location) = self.dock_state.find_tab(&Tab::ProjectHub) {
                            self.dock_state.set_active_tab(location);
                        } else {
                            self.dock_state.add_window(vec![Tab::ProjectHub]);
                        }
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
                    if ui.button("Shader Editor").clicked() {
                        self.dock_state.add_window(vec![Tab::ShaderEditor]);
                        ui.close_menu();
                    }
                    if ui.button("Shader Editor").clicked() {
                        self.dock_state.add_window(vec![Tab::ShaderEditor]);
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
                    if ui.button("Input Rebinding").clicked() {
                        self.dock_state.add_window(vec![Tab::InputRebind]);
                        ui.close_menu();
                    }
                    if ui.button("Sprite Editor").clicked() {
                        self.dock_state.add_window(vec![Tab::SpriteEditor]);
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
                    if project_dirty {
                        ui.label("Unsaved changes");
                    }
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
                                        self.project_requests.push(ProjectRequest::CreateDialog);
                                    }
                                    ui.add_space(16.0);
                                    if ui
                                        .add_sized([200.0, 40.0], egui::Button::new("Open Project"))
                                        .clicked()
                                    {
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

        let requests = std::cell::RefCell::new(Vec::new());
        if !project_blocked {
            crate::project_hub::render_project_dialogs(
                ctx,
                &mut self.project_hub,
                &mut |path| requests.borrow_mut().push(ProjectRequest::Open(path)),
                &mut |name, path, template| {
                    requests
                        .borrow_mut()
                        .push(ProjectRequest::Create(name, path, template));
                },
            );
        }
        self.project_requests.extend(requests.into_inner());

        if self.show_save_as && !project_blocked {
            egui::Window::new("Save Project As")
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label("Project file (in the current project directory):");
                    ui.text_edit_singleline(&mut self.save_as_path);
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.show_save_as = false;
                        }
                        if ui.button("Save").clicked() {
                            self.project_requests
                                .push(ProjectRequest::SaveAs(PathBuf::from(
                                    self.save_as_path.trim(),
                                )));
                            self.show_save_as = false;
                        }
                    });
                });
        }
        if let Some(error) = self.project_error.clone().filter(|_| !project_blocked) {
            let mut open = true;
            egui::Window::new("Project operation failed")
                .open(&mut open)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label(error);
                });
            if !open {
                self.project_error = None;
            }
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
            // Scene pixels render below the dock UI; leave its canvas transparent.
            egui::CentralPanel::default()
                .frame(egui::Frame::none())
                .show(ctx, |ui| {
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
                        rename_input: &mut self.rename_input,
                        role_input: &mut self.role_input,
                        sprite_reference_input: &mut self.sprite_reference_input,
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
                        camera_state: &mut self.camera,
                        current_project: &mut self.current_project,
                        project_requests: &mut self.project_requests,
                        project_blocked,
                        play_toggle_requested: &mut self.play_toggle_requested,
                        scene_save_requested: &mut self.scene_save_requested,
                        scene_load_requested: &mut self.scene_load_requested,
                        play: self.play.as_mut(),
                        fps,
                        scenes: &mut self.scenes,
                        entity_textures: &self.entity_textures,
                        entity_roles: &self.entity_roles,
                        console_log: &mut self.console_log,
                        scene_view_rect: &mut self.scene_view_rect,
                        gizmo: &mut self.gizmo,
                        transform_edit: &mut self.transform_edit,
                        scene_camera: &scene_camera,
                        scene_world_size: world,
                        entity_id_to_hecs: &mut self.entity_id_to_hecs,
                        grid_snap_enabled: &mut self.grid_snap_enabled,
                        audio_editor: &mut self.audio_editor,
                        asset_filter: &mut self.asset_filter,
                        project_hub: &mut self.project_hub,
                        profiler_panel: &mut self.profiler_panel,
                        vfx_editor: &mut self.vfx_editor,
                        version_control: &mut self.version_control,
                        ui_layout_editor: &mut self.ui_layout_editor,
                        sprite_editor: &mut self.sprite_editor,
                        shader_editor: &mut self.shader_editor,
                        preferences: &mut self.preferences,
                        plugin_manager: &mut self.plugin_manager,
                        physics_editor: &mut self.physics_editor,
                        input_rebind: &mut self.input_rebind,
                        grid_snap_size: &mut self.grid_snap_size,
                        gpu: &*gpu,
                        debugger_panel: &mut self.debugger_panel,
                        build_panel: &mut self.build_panel,
                        anim_editor: &mut self.anim_editor,
                    };
                    tree.show(ctx, &mut tab_viewer);
                });

            // Status bar
            show_status_bar(ctx, fps, entity_count, tier, is_playing);
        }

        if let Some(action) = self.lifecycle.pending() {
            let intent = match action {
                ProjectAction::Open(_) => "opening another project",
                ProjectAction::Create { .. } => "creating another project",
                ProjectAction::Exit => "exiting the editor",
            };
            egui::Window::new("Unsaved project changes")
                .id(egui::Id::new("unsaved_project_prompt"))
                .order(egui::Order::Foreground)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(format!("Save your project before {intent}?"));
                    ui.label("Discard continues without saving your changes.");
                    if let Some(error) = self.lifecycle.save_error() {
                        ui.colored_label(egui::Color32::LIGHT_RED, format!("Save failed: {error}"));
                    }
                    ui.horizontal(|ui| {
                        for (label, decision) in [
                            ("Save", SaveDecision::Save),
                            ("Discard", SaveDecision::Discard),
                            ("Cancel", SaveDecision::Cancel),
                        ] {
                            if ui.button(label).clicked() {
                                self.project_requests
                                    .push(ProjectRequest::Resolve(decision));
                            }
                        }
                    });
                });
            if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                self.project_requests
                    .push(ProjectRequest::Resolve(SaveDecision::Cancel));
            }
        }

        if let Some(prompt) = &self.scene_close_prompt {
            let scene_name = self
                .scenes
                .tabs
                .get(prompt.index)
                .map(|tab| tab.scene_id.as_str())
                .unwrap_or("scene");
            egui::Window::new("Unsaved scene changes")
                .id(egui::Id::new("unsaved_scene_close_prompt"))
                .order(egui::Order::Foreground)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(format!(
                        "Save changes to scene `{scene_name}` before closing it?"
                    ));
                    ui.label("Discard restores this scene to its last saved version.");
                    if let Some(error) = &prompt.save_error {
                        ui.colored_label(egui::Color32::LIGHT_RED, format!("Save failed: {error}"));
                    }
                    ui.horizontal(|ui| {
                        for (label, decision) in [
                            ("Save", SaveDecision::Save),
                            ("Discard", SaveDecision::Discard),
                            ("Cancel", SaveDecision::Cancel),
                        ] {
                            if ui.button(label).clicked() {
                                self.project_requests
                                    .push(ProjectRequest::ResolveSceneClose(decision));
                            }
                        }
                    });
                });
            if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                self.project_requests
                    .push(ProjectRequest::ResolveSceneClose(SaveDecision::Cancel));
            }
        }

        if self.pending_recovery.is_some() {
            egui::Window::new("Recover unsaved project changes?")
                .id(egui::Id::new("project_recovery_prompt"))
                .order(egui::Order::Foreground)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("A newer recovery snapshot was found for this project.");
                    ui.label("Restore it to continue from your last autosave.");
                    ui.horizontal(|ui| {
                        if ui.button("Restore").clicked() {
                            self.project_requests
                                .push(ProjectRequest::ResolveRecovery(true));
                        }
                        if ui.button("Discard Recovery").clicked() {
                            self.project_requests
                                .push(ProjectRequest::ResolveRecovery(false));
                        }
                    });
                });
            if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                self.project_requests
                    .push(ProjectRequest::ResolveRecovery(false));
            }
        }

        // A hidden canvas or a release outside its bounds must still end the drag.
        if self.gizmo.drag.is_some()
            && !ctx.input(|input| input.pointer.primary_down())
            && !self
                .project_requests
                .iter()
                .any(|request| matches!(request, ProjectRequest::CancelTransform))
        {
            if let Some(edit) = self.transform_edit.take() {
                edit.commit(&mut self.scenes.history);
            }
            self.gizmo.drag = None;
        }

        // Text fields retain their own editing shortcuts. Scene shortcuts apply
        // after widgets have had a chance to consume keyboard input.
        if !project_blocked && !ctx.wants_keyboard_input() && self.play.is_none() {
            ctx.input_mut(|input| {
                for (key, action) in [
                    (egui::Key::S, MenuAction::SaveProject),
                    (egui::Key::Z, MenuAction::Undo),
                    (egui::Key::Y, MenuAction::Redo),
                ] {
                    if input.consume_shortcut(&egui::KeyboardShortcut::new(
                        egui::Modifiers::COMMAND,
                        key,
                    )) {
                        menu_actions.push(action);
                    }
                }
            });
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

        // Render the scene view with the engine's sprite renderer.
        //
        // The scene is drawn into the scene-view panel: the panel's rectangle
        // becomes a viewport, the world is projected with the same camera the
        // player uses, and the HUD is drawn on top in screen space. The
        // engine owns every pixel of the panel, so the editor shows what the
        // player shows.
        if let Some(panel) = self.scene_view_rect {
            let viewport = scene_view::canvas_viewport(panel, full_output.pixels_per_point, world);
            if !viewport.is_empty() {
                let queue = gpu.renderer.queue();
                let origin_x = viewport.x;
                let origin_y = viewport.y;

                // Queue the world: background, grid, tilemaps, sprites.
                gpu.scene_view_sprite_renderer.begin_frame();
                gpu.scene_view_sprite_renderer.push(scene_view::background(
                    world.0,
                    world.1,
                    BACKGROUND_COLOR,
                ));
                for line in scene_view::grid_lines(world.0, world.1, GRID_SPACING, GRID_COLOR) {
                    gpu.scene_view_sprite_renderer.push(line);
                }
                let camera = &scene_camera;
                let view_bounds = camera.view_bounds();
                let _ = kiln_runtime::update_global_transforms(&mut self.world);
                for (_, tilemap) in self.world.world().query::<(&Transform, &Tilemap)>().iter() {
                    for (texture, instance) in tilemap.collect_visible(view_bounds) {
                        gpu.scene_view_sprite_renderer
                            .push_textured(texture, instance);
                    }
                }
                for (transform, sprite) in self
                    .world
                    .world()
                    .query::<(&kiln_runtime::GlobalTransform, &Sprite)>()
                    .iter()
                {
                    gpu.scene_view_sprite_renderer.push_textured(
                        sprite.texture,
                        SpriteInstance {
                            position: [transform.position.x, transform.position.y],
                            size: sprite.size,
                            color: sprite.color,
                            uv_rect: sprite.uv_rect,
                            material_id: 0,
                            rotation: 0.0,
                        }
                        .with_transform(transform.rotation, [transform.scale.x, transform.scale.y]),
                    );
                }

                // Determine render target: post framebuffer if available, else main view.
                let (scene_target_view, use_post) =
                    if let Some(fb) = gpu.scene_view_post_framebuffer.as_ref() {
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
                                load: if use_post {
                                    wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT)
                                } else {
                                    wgpu::LoadOp::Load
                                },
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    pass.set_viewport(
                        origin_x as f32,
                        origin_y as f32,
                        viewport.width as f32,
                        viewport.height as f32,
                        0.0,
                        1.0,
                    );
                    pass.set_scissor_rect(origin_x, origin_y, viewport.width, viewport.height);
                    gpu.scene_view_sprite_renderer
                        .flush(&mut pass, camera, queue);
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
                    x: origin_x,
                    y: origin_y,
                    width: viewport.width,
                    height: viewport.height,
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
                    pass.set_viewport(
                        origin_x as f32,
                        origin_y as f32,
                        viewport.width as f32,
                        viewport.height as f32,
                        0.0,
                        1.0,
                    );
                    pass.set_scissor_rect(origin_x, origin_y, viewport.width, viewport.height);
                    gpu.scene_view_overlay_renderer
                        .flush(&mut pass, &overlay_camera, queue);
                }
            }
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

        gpu.renderer.queue().submit([encoder.finish()]);
        gpu.window.pre_present_notify();
        frame.present();

        // Execute menu actions after UI pass (borrows are released)
        for action in menu_actions {
            match action {
                MenuAction::Exit => self
                    .project_requests
                    .push(ProjectRequest::Leave(ProjectAction::Exit)),
                MenuAction::NewProject => self.begin_new_project(),
                MenuAction::OpenProject => {
                    self.show_welcome = false;
                    self.project_hub.show_open_project_dialog = true;
                    self.project_hub.open_project_path = self.project_path_input.clone();
                }
                MenuAction::SaveProject => {
                    if let Err(e) = self.save_project() {
                        self.project_error = Some(e);
                    }
                }
                MenuAction::SaveProjectAs => {
                    self.save_as_path = self
                        .project_path
                        .as_ref()
                        .map(|path| path.with_file_name("kiln-copy.toml"))
                        .unwrap_or_else(|| PathBuf::from("kiln-copy.toml"))
                        .to_string_lossy()
                        .into_owned();
                    self.show_save_as = true;
                }
                MenuAction::SaveScene => {
                    if let Err(e) = self.save_scene() {
                        self.project_error = Some(e);
                    }
                }
                MenuAction::LoadScene => {
                    if let Err(e) = self.load_scene() {
                        self.project_error = Some(e);
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
            self.project_requests.push(ProjectRequest::Open(path));
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
                self.project_error = Some(e);
            }
        }
        if std::mem::take(&mut self.scene_load_requested) {
            if let Err(e) = self.load_scene() {
                self.project_error = Some(e);
            }
        }

        // Update the frame counter and the measured frame rate.
        self.frame_count += 1;
        let elapsed = self.last_frame.elapsed();
        if elapsed.as_secs_f32() > 0.0 {
            self.fps = elapsed.as_secs_f32().recip();
        }
        self.last_frame = Instant::now();

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
    }
}
struct TabViewer<'a> {
    world: &'a mut GameWorld,
    selected_entity: &'a mut Option<hecs::Entity>,
    rename_input: &'a mut Option<(String, String)>,
    role_input: &'a mut Option<(String, String)>,
    sprite_reference_input: &'a mut Option<(String, SpriteReferenceDraft)>,
    is_playing: &'a mut bool,
    graph: &'a mut Graph,
    render_tier: &'a mut RenderTier,
    pass_timings: &'a [PassTiming],
    profiler_supported: bool,
    ui_designer: &'a mut UIDesignerState,
    prefab_editor: &'a mut PrefabEditorPanel,
    gameplay_editor: &'a mut GameplayEditorState,
    asset_browser: &'a mut AssetBrowser,
    tilemap_editor: &'a mut TilemapEditorState,
    hierarchy_panel: &'a mut HierarchyPanel,
    camera_state: &'a mut CameraState,
    current_project: &'a mut Option<Project>,
    project_requests: &'a mut Vec<ProjectRequest>,
    project_blocked: bool,
    play_toggle_requested: &'a mut bool,
    scene_save_requested: &'a mut bool,
    scene_load_requested: &'a mut bool,
    /// The running game session, drawn over the scene view.
    play: Option<&'a mut PlaySession>,
    /// The measured frame rate, in frames per second.
    fps: f32,
    /// The history of scene edits, used by undo and redo.
    scenes: &'a mut SceneWorkspace,
    /// Asset path of each scene entity's texture, keyed by entity id.
    entity_textures: &'a HashMap<String, String>,
    /// Gameplay role of each scene entity, keyed by entity id.
    entity_roles: &'a HashMap<String, String>,
    /// Lines shown in the console panel.
    console_log: &'a mut Vec<String>,
    /// Screen-space rectangle of the scene view panel, set by the panel each
    /// frame so the render loop can draw the scene into it.
    scene_view_rect: &'a mut Option<egui::Rect>,
    /// Transform gizmo state for visual entity manipulation.
    gizmo: &'a mut GizmoState,
    transform_edit: &'a mut Option<SceneTransaction>,
    scene_camera: &'a kiln_render::camera::Camera,
    scene_world_size: (f32, f32),
    /// Mapping from project entity ID (source_id) to hecs Entity for selection sync.
    entity_id_to_hecs: &'a mut HashMap<String, hecs::Entity>,
    /// Whether grid snapping is enabled.
    grid_snap_enabled: &'a mut bool,
    /// Audio editor state.
    audio_editor: &'a mut AudioEditorState,
    /// Filter text for the asset browser.
    asset_filter: &'a mut String,
    /// Project hub state.
    project_hub: &'a mut ProjectHubState,
    /// Profiler panel state.
    profiler_panel: &'a mut ProfilerPanel,
    /// VFX editor state.
    vfx_editor: &'a mut VfxEditorState,
    /// Version control panel state.
    version_control: &'a mut VersionControlPanel,
    /// UI layout editor state.
    ui_layout_editor: &'a mut UiLayoutEditorState,
    /// Sprite editor state.
    sprite_editor: &'a mut SpriteEditorState,
    /// Shader editor state.
    shader_editor: &'a mut ShaderEditorState,
    /// The current project, if any.
    /// Preferences panel state.
    preferences: &'a mut PreferencesPanel,
    /// Plugin manager panel state.
    plugin_manager: &'a mut PluginManagerPanel,
    /// Physics editor state.
    physics_editor: &'a mut PhysicsEditorState,
    /// Input rebinding panel state.
    input_rebind: &'a mut InputRebindPanel,
    /// Grid snap size for the scene editor.
    grid_snap_size: &'a mut f32,
    /// GPU state, if initialized.
    gpu: &'a GpuState,
    /// Debugger panel state.
    debugger_panel: &'a mut DebuggerPanel,
    /// Build panel state.
    build_panel: &'a mut BuildPanel,
    /// Animation editor state.
    anim_editor: &'a mut AnimationEditorState,
}

impl TabViewer<'_> {
    fn finish_transform_edit(&mut self) {
        if let Some(edit) = self.transform_edit.take() {
            edit.commit(&mut self.scenes.history);
        }
        self.gizmo.drag = None;
    }

    fn preview_transform(&mut self, source_id: &str, value: Transform) {
        if self.editing_locked() || self.project_blocked {
            return;
        }
        let before = crate::scene_document::snapshot_for_scene(
            self.world,
            self.entity_textures,
            self.entity_roles,
            self.current_project.as_ref(),
            self.scenes.active_id(),
        );
        if self
            .transform_edit
            .as_ref()
            .is_some_and(|edit| !edit.targets(&before.id, source_id))
        {
            self.finish_transform_edit();
        }
        let mut after = before.clone();
        let scene_id = after.id.clone();
        let result = SetTransform {
            scene_id: &scene_id,
            entity_id: source_id,
            value,
        }
        .apply(&mut after, self.world);
        match result {
            Ok(true) => {
                let edit = self
                    .transform_edit
                    .get_or_insert_with(|| SceneTransaction::new(before, source_id.into()));
                edit.update(after.clone());
                if let Some(project) = self.current_project.as_mut() {
                    project.scenes.insert(scene_id, after);
                }
            }
            Ok(false) => {}
            Err(error) => self.log(&format!("[ERROR] {error}")),
        }
    }

    fn preview_group_transforms(&mut self, source_id: &str, targets: &[(String, Transform)]) {
        if self.editing_locked() || self.project_blocked {
            return;
        }
        let before = crate::scene_document::snapshot_for_scene(
            self.world,
            self.entity_textures,
            self.entity_roles,
            self.current_project.as_ref(),
            self.scenes.active_id(),
        );
        if self
            .transform_edit
            .as_ref()
            .is_some_and(|edit| !edit.targets(&before.id, source_id))
        {
            self.finish_transform_edit();
        }
        let mut after = before.clone();
        let scene_id = after.id.clone();
        let mut updated = false;
        let ordered_targets = crate::transform_gizmo::order_group_targets(&after, targets);
        for (entity_id, target) in &ordered_targets {
            let Ok(local) =
                kiln_core::hierarchy_transform::local_from_world(&after, entity_id, *target)
            else {
                continue;
            };
            match (SetTransform {
                scene_id: &scene_id,
                entity_id,
                value: local,
            })
            .apply(&mut after, self.world)
            {
                Ok(changed) => updated |= changed,
                Err(error) => self.log(&format!("[ERROR] {error}")),
            }
        }
        if updated {
            let edit = self
                .transform_edit
                .get_or_insert_with(|| SceneTransaction::new(before, source_id.into()));
            edit.update(after.clone());
            if let Some(project) = self.current_project.as_mut() {
                project.scenes.insert(scene_id, after);
            }
        }
    }

    fn preview_sprite_visual(&mut self, source_id: &str, sprite: &Sprite) {
        if self.editing_locked() || self.project_blocked {
            return;
        }
        let before = crate::scene_document::snapshot_for_scene(
            self.world,
            self.entity_textures,
            self.entity_roles,
            self.current_project.as_ref(),
            self.scenes.active_id(),
        );
        if self
            .transform_edit
            .as_ref()
            .is_some_and(|edit| !edit.targets(&before.id, source_id))
        {
            self.finish_transform_edit();
        }
        let mut after = before.clone();
        let scene_id = after.id.clone();
        match (SetSpriteVisual {
            scene_id: &scene_id,
            entity_id: source_id,
            color: sprite.color,
            layer: sprite.layer,
            size: sprite.size,
        })
        .apply(&mut after, self.world)
        {
            Ok(true) => {
                let edit = self
                    .transform_edit
                    .get_or_insert_with(|| SceneTransaction::new(before, source_id.into()));
                edit.update(after.clone());
                if let Some(project) = self.current_project.as_mut() {
                    project.scenes.insert(scene_id, after);
                }
            }
            Ok(false) => {}
            Err(error) => self.log(&format!("[ERROR] {error}")),
        }
    }

    fn transform_response(
        &mut self,
        entity: hecs::Entity,
        value: Transform,
        response: &egui::Response,
    ) {
        if response.changed() {
            if self
                .transform_edit
                .as_ref()
                .is_some_and(|edit| !edit.owns_widget(response.id))
            {
                self.finish_transform_edit();
            }
            let source = self
                .world
                .world_ref()
                .get::<&SourceId>(entity)
                .ok()
                .map(|id| id.0.clone());
            if let Some(source) = source {
                self.preview_transform(&source, value);
            }
            if let Some(edit) = self.transform_edit.as_mut() {
                edit.set_widget(response.id);
            }
        }
        if self
            .transform_edit
            .as_ref()
            .is_some_and(|edit| edit.owns_widget(response.id))
            && (response.drag_stopped()
                || response.lost_focus()
                || (response.changed() && !response.dragged() && !response.has_focus()))
        {
            self.finish_transform_edit();
        }
    }

    fn sprite_visual_response(
        &mut self,
        entity: hecs::Entity,
        sprite: &Sprite,
        response: &egui::Response,
    ) {
        if response.changed() {
            if self
                .transform_edit
                .as_ref()
                .is_some_and(|edit| !edit.owns_widget(response.id))
            {
                self.finish_transform_edit();
            }
            let source = self
                .world
                .world_ref()
                .get::<&SourceId>(entity)
                .ok()
                .map(|id| id.0.clone());
            if let Some(source) = source {
                self.preview_sprite_visual(&source, sprite);
            }
            if let Some(edit) = self.transform_edit.as_mut() {
                edit.set_widget(response.id);
            }
        }
        if self
            .transform_edit
            .as_ref()
            .is_some_and(|edit| edit.owns_widget(response.id))
            && (response.drag_stopped()
                || response.lost_focus()
                || (response.changed() && !response.dragged() && !response.has_focus()))
        {
            self.finish_transform_edit();
        }
    }

    fn translation_canvas(&mut self, ui: &mut egui::Ui, panel: egui::Rect) {
        if self.editing_locked() || self.project_blocked || self.gizmo.mode == GizmoMode::None {
            return;
        }
        let ppp = ui.ctx().pixels_per_point();
        let viewport = scene_view::canvas_viewport(panel, ppp, self.world_size());
        let Some(units) = scene_view::units_per_point(viewport, ppp, self.scene_camera) else {
            return;
        };
        let canvas = egui::Rect::from_min_size(
            egui::pos2(viewport.x as f32 / ppp, viewport.y as f32 / ppp),
            egui::vec2(viewport.width as f32 / ppp, viewport.height as f32 / ppp),
        );
        let response = ui.interact(
            canvas,
            ui.id().with("transform_gizmo_canvas"),
            egui::Sense::drag(),
        );
        if response.drag_started() {
            self.finish_transform_edit();
            if let Some(entity) = *self.selected_entity {
                let source = self
                    .world
                    .world_ref()
                    .get::<&SourceId>(entity)
                    .ok()
                    .map(|id| id.0.clone());
                let initial = self
                    .world
                    .world_ref()
                    .get::<&kiln_runtime::GlobalTransform>(entity)
                    .ok()
                    .map(|t| t.0);
                let origin = ui.input(|input| input.pointer.press_origin());
                if let (Some(source), Some(initial), Some(origin)) = (source, initial, origin) {
                    let canvas = egui::Rect::from_min_size(
                        egui::pos2(viewport.x as f32 / ppp, viewport.y as f32 / ppp),
                        egui::vec2(viewport.width as f32 / ppp, viewport.height as f32 / ppp),
                    );
                    let center = egui::pos2(
                        canvas.left()
                            + (initial.position.x - self.scene_camera.position[0]) / units[0],
                        canvas.bottom()
                            - (initial.position.y - self.scene_camera.position[1]) / units[1],
                    );
                    if !hits_handle(self.gizmo.mode, origin, center) {
                        return;
                    }
                    let pointer_world = kiln_core::Vec2::new(
                        self.scene_camera.position[0] + (origin.x - canvas.left()) * units[0],
                        self.scene_camera.position[1] + (canvas.bottom() - origin.y) * units[1],
                    );
                    if let Some(drag) =
                        GizmoDrag::new(self.gizmo.mode, origin, initial, units, pointer_world)
                    {
                        let mut selected: Vec<(String, Transform)> = self
                            .hierarchy_panel
                            .selected_ids
                            .iter()
                            .filter_map(|id| {
                                let entity = *self.entity_id_to_hecs.get(id)?;
                                let transform = self
                                    .world
                                    .world_ref()
                                    .get::<&kiln_runtime::GlobalTransform>(entity)
                                    .ok()?
                                    .0;
                                Some((id.clone(), transform))
                            })
                            .collect();
                        if !selected.iter().any(|(id, _)| id == &source) {
                            selected.push((source.clone(), initial));
                        }
                        self.gizmo.drag = Some(GizmoDragSession {
                            source,
                            drag,
                            selected,
                        });
                    }
                }
            }
        }
        if self.gizmo.drag.is_some()
            && ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.project_requests.push(ProjectRequest::CancelTransform);
            self.gizmo.drag = None;
            return;
        }
        if response.dragged() || response.drag_stopped() {
            if let (Some(session), Some(pointer)) = (
                self.gizmo.drag.clone(),
                ui.input(|input| input.pointer.interact_pos()),
            ) {
                let grid = (*self.grid_snap_enabled).then_some(*self.grid_snap_size);
                let world_value = session.drag.at(pointer, grid);
                if session.selected.len() > 1 {
                    let targets = crate::transform_gizmo::apply_group_transform(
                        self.gizmo.mode,
                        &session.source,
                        world_value,
                        &session.selected,
                    );
                    self.preview_group_transforms(&session.source, &targets);
                } else {
                    let local_value = crate::scene_document::snapshot_for_scene(
                        self.world,
                        self.entity_textures,
                        self.entity_roles,
                        self.current_project.as_ref(),
                        self.scenes.active_id(),
                    );
                    if let Ok(local) = kiln_core::hierarchy_transform::local_from_world(
                        &local_value,
                        &session.source,
                        world_value,
                    ) {
                        self.preview_transform(&session.source, local);
                    } else {
                        self.log(
                            "[ERROR] Could not convert dragged world transform to local space",
                        );
                    }
                }
            }
        }
        if response.drag_stopped() {
            self.finish_transform_edit();
        }
    }

    /// Queue creation through the validated scene document command path.
    fn add_entity(&mut self) {
        if self.editing_locked() {
            return;
        }
        self.finish_transform_edit();
        let Some(scene_id) = self.scenes.active_id().map(str::to_owned) else {
            return;
        };
        let size = self.world_size();
        let position = [size.0 / 2.0, size.1 / 2.0];
        self.project_requests.push(ProjectRequest::Entity(
            crate::entity_command::EntityCommand {
                scene_id,
                edit: crate::entity_command::EntityEdit::Create {
                    name: "Entity".into(),
                    position,
                },
            },
        ));
    }

    /// The size of the world the scene view shows, in world units.
    fn world_size(&self) -> (f32, f32) {
        self.scene_world_size
    }

    fn authored_entity(&self, entity: hecs::Entity) -> Option<kiln_core::Entity> {
        let source = self
            .world
            .world_ref()
            .get::<&SourceId>(entity)
            .ok()?
            .0
            .clone();
        crate::scene_document::snapshot_for_scene(
            self.world,
            self.entity_textures,
            self.entity_roles,
            self.current_project.as_ref(),
            self.scenes.active_id(),
        )
        .entities
        .into_iter()
        .find(|entity| entity.id == source)
    }

    fn queue_component(&mut self, entity: hecs::Entity, edit: ComponentEdit) {
        if self.editing_locked() || self.project_blocked {
            return;
        }
        let Some(scene_id) = self.scenes.active_id().map(str::to_owned) else {
            return;
        };
        let source = self
            .world
            .world_ref()
            .get::<&SourceId>(entity)
            .ok()
            .map(|id| id.0.clone());
        if let Some(entity_id) = source {
            self.project_requests
                .push(ProjectRequest::Component(ComponentCommand {
                    scene_id,
                    entity_id,
                    edit,
                }));
        }
    }

    /// Remove an authored component, including unresolved resource references.
    fn remove_component(&mut self, entity: hecs::Entity, kind: ComponentKind) {
        self.queue_component(entity, ComponentEdit::Remove(kind));
    }

    /// Add an explicit authored component, independent of projection defaults.
    fn add_component(&mut self, entity: hecs::Entity, kind: ComponentKind) {
        if let Some(component) = kind.default_component() {
            self.queue_component(entity, ComponentEdit::Add(component));
        }
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

        let scene = crate::scene_document::snapshot_for_scene(
            self.world,
            self.entity_textures,
            self.entity_roles,
            self.current_project.as_ref(),
            self.scenes.active_id(),
        );
        let Some(source) = scene.entities.iter().find(|e| e.id == source_id).cloned() else {
            self.log("[INFO] The selected entity has no scene data");
            return;
        };

        let name = format!("Prefab_{}", source.id);
        let prefab = crate::editing::prefab_from_entity(&source, &name);
        let id = prefab.id.clone();
        self.prefab_editor.state.add_prefab(prefab);
        self.prefab_editor.state.select_prefab(&id);
        self.log(&format!("[INFO] Created the prefab {name}"));
    }

    /// Add an instance of the selected prefab to the scene.
    fn instantiate_selected_prefab(&mut self) {
        self.finish_transform_edit();
        let Some(prefab) = self.prefab_editor.state.selected_prefab().cloned() else {
            self.log("[INFO] Select a prefab to instantiate it");
            return;
        };

        let before = crate::scene_document::snapshot_for_scene(
            self.world,
            self.entity_textures,
            self.entity_roles,
            self.current_project.as_ref(),
            self.scenes.active_id(),
        );
        let existing: Vec<String> = self.entity_textures.keys().cloned().collect();
        let size = self.world_size();
        let position = [size.0 / 2.0, size.1 / 2.0];
        let id = crate::editing::instantiate_prefab(self.world, &prefab, &existing, position);
        match id {
            Some(id) => {
                self.scenes.history.record(before);
                self.log(&format!("[INFO] Instantiated the prefab as {id}"));
            }
            None => self.log("[ERROR] The prefab could not be instantiated"),
        }
    }

    /// Create and activate a real scene after this UI pass.
    fn create_scene(&mut self) {
        if !self.editing_locked() {
            self.project_requests
                .push(ProjectRequest::Scene(SceneAction::Create));
        }
    }

    /// Add another scene as a nested scene of the current one.
    fn nest_scene(&mut self) {
        self.finish_transform_edit();
        let Some(project) = self.current_project.as_mut() else {
            self.log("[INFO] Open a project to nest a scene in it");
            return;
        };
        let Some(parent_id) = self.scenes.active_id().map(str::to_owned) else {
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
        if self.project_blocked {
            ui.disable();
        }
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

                        if ui.button("Frame Selection (F)").clicked()
                            || (ui.ctx().input(|input| input.key_pressed(egui::Key::F))
                                && !ui.ctx().wants_keyboard_input())
                        {
                            self.frame_selection();
                        }

                        ui.separator();

                        // Gizmo mode toggle
                        let gizmo_mode_label = match self.gizmo.mode {
                            GizmoMode::Translate => "Gizmo: Move",
                            GizmoMode::Rotate => "Gizmo: Rotate",
                            GizmoMode::Scale => "Gizmo: Scale",
                            GizmoMode::None => "Gizmo: None",
                        };
                        if ui.button(gizmo_mode_label).clicked() {
                            self.finish_transform_edit();
                            self.gizmo.mode = match self.gizmo.mode {
                                GizmoMode::None => GizmoMode::Translate,
                                GizmoMode::Translate => GizmoMode::Rotate,
                                GizmoMode::Rotate => GizmoMode::Scale,
                                GizmoMode::Scale => GizmoMode::None,
                            };
                        }

                        ui.separator();

                        // Grid snap controls
                        ui.label("Grid:");
                        ui.add_sized([60.0, 20.0], egui::DragValue::new(self.grid_snap_size));
                        if ui
                            .button(
                                (if *self.grid_snap_enabled {
                                    "✓"
                                } else {
                                    "○"
                                })
                                .to_string(),
                            )
                            .clicked()
                        {
                            *self.grid_snap_enabled = !*self.grid_snap_enabled;
                        }
                        ui.label("Snap");
                    });

                    if ui.button("Save Scene").clicked() {
                        *self.scene_save_requested = true;
                    }
                    if ui.button("Load Scene").clicked() {
                        *self.scene_load_requested = true;
                    }

                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if let Some(play) = &self.play {
                                ui.label(format!("Playing: {}", play.scene_id));
                            }
                            ui.label(format!("FPS: {:.0}", self.fps));
                        });
                    });
                });

                // Requests apply next frame, after dock UI borrows are released.
                if self.current_project.is_some() {
                    ui.add_space(4.0);
                    ui.add_enabled_ui(!self.editing_locked(), |ui| {
                        ui.horizontal(|ui| {
                            if ui.button("+").on_hover_text("Create scene").clicked() {
                                self.project_requests
                                    .push(ProjectRequest::Scene(SceneAction::Create));
                            }
                            ui.menu_button("Scenes", |ui| {
                                if let Some(project) = self.current_project.as_ref() {
                                    let mut ids: Vec<_> = project.scenes.keys().collect();
                                    ids.sort();
                                    for id in ids {
                                        if ui.button(id).clicked() {
                                            self.project_requests.push(ProjectRequest::Scene(
                                                SceneAction::Open(id.clone()),
                                            ));
                                            ui.close_menu();
                                        }
                                    }
                                }
                            });
                            for (index, tab) in self.scenes.tabs.iter().enumerate() {
                                let response = ui
                                    .selectable_label(
                                        self.scenes.active == Some(index),
                                        format!(
                                            "{}{}",
                                            if tab.dirty { "*" } else { "" },
                                            tab.scene_id
                                        ),
                                    )
                                    .on_hover_text(
                                        "Click to switch; right-click to close a saved tab",
                                    );
                                if response.clicked() {
                                    self.project_requests
                                        .push(ProjectRequest::Scene(SceneAction::Switch(index)));
                                }
                                if response.secondary_clicked() {
                                    self.project_requests
                                        .push(ProjectRequest::Scene(SceneAction::Close(index)));
                                }
                            }
                        });
                    });
                }

                // Entity list for selection
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.heading("Entities");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add_enabled(!self.editing_locked(), egui::Button::new("+ Add Entity"))
                            .clicked()
                        {
                            self.add_entity();
                        }
                    });
                });

                ui.add_space(4.0);

                egui::ScrollArea::vertical()
                    .max_height(120.0)
                    .show(ui, |ui| {
                        let scene = crate::scene_document::snapshot_for_scene(
                            self.world,
                            self.entity_textures,
                            self.entity_roles,
                            self.current_project.as_ref(),
                            self.scenes.active_id(),
                        );
                        let handles: HashMap<_, _> = self
                            .world
                            .world_ref()
                            .query::<(hecs::Entity, &SourceId)>()
                            .iter()
                            .map(|(entity, source)| (source.0.clone(), entity))
                            .collect();
                        let entities: Vec<_> = scene
                            .entities
                            .iter()
                            .filter_map(|authored| {
                                let entity = *handles.get(&authored.id)?;
                                let label = format!(
                                    "{}{}",
                                    authored.name,
                                    if authored
                                        .components
                                        .iter()
                                        .any(|c| matches!(c, kiln_core::Component::Sprite { .. }))
                                    {
                                        ""
                                    } else {
                                        " (nonvisual)"
                                    }
                                );
                                Some((entity, authored.id.clone(), label))
                            })
                            .collect();
                        for (entity, source, label) in entities {
                            let selected = self.hierarchy_panel.selected_ids.contains(&source);
                            if ui
                                .selectable_label(selected, label)
                                .on_hover_text(&source)
                                .clicked()
                            {
                                self.finish_transform_edit();
                                *self.selected_entity = Some(entity);
                                let additive = ui
                                    .input(|input| input.modifiers.command || input.modifiers.ctrl);
                                if additive {
                                    self.hierarchy_panel.toggle_select(&source);
                                } else {
                                    self.hierarchy_panel.select(&source);
                                }
                                *self.selected_entity = self
                                    .hierarchy_panel
                                    .selected_entity_id()
                                    .and_then(|id| self.entity_id_to_hecs.get(id).copied());
                            }
                        }
                    });
                let rect = ui.available_rect_before_wrap();

                ui.add_space(8.0);

                // The engine draws the scene here, so the panel must not
                // paint over it. The rectangle is handed to the render
                // loop, which converts it to pixels and draws the scene
                // with the engine's sprite renderer.
                *self.scene_view_rect = Some(rect);
                self.translation_canvas(ui, rect);

                // Draw gizmo mode indicator in the top-left of the scene view
                if self.gizmo.mode != GizmoMode::None {
                    if let Some(entity) = self.selected_entity {
                        let gizmo_color = match self.gizmo.mode {
                            GizmoMode::Translate => egui::Color32::from_rgb(255, 0, 0),
                            GizmoMode::Rotate => egui::Color32::from_rgb(0, 255, 0),
                            GizmoMode::Scale => egui::Color32::from_rgb(0, 0, 255),
                            GizmoMode::None => egui::Color32::from_rgb(128, 128, 128),
                        };
                        ui.painter().rect_filled(
                            egui::Rect::from_min_size(
                                rect.min + egui::vec2(8.0, 8.0),
                                egui::vec2(16.0, 16.0),
                            ),
                            4.0,
                            gizmo_color,
                        );
                        ui.painter().text(
                            rect.min + egui::vec2(28.0, 12.0),
                            egui::Align2::LEFT_TOP,
                            format!("Gizmo: {:?}", self.gizmo.mode),
                            egui::FontId::proportional(10.0),
                            gizmo_color,
                        );
                        if let Ok(transform) = self
                            .world
                            .world_ref()
                            .get::<&kiln_runtime::GlobalTransform>(*entity)
                        {
                            let ppp = ui.ctx().pixels_per_point();
                            let viewport =
                                scene_view::canvas_viewport(rect, ppp, self.world_size());
                            if let Some(units) =
                                scene_view::units_per_point(viewport, ppp, self.scene_camera)
                            {
                                let canvas = egui::Rect::from_min_size(
                                    egui::pos2(viewport.x as f32 / ppp, viewport.y as f32 / ppp),
                                    egui::vec2(
                                        viewport.width as f32 / ppp,
                                        viewport.height as f32 / ppp,
                                    ),
                                );
                                let center = egui::pos2(
                                    canvas.left()
                                        + (transform.position.x - self.scene_camera.position[0])
                                            / units[0],
                                    canvas.bottom()
                                        - (transform.position.y - self.scene_camera.position[1])
                                            / units[1],
                                );
                                match self.gizmo.mode {
                                    GizmoMode::Translate => {
                                        ui.painter().line_segment(
                                            [center, center + egui::vec2(42.0, 0.0)],
                                            egui::Stroke::new(
                                                2.0_f32,
                                                egui::Color32::from_rgb(255, 80, 80),
                                            ),
                                        );
                                        ui.painter().line_segment(
                                            [center, center + egui::vec2(0.0, -42.0)],
                                            egui::Stroke::new(
                                                2.0_f32,
                                                egui::Color32::from_rgb(80, 255, 100),
                                            ),
                                        );
                                    }
                                    GizmoMode::Rotate => {
                                        ui.painter().circle_stroke(
                                            center,
                                            38.0,
                                            egui::Stroke::new(2.0_f32, gizmo_color),
                                        );
                                    }
                                    GizmoMode::Scale => {
                                        let handle = center + egui::vec2(54.0, 0.0);
                                        ui.painter().line_segment(
                                            [center, handle],
                                            egui::Stroke::new(2.0_f32, gizmo_color),
                                        );
                                        ui.painter().rect_filled(
                                            egui::Rect::from_center_size(
                                                handle,
                                                egui::vec2(12.0, 12.0),
                                            ),
                                            2.0,
                                            gizmo_color,
                                        );
                                    }
                                    GizmoMode::None => {}
                                }
                            }
                        }
                    }
                }

                // The grid is drawn by the engine as world-space quads, so
                // it lines up with the world instead of the panel.

                // The sprites and the HUD are drawn by the engine too, so
                // the editor shows the same pixels the player shows. The
                // panel itself draws nothing over the canvas.
            }
            Tab::Inspector => {
                ui.vertical(|ui| {
                    ui.heading("Inspector");
                    if self.editing_locked() { ui.disable(); }

                    // The selected entity is copied out, so the world can be
                    // borrowed while the inspector edits it.
                    let selected = *self.selected_entity;
                    if let Some(entity) = selected {
                        let Some(authored) = self.authored_entity(entity) else {
                            ui.label("Select a root-scene entity to edit components.");
                            return;
                        };
                        let has = |kind| authored.components.iter().any(|c| ComponentKind::of(c) == Some(kind));
                        let has_transform = has(ComponentKind::Transform);
                        let has_sprite = has(ComponentKind::Sprite);
                        let has_role = has(ComponentKind::Role);
                        let has_tilemap = has(ComponentKind::Tilemap);
                        if self
                            .rename_input
                            .as_ref()
                            .is_some_and(|(id, _)| id != &authored.id)
                        {
                            *self.rename_input = None;
                        }
                        if self
                            .rename_input
                            .as_ref()
                            .is_some_and(|(id, _)| id == &authored.id)
                        {
                            let mut apply_rename = false;
                            let mut cancel_rename = false;
                            let can_rename = !self.editing_locked() && !self.project_blocked;
                            ui.horizontal(|ui| {
                                if let Some((_, name)) = self.rename_input.as_mut() {
                                    ui.add_enabled(
                                        can_rename,
                                        egui::TextEdit::singleline(name),
                                    );
                                    apply_rename = ui
                                        .add_enabled(can_rename, egui::Button::new("Apply"))
                                        .clicked();
                                    cancel_rename = ui.button("Cancel").clicked();
                                }
                            });
                            if apply_rename {
                                if let Some((entity_id, name)) = self.rename_input.take() {
                                    if let Some(scene_id) = self.scenes.active_id().map(str::to_owned) {
                                        self.project_requests.push(ProjectRequest::Entity(
                                            crate::entity_command::EntityCommand {
                                                scene_id,
                                                edit: crate::entity_command::EntityEdit::Rename {
                                                    entity_id,
                                                    name,
                                                },
                                            },
                                        ));
                                    }
                                }
                            } else if cancel_rename {
                                *self.rename_input = None;
                            }
                        } else {
                            ui.horizontal(|ui| {
                                ui.label(format!("{} ({})", authored.name, authored.id));
                                if ui
                                    .add_enabled(
                                        !self.editing_locked() && !self.project_blocked,
                                        egui::Button::new("Rename"),
                                    )
                                    .clicked()
                                {
                                    *self.rename_input =
                                        Some((authored.id.clone(), authored.name.clone()));
                                }
                            });
                        }
                        ui.add_space(8.0);

                        // Read current values once
                        let transform = authored.components.iter().find_map(|component| match component {
                            kiln_core::Component::Transform { x, y, rotation, scale } => Some(
                                Transform::new(*x, *y)
                                    .with_rotation(*rotation)
                                    .with_scale(kiln_core::Vec2::splat(*scale)),
                            ),
                            _ => None,
                        }).filter(|_| has_transform);
                        let sprite = self
                            .world
                            .world()
                            .get::<&Sprite>(entity)
                            .ok()
                            .map(|s| (*s).clone()).filter(|_| has_sprite);

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
                                        self.transform_response(entity, transform, &response);
                                        ui.add_space(8.0);
                                        ui.label("Y:");
                                        let response = ui.add_enabled(
                                            !self.editing_locked(),
                                            egui::DragValue::new(&mut transform.position.y)
                                                .speed(1.0),
                                        );
                                        self.transform_response(entity, transform, &response);
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Rotation");
                                        ui.add_space(7.0);
                                        let mut degrees = transform.rotation.to_degrees();
                                        let response = ui.add_enabled(
                                            !self.editing_locked(),
                                            egui::DragValue::new(&mut degrees)
                                                .speed(1.0)
                                                .suffix("°"),
                                        );
                                        if response.changed() {
                                            transform.rotation = degrees.to_radians();
                                        }
                                        self.transform_response(entity, transform, &response);
                                        ui.add_space(8.0);
                                        ui.label("Scale");
                                        let mut scale = transform.scale.x;
                                        let response = ui.add_enabled(
                                            !self.editing_locked(),
                                            egui::DragValue::new(&mut scale).speed(0.01),
                                        );
                                        if response.changed() {
                                            transform.scale = kiln_core::Vec2::splat(scale);
                                        }
                                        self.transform_response(entity, transform, &response);
                                    });
                                });
                        }

                        // Sprite component
                        if let Some(mut sprite) = sprite.clone() {
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
                                        if response.changed() {
                                            self.sprite_visual_response(entity, &sprite, &response);
                                        }
                                        ui.add_space(8.0);
                                        ui.label("G:");
                                        let response = ui.add_enabled(
                                            !self.editing_locked(),
                                            egui::DragValue::new(&mut sprite.color[1])
                                                .speed(0.01)
                                                .range(0.0..=1.0),
                                        );
                                        if response.changed() {
                                            self.sprite_visual_response(entity, &sprite, &response);
                                        }
                                        ui.add_space(8.0);
                                        ui.label("B:");
                                        let response = ui.add_enabled(
                                            !self.editing_locked(),
                                            egui::DragValue::new(&mut sprite.color[2])
                                                .speed(0.01)
                                                .range(0.0..=1.0),
                                        );
                                        if response.changed() {
                                            self.sprite_visual_response(entity, &sprite, &response);
                                        }
                                        ui.add_space(8.0);
                                        ui.label("A:");
                                        let response = ui.add_enabled(
                                            !self.editing_locked(),
                                            egui::DragValue::new(&mut sprite.color[3])
                                                .speed(0.01)
                                                .range(0.0..=1.0),
                                        );
                                        if response.changed() {
                                            self.sprite_visual_response(entity, &sprite, &response);
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Size");
                                        ui.add_space(16.0);
                                        ui.label("W:");
                                        let response = ui.add_enabled(
                                            !self.editing_locked(),
                                            egui::DragValue::new(&mut sprite.size[0])
                                                .speed(0.5)
                                                .range(0.001..=8192.0)
                                                .suffix(" px"),
                                        );
                                        if response.changed() {
                                            self.sprite_visual_response(entity, &sprite, &response);
                                        }
                                        ui.add_space(8.0);
                                        ui.label("H:");
                                        let response = ui.add_enabled(
                                            !self.editing_locked(),
                                            egui::DragValue::new(&mut sprite.size[1])
                                                .speed(0.5)
                                                .range(0.001..=8192.0)
                                                .suffix(" px"),
                                        );
                                        if response.changed() {
                                            self.sprite_visual_response(entity, &sprite, &response);
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Layer");
                                        let response = ui.add_enabled(
                                            !self.editing_locked(),
                                            egui::DragValue::new(&mut sprite.layer).speed(0.1),
                                        );
                                        if response.changed() {
                                            self.sprite_visual_response(entity, &sprite, &response);
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

                        ui.add_space(12.0);
                        if let Some((texture, material, animation_clip)) = authored.components.iter().find_map(|c| match c {
                            kiln_core::Component::Sprite { texture, material, animation_clip, .. } => Some((texture, material, animation_clip)),
                            _ => None,
                        }) {
                            if self.sprite_reference_input.as_ref().is_some_and(|(id, _)| id != &authored.id) {
                                *self.sprite_reference_input = None;
                            }
                            ui.group(|ui| {
                                ui.strong("Sprite References");
                                if self.sprite_reference_input.as_ref().is_some_and(|(id, _)| id == &authored.id) {
                                    let mut apply = false;
                                    let mut cancel = false;
                                    let enabled = !self.editing_locked() && !self.project_blocked;
                                    ui.vertical(|ui| {
                                        if let Some((_, draft)) = self.sprite_reference_input.as_mut() {
                                            ui.label("Texture path");
                                            ui.add_enabled(enabled, egui::TextEdit::singleline(&mut draft.texture));
                                            if let Some(project) = self.current_project.as_ref() {
                                                let mut paths = crate::scene_view::texture_paths(project);
                                                paths.extend(
                                                    self.asset_browser
                                                        .entries()
                                                        .iter()
                                                        .filter(|entry| entry.kind == crate::assets::AssetKind::Image)
                                                        .map(|entry| entry.path.clone()),
                                                );
                                                paths.sort();
                                                paths.dedup();
                                                egui::ComboBox::from_id_salt(("sprite_texture", &authored.id))
                                                    .selected_text("Choose project texture…")
                                                    .show_ui(ui, |ui| {
                                                        for path in paths {
                                                            ui.selectable_value(&mut draft.texture, path.clone(), path);
                                                        }
                                                    });
                                            }
                                            ui.label("Material");
                                            ui.add_enabled(enabled, egui::TextEdit::singleline(&mut draft.material));
                                            if let Some(project) = self.current_project.as_ref() {
                                                let mut ids: Vec<_> = project.materials.keys().cloned().collect();
                                                ids.sort();
                                                egui::ComboBox::from_id_salt(("sprite_material", &authored.id))
                                                    .selected_text("Choose project material…")
                                                    .show_ui(ui, |ui| {
                                                        for id in ids {
                                                            ui.selectable_value(&mut draft.material, id.clone(), id);
                                                        }
                                                    });
                                            }
                                            ui.label("Animation clip");
                                            ui.add_enabled(enabled, egui::TextEdit::singleline(&mut draft.animation_clip));
                                            if let Some(project) = self.current_project.as_ref() {
                                                let mut ids: Vec<_> = project.animation_clips.keys().cloned().collect();
                                                ids.sort();
                                                egui::ComboBox::from_id_salt(("sprite_animation", &authored.id))
                                                    .selected_text("Choose project animation…")
                                                    .show_ui(ui, |ui| {
                                                        for id in ids {
                                                            ui.selectable_value(&mut draft.animation_clip, id.clone(), id);
                                                        }
                                                    });
                                            }
                                        }
                                        ui.horizontal(|ui| {
                                            apply = ui.add_enabled(enabled, egui::Button::new("Apply")).clicked();
                                            cancel = ui.button("Cancel").clicked();
                                        });
                                    });
                                    if apply {
                                        if let Some((_, draft)) = self.sprite_reference_input.take() {
                                            let option = |value: String| (!value.trim().is_empty()).then_some(value);
                                            self.queue_component(entity, ComponentEdit::SetSpriteReferences {
                                                texture: option(draft.texture),
                                                material: option(draft.material),
                                                animation_clip: option(draft.animation_clip),
                                            });
                                        }
                                    } else if cancel {
                                        *self.sprite_reference_input = None;
                                    }
                                } else {
                                    ui.label(format!("Texture: {}", texture.as_deref().unwrap_or("(none)")));
                                    ui.label(format!("Material: {}", material.as_deref().unwrap_or("(none)")));
                                    ui.label(format!("Animation: {}", animation_clip.as_deref().unwrap_or("(none)")));
                                    if let Some(path) = texture {
                                        let image_exists = self.asset_browser.entries().iter().any(|entry| {
                                            entry.kind == crate::assets::AssetKind::Image && entry.path == *path
                                        });
                                        if !image_exists {
                                            ui.colored_label(egui::Color32::YELLOW, "Texture asset is missing or is not a PNG image");
                                        } else if self.asset_browser.errors().iter().any(|(failed_path, _)| failed_path == path) {
                                            ui.colored_label(egui::Color32::YELLOW, "Texture asset could not be decoded");
                                        }
                                    }
                                    if let Some(project) = self.current_project.as_ref() {
                                        if material.as_ref().is_some_and(|id| !project.materials.contains_key(id)) {
                                            ui.colored_label(egui::Color32::YELLOW, "Sprite material reference is missing");
                                        }
                                        if animation_clip.as_ref().is_some_and(|id| !project.animation_clips.contains_key(id)) {
                                            ui.colored_label(egui::Color32::YELLOW, "Animation clip reference is missing");
                                        }
                                    }
                                    if ui.add_enabled(!self.editing_locked() && !self.project_blocked, egui::Button::new("Edit References")).clicked() {
                                        *self.sprite_reference_input = Some((authored.id.clone(), SpriteReferenceDraft {
                                            texture: texture.clone().unwrap_or_default(),
                                            material: material.clone().unwrap_or_default(),
                                            animation_clip: animation_clip.clone().unwrap_or_default(),
                                        }));
                                    }
                                }
                            });
                        }

                        if self
                            .role_input
                            .as_ref()
                            .is_some_and(|(id, _)| id != &authored.id)
                        {
                            *self.role_input = None;
                        }
                        if let Some(role) = authored.components.iter().find_map(|c| match c { kiln_core::Component::Role { role } => Some(role), _ => None }) {
                            ui.group(|ui| {
                                if self
                                    .role_input
                                    .as_ref()
                                    .is_some_and(|(id, _)| id == &authored.id)
                                {
                                    let mut apply_role = false;
                                    let mut cancel_role = false;
                                    let can_edit = !self.editing_locked() && !self.project_blocked;
                                    ui.horizontal(|ui| {
                                        if let Some((_, value)) = self.role_input.as_mut() {
                                            ui.add_enabled(
                                                can_edit,
                                                egui::TextEdit::singleline(value),
                                            );
                                            apply_role = ui
                                                .add_enabled(can_edit, egui::Button::new("Apply"))
                                                .clicked();
                                            cancel_role = ui.button("Cancel").clicked();
                                        }
                                    });
                                    if apply_role {
                                        if let Some((_, role)) = self.role_input.take() {
                                            self.queue_component(
                                                entity,
                                                ComponentEdit::SetRole { role },
                                            );
                                        }
                                    } else if cancel_role {
                                        *self.role_input = None;
                                    }
                                } else {
                                    ui.horizontal(|ui| {
                                        ui.label(format!("Role: {role}"));
                                        if ui
                                            .add_enabled(
                                                !self.editing_locked() && !self.project_blocked,
                                                egui::Button::new("Edit"),
                                            )
                                            .clicked()
                                        {
                                            *self.role_input =
                                                Some((authored.id.clone(), role.clone()));
                                        }
                                    });
                                }
                                if ui.button("Remove Role").clicked() { self.remove_component(entity, ComponentKind::Role); }
                            });
                        }
                        if let Some(map) = authored.components.iter().find_map(|c| match c { kiln_core::Component::Tilemap { map } => Some(map), _ => None }) {
                            let mut map_options: Vec<_> = self.current_project.as_ref()
                                .map(|project| project.tilemaps.keys().cloned().collect())
                                .unwrap_or_default();
                            map_options.sort();
                            ui.group(|ui| {
                                let map_exists = self.current_project.as_ref().is_some_and(|p| p.tilemaps.contains_key(map));
                                ui.label(format!("Tilemap: {map}"));
                                if !map_exists {
                                    ui.colored_label(egui::Color32::YELLOW, "Tilemap resource is missing");
                                }
                                if !map_options.is_empty() {
                                    egui::ComboBox::from_id_salt(("entity_tilemap", &authored.id))
                                        .selected_text("Change Tilemap…")
                                        .show_ui(ui, |ui| {
                                            for candidate in &map_options {
                                                if ui.selectable_label(candidate == map, candidate).clicked() {
                                                    self.queue_component(entity, ComponentEdit::SetTilemapReference { map: candidate.clone() });
                                                }
                                            }
                                        });
                                } else {
                                    ui.label("Create a tilemap resource in the Tilemap Editor first.");
                                }
                                if ui.button("Remove Tilemap").clicked() { self.remove_component(entity, ComponentKind::Tilemap); }
                            });
                        }
                        ui.add_space(12.0);
                        ui.horizontal_wrapped(|ui| {
                            if !has_transform && ui.button("+ Transform").clicked() { self.add_component(entity, ComponentKind::Transform); }
                            if !has_sprite && ui.button("+ Sprite").clicked() { self.add_component(entity, ComponentKind::Sprite); }
                            if !has_role && ui.button("+ Role").clicked() { self.add_component(entity, ComponentKind::Role); }
                            if !has_tilemap {
                                ui.menu_button("+ Tilemap", |ui| {
                                    let mut maps: Vec<_> = self.current_project.as_ref().map(|p| p.tilemaps.keys().cloned().collect()).unwrap_or_default();
                                    maps.sort();
                                    if maps.is_empty() { ui.label("Create a tilemap resource in the Tilemap Editor first."); }
                                    for map in maps {
                                        if ui.button(&map).clicked() {
                                            self.queue_component(entity, ComponentEdit::Add(kiln_core::Component::Tilemap { map }));
                                            ui.close_menu();
                                        }
                                    }
                                });
                            }
                        });
                        if has_transform && has_sprite && has_role && has_tilemap { ui.label("The entity has every inspector component."); }
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
                            if ui.button("New Folder").clicked() {
                                if let Some(dir) =
                                    self.asset_browser.root().map(|p| p.to_path_buf())
                                {
                                    if let Err(e) = std::fs::create_dir(dir.join("New Folder")) {
                                        self.log(&format!("Failed to create folder: {e}"));
                                    } else {
                                        self.asset_browser.scan(&dir);
                                    }
                                }
                            }
                        });
                        ui.add_space(8.0);
                        ui.label("Filter:");
                        ui.text_edit_singleline(self.asset_filter);
                        if self.asset_browser.filter() != self.asset_filter {
                            self.asset_browser.set_filter(self.asset_filter.clone());
                        }
                    });

                    ui.add_space(8.0);

                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(35, 35, 40))
                        .rounding(4.0)
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            let entries: Vec<_> = self
                                .asset_browser
                                .filtered_entries()
                                .into_iter()
                                .cloned()
                                .collect();
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

                                        let selectable = ui
                                            .selectable_label(selected, &name)
                                            .on_hover_text(&entry.path);

                                        // Drag-and-drop support: start drag on the asset entry.
                                        if selectable.clicked() && entry.kind == AssetKind::Image {
                                            if let Err(e) = self.asset_browser.select(&entry.path) {
                                                log::warn!("asset '{}': {e}", entry.path);
                                            }
                                        }

                                        // Drag-and-drop: start drag on mouse press.
                                        if selectable.drag_started() {
                                            self.asset_browser.set_drag_source(Some(&entry.path));
                                        }

                                        // Handle drop onto this entity (if it's a sprite/texture target).
                                        if let Some(drag_source) = self.asset_browser.drag_source()
                                        {
                                            if selectable.hovered()
                                                && selectable.hovered()
                                                && entry.kind == AssetKind::Image
                                            {
                                                // We could support dropping an asset onto an entity here
                                                // to change its texture. For now, just log.
                                                log::info!(
                                                    "Drag {:?} onto {}",
                                                    drag_source,
                                                    entry.path
                                                );
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
            Tab::PrefabEditor | Tab::PrefabEditorPanel => {
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
                            ui.label(format!(
                                "Prefabs: {}",
                                self.prefab_editor.state.prefab_list().len()
                            ));
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
                            .into_iter()
                            .cloned()
                            .collect();
                        for prefab in prefabs {
                            let updated =
                                crate::editing::update_prefab_instances(self.world, &prefab);
                            if updated > 0 {
                                self.log(&format!(
                                    "[INFO] Updated {} instances of prefab '{}'",
                                    updated, prefab.name
                                ));
                            }
                        }
                    }
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

            Tab::AnimationEditor => {
                ui.vertical(|ui| {
                    if let Some(project) = self.current_project.as_mut() {
                        AnimationEditorState::render_animation_editor(
                            ui,
                            self.anim_editor,
                            &mut project.animation_clips,
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
                        crate::build_panel::BuildPanel::render(self.build_panel, ui, project);
                    } else {
                        ui.heading("Build / Export");
                        ui.add_space(8.0);
                        ui.label("Open a project to build and export.");
                    }
                });
            }
            Tab::ProjectHub => {
                let requests = std::cell::RefCell::new(Vec::new());
                crate::project_hub::render_project_hub(ui, self.project_hub, &mut |path| {
                    requests.borrow_mut().push(ProjectRequest::Open(path))
                });
                self.project_requests.extend(requests.into_inner());
            }
            Tab::ProfilerPanel => {
                ui.vertical(|ui| {
                    let gpu = self.gpu;
                    self.profiler_panel.update(gpu.renderer.profiler());
                    self.profiler_panel.render(ui, gpu.renderer.profiler());
                });
            }
            Tab::DebuggerPanel => {
                ui.vertical(|ui| {
                    if let Some(play) = &mut self.play {
                        // Use the SliceSession's debugger
                        if let Some(debugger) = play.slice.debugger_mut() {
                            self.debugger_panel.render(ui, debugger);
                        } else {
                            ui.heading("KilnScript Debugger");
                            ui.add_space(8.0);
                            ui.label("Debugger not available");
                        }
                    } else {
                        ui.heading("KilnScript Debugger");
                        ui.add_space(8.0);
                        ui.label("Start a play session to use the debugger.");
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
                    #[cfg(not(feature = "audio"))]
                    ui.label("Audio preview requires an audio-enabled build.");
                    // Connect audio editor to the slice's audio engine
                    #[cfg(feature = "audio")]
                    if let Some(play) = self.play.as_mut() {
                        let slice = &mut play.slice;
                        // Handle preview playback
                        if let Some(name) = self.audio_editor.preview_asset.clone() {
                            if self.audio_editor.preview_playing {
                                slice.host_mut().play_audio_preview(&name);
                            } else {
                                slice.host_mut().stop_audio_preview();
                            }
                        }
                        // Register all project audio assets with the slice host
                        for (name, path) in self
                            .current_project
                            .as_ref()
                            .map(|p| {
                                p.audio_assets
                                    .iter()
                                    .map(|a| (a.name.clone(), a.path.clone()))
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default()
                        {
                            slice.host_mut().register_audio(&name, &path);
                        }
                    }
                });
            }
            Tab::PhysicsEditor => {
                ui.vertical(|ui| {
                    self.physics_editor.render(ui);
                });
            }
            Tab::ShaderEditor => {
                ui.vertical(|ui| {
                    if let Some(project) = self.current_project.as_mut() {
                        crate::shader_editor::render_shader_editor(ui, self.shader_editor, project);
                    } else {
                        ui.heading("Shader Editor");
                        ui.add_space(8.0);
                        ui.label("Open a project to edit shaders.");
                    }
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
                    let mut context = kiln_plugin_api::PluginContext {
                        project: self.current_project.clone(),
                        ui: None,
                        world: None,
                        slice_session: None,
                        config: Default::default(),
                        project_root: std::env::current_dir().unwrap_or_default(),
                        temp_dir: std::env::temp_dir(),
                    };
                    self.plugin_manager.render(ui, &mut context);
                });
            }
            Tab::VersionControl => {
                ui.vertical(|ui| {
                    self.version_control.render(ui);
                });
            }
            Tab::InputRebind => {
                ui.vertical(|ui| {
                    if let Some(project) = self.current_project.as_mut() {
                        self.input_rebind.render(ui, project);
                    } else {
                        ui.heading("Input Rebinding");
                        ui.add_space(8.0);
                        ui.label("Open a project to rebind inputs.");
                    }
                });
            }
            Tab::SpriteEditor => {
                ui.vertical(|ui| {
                    if let Some(project) = self.current_project.as_mut() {
                        self.sprite_editor.render(ui, project);
                    } else {
                        ui.heading("Sprite Editor");
                        ui.add_space(8.0);
                        ui.label("Open a project to edit sprites.");
                    }
                });
            }
            Tab::Hierarchy => {
                let hierarchy_size = self.world_size();
                ui.vertical(|ui| {
                    if let Some(project) = self.current_project.as_mut() {
                        let scene_id = self.scenes.active_id().unwrap_or_default().to_owned();
                        // Render hierarchy panel and handle actions.
                        if let Some(action) = crate::hierarchy::render_hierarchy_panel(
                            ui,
                            self.hierarchy_panel,
                            project,
                            &scene_id,
                        ) {
                            if let Some(edit) = self.transform_edit.take() {
                                edit.commit(&mut self.scenes.history);
                            }
                            self.gizmo.drag = None;
                            match action {
                                crate::hierarchy::HierarchyAction::SelectEntity(project_id) => {
                                    // Sync selection: project entity ID -> hecs Entity
                                    let selected_id = self
                                        .hierarchy_panel
                                        .selected_entity_id()
                                        .unwrap_or(&project_id);
                                    *self.selected_entity =
                                        self.entity_id_to_hecs.get(selected_id).copied();
                                }
                                crate::hierarchy::HierarchyAction::RemoveEntity(source_id) => {
                                    self.project_requests.push(ProjectRequest::Entity(
                                        crate::entity_command::EntityCommand {
                                            scene_id: scene_id.clone(),
                                            edit: crate::entity_command::EntityEdit::Delete {
                                                entity_id: source_id,
                                            },
                                        },
                                    ));
                                }
                                crate::hierarchy::HierarchyAction::AddEntity(name) => {
                                    let size = hierarchy_size;
                                    self.project_requests.push(ProjectRequest::Entity(
                                        crate::entity_command::EntityCommand {
                                            scene_id: scene_id.clone(),
                                            edit: crate::entity_command::EntityEdit::Create {
                                                name,
                                                position: [size.0 / 2.0, size.1 / 2.0],
                                            },
                                        },
                                    ));
                                }
                                crate::hierarchy::HierarchyAction::Reparent {
                                    child_id,
                                    parent_id,
                                } => {
                                    self.project_requests.push(ProjectRequest::Entity(
                                        crate::entity_command::EntityCommand {
                                            scene_id: scene_id.clone(),
                                            edit: crate::entity_command::EntityEdit::Reparent {
                                                entity_id: child_id,
                                                parent_id,
                                            },
                                        },
                                    ));
                                }
                                crate::hierarchy::HierarchyAction::DuplicateEntity(source_id) => {
                                    self.project_requests.push(ProjectRequest::Entity(
                                        crate::entity_command::EntityCommand {
                                            scene_id: scene_id.clone(),
                                            edit: crate::entity_command::EntityEdit::Duplicate {
                                                entity_id: source_id,
                                            },
                                        },
                                    ));
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

    fn clear_background(&self, tab: &Self::Tab) -> bool {
        !matches!(tab, Tab::SceneView)
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
            Tab::PrefabEditor | Tab::PrefabEditorPanel => "Prefab Editor".into(),
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
            Tab::ShaderEditor => "Shader Editor".into(),
            Tab::UiLayoutEditor => "UI Layout Editor".into(),
            Tab::Preferences => "Preferences".into(),
            Tab::PluginManagerPanel => "Plugin Manager".into(),
            Tab::VersionControl => "Version Control".into(),
            Tab::InputRebind => "Input Rebinding".into(),
            Tab::SpriteEditor => "Sprite Editor".into(),
        }
    }
}

impl<'a> TabViewer<'a> {
    fn frame_selection(&mut self) {
        let mut bounds: Option<[f32; 4]> = None;
        for id in self.hierarchy_panel.selected_ids.clone() {
            let Some(entity) = self.entity_id_to_hecs.get(&id).copied() else {
                continue;
            };
            let Ok(transform) = self
                .world
                .world_ref()
                .get::<&kiln_runtime::GlobalTransform>(entity)
            else {
                continue;
            };
            let entity_bounds = if let Ok(sprite) = self.world.world_ref().get::<&Sprite>(entity) {
                scene_view::transformed_sprite_bounds(transform.0, sprite.size)
            } else {
                [
                    transform.0.position.x,
                    transform.0.position.y,
                    transform.0.position.x,
                    transform.0.position.y,
                ]
            };
            bounds = Some(match bounds {
                Some([min_x, min_y, max_x, max_y]) => [
                    min_x.min(entity_bounds[0]),
                    min_y.min(entity_bounds[1]),
                    max_x.max(entity_bounds[2]),
                    max_y.max(entity_bounds[3]),
                ],
                None => entity_bounds,
            });
        }
        let Some([min_x, min_y, max_x, max_y]) = bounds else {
            return;
        };
        self.camera_state.target = None;
        let base = scene_view::world_camera(self.scene_world_size.0, self.scene_world_size.1);
        let width = (max_x - min_x).max(1.0) * 1.2;
        let height = (max_y - min_y).max(1.0) * 1.2;
        self.camera_state.zoom = (base.viewport_size[0] / width)
            .min(base.viewport_size[1] / height)
            .clamp(0.1, 10.0);
        self.camera_state.position = [
            (min_x + max_x) * 0.5 - base.viewport_size[0] / self.camera_state.zoom * 0.5,
            (min_y + max_y) * 0.5 - base.viewport_size[1] / self.camera_state.zoom * 0.5,
        ];
    }
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
    }
}
