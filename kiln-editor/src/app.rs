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
    PostPipeline, RenderTier, SpriteRenderer,
};
use kiln_runtime::{GameWorld, SourceId, Sprite, Tilemap};
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
use crate::assets::AssetBrowser;
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
    Export(crate::build_panel::BuildConfig),
    RefreshVisuals,
    PausePlay,
    StepPlay,
    RestartPlay,
    PlayProject,
    ApplyPlayTransforms,
    TransitionPlay(String),
    Component(ComponentCommand),
    Entity(crate::entity_command::EntityCommand),
    ApplyPrefabInstance(String),
    UndoPrefabApply,
    RedoPrefabApply,
    RenameAsset { old_path: String, new_path: String },
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

#[derive(Clone)]
struct PrefabApplySnapshot {
    project_path: PathBuf,
    prefabs: HashMap<String, kiln_core::Prefab>,
    scenes: HashMap<String, Scene>,
}

#[derive(Clone)]
struct PrefabApplyHistoryEntry {
    before: PrefabApplySnapshot,
    after: PrefabApplySnapshot,
}

fn prefab_snapshots_match(left: &PrefabApplySnapshot, right: &PrefabApplySnapshot) -> bool {
    left.project_path == right.project_path
        && serde_json::to_value((&left.prefabs, &left.scenes)).ok()
            == serde_json::to_value((&right.prefabs, &right.scenes)).ok()
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
    clock: kiln_runtime::play_clock::PlayClock,
    last_tick: Instant,
    camera_controller: kiln_runtime::camera::CameraController,
    game_camera: kiln_render::camera::Camera,
}

/// The world size the scene view falls back to when no project is open.
const DEFAULT_WORLD_SIZE: (f32, f32) = (1280.0, 720.0);

/// The spacing of the scene view's grid, in world units.
const GRID_SPACING: f32 = 32.0;

/// The colour of the scene view's grid lines.
const GRID_COLOR: [f32; 4] = [0.016, 0.016, 0.016, 1.0];

/// The colour behind the scene view.
const BACKGROUND_COLOR: [f32; 4] = [0.01, 0.01, 0.01, 1.0];

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
    FocusPanel(Tab),
    ResetLayout,
}

pub struct EditorApp {
    gpu: Option<GpuState>,
    egui_winit: egui_winit::State,
    dock_state: egui_dock::DockState<Tab>,
    shell: crate::editor_shell::EditorShell,
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
    /// Measured frame rate displayed in the editor status bar.
    fps: f32,
    /// The time the previous frame was rendered, used to measure the frame
    /// rate.
    last_frame: Instant,
    /// The number of frames rendered so far.
    frame_count: u32,
    /// The history of scene edits, used by undo and redo.
    scenes: SceneWorkspace,
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
    #[cfg(feature = "gamepad")]
    gamepad: Option<kiln_input::GamepadManager>,
    preferences: PreferencesPanel,
    prefab_editor: PrefabEditorPanel,
    prefab_apply_undo: Vec<PrefabApplyHistoryEntry>,
    prefab_apply_redo: Vec<PrefabApplyHistoryEntry>,
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
    asset_rename: Option<(String, String)>,
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
    GameView,
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

fn play_input_edges(
    input: &egui::InputState,
    pixels_per_point: f32,
    text_input: bool,
) -> (Vec<String>, Vec<([f32; 2], bool)>) {
    let mut keys = Vec::new();
    let mut points = Vec::new();
    for event in &input.events {
        match event {
            egui::Event::Key {
                key,
                pressed: true,
                repeat: false,
                modifiers,
                ..
            } if !text_input => {
                let key = match key {
                    egui::Key::ArrowUp => "ArrowUp".into(),
                    egui::Key::ArrowDown => "ArrowDown".into(),
                    egui::Key::ArrowLeft => "ArrowLeft".into(),
                    egui::Key::ArrowRight => "ArrowRight".into(),
                    egui::Key::Tab if modifiers.shift => "BackTab".into(),
                    _ => format!("{key:?}"),
                };
                keys.push(key);
            }
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                ..
            } => {
                points.push((
                    [pos.x * pixels_per_point, pos.y * pixels_per_point],
                    *pressed,
                ));
            }
            _ => {}
        }
    }
    (keys, points)
}

fn default_dock_layout() -> egui_dock::DockState<Tab> {
    let mut dock =
        egui_dock::DockState::new(vec![Tab::SceneView, Tab::GameView, Tab::AnimationEditor]);
    let tree = dock.main_surface_mut();
    let [workspace, _] = tree.split_below(
        egui_dock::NodeIndex::root(),
        0.75,
        vec![Tab::Assets, Tab::Console, Tab::ProfilerPanel],
    );
    let [center, _] = tree.split_left(workspace, 0.167, vec![Tab::Hierarchy]);
    tree.split_right(center, 0.75, vec![Tab::Inspector]);
    dock
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
    ui_fonts: kiln_ui::authored::UiFonts,
    /// Post-processing framebuffer for the scene view (separate from main renderer).
    scene_view_post_framebuffer: Option<PostFramebuffer>,
    /// Post-processing pipeline for the scene view.
    scene_view_post_pipeline: Option<PostPipeline>,
}

/// UI designer state for the visual UI editor.
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

/// Start a game session for a project.
///
/// The scene's script is read from the project directory, compiled and
/// paired with a world holding the scene. The player and the editor run the
/// same session, so a project behaves the same in both.
fn start_play_session(
    project: &Project,
    scene_id: &str,
    root: &Path,
    textures: &SceneTextures,
) -> Result<(kiln_runtime::SliceSession, String, GameWorld), String> {
    let mut slice = kiln_runtime::SliceSession::with_scene_sources(project, scene_id, |path| {
        std::fs::read_to_string(root.join(path)).map_err(|error| error.to_string())
    })?;

    #[cfg(feature = "audio")]
    for error in slice.host_mut().configure_audio(project, |path| {
        std::fs::read(root.join(path)).map_err(|error| error.to_string())
    }) {
        log::error!("{error}");
    }
    slice.host_mut().set_save_dir(&root.join(".kiln-save"));
    let (world, _spawned) = spawn_editor_scene(project, scene_id, textures);
    Ok((slice, scene_id.to_owned(), world))
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
        crate::editing::attach_prefab_instances(&mut world, scene);
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

impl EditorApp {
    pub fn new(event_loop: &EventLoop<()>) -> Result<Self, EditorError> {
        let egui_ctx = egui::Context::default();

        crate::editor_shell::apply_theme(&egui_ctx);

        let egui_winit = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            event_loop,
            None,
            None,
            None,
        );
        let dock_state = default_dock_layout();

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
            shell: crate::editor_shell::EditorShell::default(),
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
            #[cfg(feature = "gamepad")]
            gamepad: kiln_input::GamepadManager::new().ok(),
            preferences: PreferencesPanel::new(),
            prefab_editor: PrefabEditorPanel::new(),
            prefab_apply_undo: Vec::new(),
            prefab_apply_redo: Vec::new(),
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
            asset_rename: None,
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
            let sr = SpriteRenderer::new(device, queue, renderer.scene_format());
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
            ui_fonts: Default::default(),
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

    /// Save a project copy while preserving relative assets, committing state only on success.
    pub fn save_project_as(&mut self, path: PathBuf) -> Result<(), String> {
        self.finish_transform_edit();
        let source = self
            .project_path
            .as_ref()
            .ok_or("Open or create a project before Save As")?;
        let project = self.project_to_save()?;
        let path = crate::project_files::save_as(&project, source, &path)?;
        self.prefab_apply_undo.clear();
        self.prefab_apply_redo.clear();
        let _ = crate::recovery::clear(&path);
        self.project_path = Some(path.clone());
        self.project_path_input = path.to_string_lossy().into_owned();
        self.scenes.mark_saved(&project);
        self.lifecycle.mark_saved(&project);
        self.current_project = Some(project);
        if let Some(root) = path.parent() {
            self.audio_editor.stop_preview();
            self.project_root = Some(root.to_path_buf());
            self.asset_browser.scan(root);
        }
        if let Some(project) = self.current_project.as_ref() {
            self.project_hub
                .add_recent(path.clone(), project.name.clone());
        }
        self.upload_scene_textures();
        self.project_error = None;
        Ok(())
    }

    /// Load a project from a file path.
    pub fn load_project(&mut self, path: PathBuf) -> Result<(), String> {
        let loaded = crate::project_files::load(&path)?;
        self.prefab_apply_undo.clear();
        self.prefab_apply_redo.clear();
        self.finish_transform_edit();
        if self.play.is_some() {
            self.stop_play();
        }
        self.is_playing = false;
        let project = loaded.project;
        let path = loaded.path;
        self.prefab_editor
            .state
            .replace_prefabs(project.prefabs.clone());
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
            self.audio_editor.stop_preview();
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
        let scene_id = self.scenes.active_id().ok_or("No active scene")?.to_owned();
        self.start_play_scene(&scene_id)
    }

    fn start_play_scene(&mut self, scene_id: &str) -> Result<(), String> {
        self.shell.set_preview(false);
        self.finish_transform_edit();
        self.capture_active_scene()?;
        self.upload_scene_textures();
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
        self.audio_editor.stop_preview();
        if let Some(gpu) = self.gpu.as_mut() {
            gpu.scene_view_overlay_renderer.clear_asset_textures();
            let (fonts, errors) = kiln_ui::authored::UiFonts::load(
                &project.ui,
                |path| std::fs::read(root.join(path)).map_err(|e| e.to_string()),
                |id, size, rgba| {
                    gpu.scene_view_overlay_renderer
                        .create_texture_rgba_with_options(
                            gpu.renderer.device(),
                            gpu.renderer.queue(),
                            id,
                            size,
                            rgba,
                            [false, false],
                        )
                },
            );
            gpu.ui_fonts = fonts;
            for error in errors {
                self.console_log.push(format!("[ERROR] {error}"));
            }
        }
        let (slice, scene_id, world) = start_play_session(&project, scene_id, &root, &textures)?;
        #[cfg(feature = "audio")]
        let slice = {
            let mut slice = slice;
            if !project.audio_assets.is_empty() {
                if let Err(error) = slice.host_mut().open_audio_output() {
                    self.console_log
                        .push(format!("[WARN] Audio output unavailable: {error}"));
                }
            }
            slice
        };
        let mut camera_controller = kiln_runtime::camera::CameraController::default();
        let settings = project.cameras.get(&scene_id).cloned().unwrap_or_default();
        let game_camera = camera_controller.update(
            &settings,
            &world,
            [project.window.width as f32, project.window.height as f32],
            0.0,
        )?;

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
            clock: kiln_runtime::play_clock::PlayClock::default(),
            last_tick: Instant::now(),
            camera_controller,
            game_camera,
        });
        self.is_playing = true;

        Ok(())
    }

    /// Stop playing and return to editing the scene.
    fn stop_play(&mut self) {
        let _ = self.stop_play_with_progress();
    }

    fn stop_play_with_progress(&mut self) -> Option<kiln_runtime::GameProgress> {
        if let Some(play) = self.play.as_mut() {
            if let Err(error) = play.slice.stop(&mut self.world) {
                self.console_log
                    .push(format!("[ERROR] Stop callback: {error}"));
            }
        }
        let progress = self.play.as_ref().map(|play| play.slice.game_progress());
        self.play = None;
        self.is_playing = false;
        self.reset_scene_view();
        progress
    }

    fn apply_play_transforms(&mut self) -> Result<(), String> {
        let play = self.play.as_ref().ok_or("No running game")?;
        if !play.clock.paused() {
            return Err("Pause the game before applying runtime transforms".into());
        }
        if self.scenes.active_id() != Some(play.scene_id.as_str()) {
            return Err(
                "Runtime transforms can only be applied when playing the active scene".into(),
            );
        }
        let project = self.current_project.as_ref().ok_or("No project loaded")?;
        let authored = project
            .scenes
            .get(&play.scene_id)
            .ok_or("Authored scene is missing")?
            .clone();
        let runtime = crate::scene_document::snapshot_for_scene(
            &self.world,
            &self.entity_textures,
            &self.entity_roles,
            Some(project),
            Some(&play.scene_id),
        );
        let applied = crate::play_changes::apply_transforms(&authored, &runtime)?;
        self.stop_play();
        self.scenes.history.record(authored);
        self.apply_scene(applied)
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
        if self.play.is_some() {
            return base;
        }
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
        if let Some(play) = &self.play {
            return play.game_camera.clone();
        }
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
    fn update_play(&mut self) {
        if !self.egui_winit.egui_ctx().input(|i| i.focused) {
            return;
        }
        let settings = self
            .play
            .as_ref()
            .and_then(|play| {
                self.current_project
                    .as_ref()
                    .and_then(|project| project.cameras.get(&play.scene_id))
            })
            .cloned()
            .unwrap_or_default();
        let size = self
            .current_project
            .as_ref()
            .map(|project| [project.window.width as f32, project.window.height as f32])
            .unwrap_or([1280.0, 720.0]);
        let axis = self.play_axis();
        let mouse_mapping = self.scene_view_rect.and_then(|rect| {
            let ppp = self.egui_winit.egui_ctx().pixels_per_point();
            let viewport = scene_view::camera_viewport(rect, ppp, &self.scene_camera());
            self.egui_winit
                .egui_ctx()
                .input(|input| input.pointer.hover_pos())
                .map(|point| {
                    (
                        [point.x * ppp, point.y * ppp],
                        [
                            viewport.x as f32,
                            viewport.y as f32,
                            viewport.width as f32,
                            viewport.height as f32,
                        ],
                    )
                })
        });
        let text_input = self.egui_winit.egui_ctx().wants_keyboard_input();
        let mut keys = self.egui_winit.egui_ctx().input(|input| {
            if text_input {
                Vec::new()
            } else {
                let mut keys = input
                    .keys_down
                    .iter()
                    .map(|key| format!("{key:?}"))
                    .collect::<Vec<_>>();
                if input.modifiers.shift {
                    keys.push("Shift".into());
                }
                keys
            }
        });
        #[cfg(feature = "gamepad")]
        if !text_input {
            if let Some(gamepad) = &mut self.gamepad {
                let _ = gamepad.update();
                keys.extend(gamepad.pressed_tokens());
            }
        }
        // Egui names arrows Up/Down/Left/Right; scripts and UI use winit tokens.
        for key in &mut keys {
            *key = match key.as_str() {
                "Up" => "ArrowUp".into(),
                "Down" => "ArrowDown".into(),
                "Left" => "ArrowLeft".into(),
                "Right" => "ArrowRight".into(),
                _ => key.clone(),
            };
        }
        let pointer_down = self
            .egui_winit
            .egui_ctx()
            .input(|i| i.pointer.primary_down());
        let ui_area = self.scene_view_rect.map(|rect| {
            let ppp = self.egui_winit.egui_ctx().pixels_per_point();
            let view = scene_view::camera_viewport(rect, ppp, &self.scene_camera());
            [
                view.x as f32,
                view.y as f32,
                view.width as f32,
                view.height as f32,
            ]
        });
        // Context::input holds egui's context lock. Query display scale before
        // entering it; re-entering Context here freezes the first Play frame.
        let pixels_per_point = self.egui_winit.egui_ctx().pixels_per_point();
        let (key_edges, pointer_edges) = self
            .egui_winit
            .egui_ctx()
            .input(|input| play_input_edges(input, pixels_per_point, text_input));
        let EditorApp {
            world,
            play,
            project_requests,
            current_project,
            project_root,
            gpu,
            ..
        } = self;
        let Some(play) = play.as_mut() else {
            return;
        };
        if play.clock.paused() {
            play.slice.host_mut().reset_ui_input();
        } else {
            for key in key_edges {
                play.slice.host_mut().queue_ui_key(&key);
            }
            if let Some(area) = ui_area {
                for (point, down) in pointer_edges {
                    play.slice.host_mut().queue_ui_pointer(area, point, down);
                }
            }
        }
        let elapsed = play.last_tick.elapsed().as_secs_f64();
        play.last_tick = Instant::now();
        play.slice.host_mut().set_pressed_keys(&keys);
        for _ in 0..play.clock.advance(elapsed) {
            if let Some((point, viewport)) = mouse_mapping {
                play.slice
                    .host_mut()
                    .set_ui_pointer(viewport, Some(point), pointer_down);
                if let Some(position) = play.game_camera.screen_to_world(point, viewport) {
                    play.slice
                        .host_mut()
                        .set_mouse_world(position.map(|value| value as f64));
                }
            }
            if mouse_mapping.is_none() {
                play.slice
                    .host_mut()
                    .set_ui_pointer([0.0, 0.0, size[0], size[1]], None, false);
            }
            if let Err(error) = play
                .slice
                .step(world, axis, kiln_runtime::play_clock::PLAY_STEP)
            {
                play.error = Some(format!("Scene '{}': {error}", play.scene_id));
                play.clock.set_paused(true);
                break;
            }
            if let (Some(project), Some(root), Some(gpu)) = (
                current_project.as_ref(),
                project_root.as_ref(),
                gpu.as_ref(),
            ) {
                match play.slice.service_saves(
                    world,
                    project,
                    |path| std::fs::read_to_string(root.join(path)).map_err(|e| e.to_string()),
                    |path| {
                        gpu.scene_textures
                            .get(path)
                            .map(|t| (t.id, [t.width, t.height]))
                    },
                ) {
                    Ok(true) => {
                        play.scene_id = play.slice.scene_id().unwrap_or(&play.scene_id).to_owned();
                        play.camera_controller = Default::default();
                        let settings = project
                            .cameras
                            .get(&play.scene_id)
                            .cloned()
                            .unwrap_or_default();
                        if let Ok(camera) =
                            play.camera_controller.update(&settings, world, size, 0.0)
                        {
                            play.game_camera = camera;
                        }
                        return;
                    }
                    Ok(false) => {}
                    Err(error) => log::warn!("Gameplay storage: {error}"),
                }
            }
            play.error = None;
            let _ = play.slice.take_ui_commands();
            let _ = kiln_runtime::scene::update_global_transforms(world);
            match play.camera_controller.update(
                &settings,
                world,
                size,
                kiln_runtime::play_clock::PLAY_STEP as f32,
            ) {
                Ok(camera) => play.game_camera = camera,
                Err(error) => {
                    play.error = Some(error);
                    play.clock.set_paused(true);
                    break;
                }
            }
            if let Some(id) = play.slice.take_scene_request() {
                project_requests.push(ProjectRequest::TransitionPlay(id));
                play.clock.set_paused(true);
                break;
            }
        }
    }

    /// The movement axis from the keys held down in the editor.
    fn play_axis(&self) -> (f64, f64) {
        let ctx = self.egui_winit.egui_ctx();
        if ctx.wants_keyboard_input() {
            return (0.0, 0.0);
        }
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
        if matches!(
            command.edit,
            crate::entity_command::EntityEdit::CreateSprite { .. }
        ) {
            self.apply_scene_with_texture_refresh(scene)?;
        } else {
            self.apply_scene(scene)?;
        }
        if deleted.is_some_and(|id| self.hierarchy_panel.selected_entity_id() == Some(id)) {
            self.hierarchy_panel.clear_selection();
        }
        if let Some(id) = selected {
            self.selected_entity = self.entity_id_to_hecs.get(&id).copied();
            self.hierarchy_panel.select(&id);
        }
        Ok(())
    }

    fn prefab_apply_snapshot(&self) -> Result<PrefabApplySnapshot, String> {
        let project = self.current_project.as_ref().ok_or("No project loaded")?;
        let project_path = self
            .project_path
            .clone()
            .ok_or("Save or open a project before applying a prefab instance")?;
        Ok(PrefabApplySnapshot {
            project_path,
            prefabs: project.prefabs.clone(),
            scenes: project.scenes.clone(),
        })
    }

    fn apply_prefab_instance(&mut self, entity_id: &str) -> Result<(), String> {
        self.require_edit_mode()?;
        self.finish_transform_edit();
        let Some(scene_id) = self.scenes.active_id().map(str::to_owned) else {
            return Err("Open a scene before applying a prefab instance".into());
        };
        let scene = self.scene_from_world();
        let index = scene
            .entities
            .iter()
            .position(|entity| entity.id == entity_id)
            .ok_or("Prefab instance is missing from the active scene")?;
        if !scene.entities[index]
            .components
            .iter()
            .any(|component| matches!(component, kiln_core::Component::PrefabInstance { .. }))
        {
            return Err("Selected entity is not a prefab instance".into());
        }
        let mut before = self.prefab_apply_snapshot()?;
        before.scenes.insert(scene_id.clone(), scene);
        let project = self.current_project.as_mut().ok_or("No project loaded")?;
        let updated_prefab =
            crate::editing::apply_prefab_instance_to_definition(project, &scene_id, entity_id)?;
        self.prefab_apply_redo.clear();
        self.prefab_editor
            .state
            .replace_prefabs(project.prefabs.clone());
        crate::editing::update_prefab_instances(&mut self.world, &updated_prefab);
        let updated_scene = project
            .scenes
            .get(&scene_id)
            .cloned()
            .ok_or("Active scene is missing")?;
        self.apply_scene(updated_scene)?;
        let after = self.prefab_apply_snapshot()?;
        self.prefab_apply_undo
            .push(PrefabApplyHistoryEntry { before, after });
        Ok(())
    }

    fn restore_prefab_snapshot(&mut self, snapshot: PrefabApplySnapshot) -> Result<(), String> {
        if self.project_path.as_ref() != Some(&snapshot.project_path) {
            return Err("Prefab apply history belongs to a different project".into());
        }
        let Some(project) = self.current_project.as_mut() else {
            return Err("No project loaded".into());
        };
        project.prefabs = snapshot.prefabs;
        project.scenes = snapshot.scenes;
        self.prefab_editor
            .state
            .replace_prefabs(project.prefabs.clone());
        let scene_id = self.scenes.active_id().ok_or("No active scene")?;
        let scene = project
            .scenes
            .get(scene_id)
            .cloned()
            .ok_or("Active scene is missing")?;
        self.apply_scene(scene)
    }

    fn undo_prefab_apply(&mut self) -> Result<(), String> {
        let entry = self
            .prefab_apply_undo
            .last()
            .cloned()
            .ok_or("No prefab apply to undo")?;
        let current = self.prefab_apply_snapshot()?;
        if !prefab_snapshots_match(&current, &entry.after) {
            return Err(
                "Project content changed after Apply Overrides; undo was not applied".into(),
            );
        }
        self.restore_prefab_snapshot(entry.before.clone())?;
        self.prefab_apply_undo.pop();
        self.prefab_apply_redo.push(entry);
        Ok(())
    }

    fn redo_prefab_apply(&mut self) -> Result<(), String> {
        let entry = self
            .prefab_apply_redo
            .last()
            .cloned()
            .ok_or("No prefab apply to redo")?;
        let current = self.prefab_apply_snapshot()?;
        if !prefab_snapshots_match(&current, &entry.before) {
            return Err(
                "Project content changed after undoing Apply Overrides; redo was not applied"
                    .into(),
            );
        }
        self.restore_prefab_snapshot(entry.after.clone())?;
        self.prefab_apply_redo.pop();
        self.prefab_apply_undo.push(entry);
        Ok(())
    }

    fn rename_project_asset(&mut self, old_path: &str, new_path: &str) -> Result<(), String> {
        self.finish_transform_edit();
        if self.current_project.is_none() {
            return Err("Open a project before moving an asset".into());
        }
        let active = self.scenes.active_id().map(str::to_owned);
        let current_scene = active.as_deref().map(|_| self.scene_from_world());
        let root = self
            .asset_browser
            .root()
            .ok_or("No project asset root")?
            .to_path_buf();
        let old_cached_path = root.join(old_path).canonicalize().ok();
        let new_full_path = root.join(new_path);
        self.asset_browser.rename_asset(old_path, new_path)?;
        let mut cache = kiln_assets::AssetCache::new(root.join(".kiln-cache"));
        if let Err(error) = cache.load() {
            self.console_log.push(format!(
                "[WARN] Asset moved, but import cache could not be loaded: {error}"
            ));
        } else {
            if let Some(old) = old_cached_path {
                if let Err(error) = cache.rename_cached_path(&old, &new_full_path) {
                    self.console_log.push(format!(
                        "[WARN] Asset moved, but cache tracking could not be updated: {error}"
                    ));
                }
            }
            let _ = cache.import(&new_full_path);
            if let Err(error) = cache.save() {
                self.console_log.push(format!(
                    "[WARN] Asset moved, but import cache could not be saved: {error}"
                ));
            }
        }
        let project = self.current_project.as_mut().ok_or("No project loaded")?;
        if let (Some(scene_id), Some(scene)) = (active.as_ref(), current_scene) {
            project.scenes.insert(scene_id.clone(), scene);
        }
        let updated = crate::editing::rewrite_asset_references(project, old_path, new_path);
        if let Some(id) = project.asset_ids.remove(old_path) {
            project.asset_ids.insert(new_path.to_owned(), id);
        }
        self.prefab_editor
            .state
            .replace_prefabs(project.prefabs.clone());
        if let Some(scene_id) = active {
            if let Some(scene) = project.scenes.get(&scene_id).cloned() {
                self.apply_scene_with_texture_refresh(scene)?;
            }
        } else {
            self.upload_scene_textures();
        }
        self.console_log.push(format!(
            "[INFO] Moved asset {old_path} to {new_path} and updated {updated} references"
        ));
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
        let Some(play) = self.play.as_mut() else {
            return;
        };
        match play.slice.save_game(&self.world, 0) {
            Ok(()) => self.console_log.push("[INFO] Game saved".into()),
            Err(error) => self
                .console_log
                .push(format!("[ERROR] Could not save game: {error}")),
        }
    }

    /// Restore a validated candidate world without modifying authored project data.
    fn load_game(&mut self) {
        let (Some(play), Some(project), Some(root), Some(gpu)) = (
            self.play.as_mut(),
            self.current_project.as_ref(),
            self.project_root.as_ref(),
            self.gpu.as_ref(),
        ) else {
            return;
        };
        match play.slice.load_game(
            &mut self.world,
            0,
            project,
            |path| std::fs::read_to_string(root.join(path)).map_err(|e| e.to_string()),
            |path| {
                gpu.scene_textures
                    .get(path)
                    .map(|t| (t.id, [t.width, t.height]))
            },
        ) {
            Ok(()) => {
                play.scene_id = play.slice.scene_id().unwrap_or(&play.scene_id).to_owned();
                play.camera_controller = Default::default();
                let settings = project
                    .cameras
                    .get(&play.scene_id)
                    .cloned()
                    .unwrap_or_default();
                if let Ok(camera) = play.camera_controller.update(
                    &settings,
                    &self.world,
                    [project.window.width as f32, project.window.height as f32],
                    0.0,
                ) {
                    play.game_camera = camera;
                }
                self.console_log.push("[INFO] Game loaded".into());
            }
            Err(error) => self
                .console_log
                .push(format!("[ERROR] Could not load game: {error}")),
        }
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
                        .with_inner_size(winit::dpi::LogicalSize::new(1440u32, 900u32))
                        .with_min_inner_size(winit::dpi::LogicalSize::new(1000u32, 640u32)),
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
            WindowEvent::Focused(false) => {
                if let Some(play) = self.play.as_mut() {
                    play.slice.host_mut().reset_ui_input();
                }
                self.finish_transform_edit();
                if let Some(play) = self.play.as_mut() {
                    play.clock.set_paused(true);
                }
            }
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
            if !response.consumed && !self.shell.quick_actions.open {
                match key {
                    Key::Character(key) if ["q", "w", "e", "r"].contains(&key.as_str()) => {
                        self.finish_transform_edit();
                        self.gizmo.mode = match key.as_str() {
                            "q" => GizmoMode::None,
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
        self.build_panel.poll();
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
                ProjectRequest::Export(config) => {
                    let result = self.save_project().and_then(|_| {
                        let path = self.project_path.as_ref().ok_or("Save the project first")?;
                        let project = self
                            .current_project
                            .as_ref()
                            .ok_or("Open a project first")?;
                        self.build_panel.start(config, path, &project.id)
                    });
                    if let Err(error) = &result {
                        self.build_panel.fail(error.clone());
                    }
                    result
                }
                ProjectRequest::RefreshVisuals => self.require_edit_mode().and_then(|_| {
                    let scene = self
                        .scenes
                        .active_id()
                        .and_then(|id| self.current_project.as_ref()?.scenes.get(id))
                        .cloned();
                    if let Some(scene) = scene {
                        let refresh = self
                            .gpu
                            .as_ref()
                            .zip(self.current_project.as_ref())
                            .is_some_and(|(gpu, project)| {
                                gpu.scene_textures.needs_refresh(project)
                            });
                        if refresh {
                            self.apply_scene_with_texture_refresh(scene)
                        } else {
                            self.apply_scene(scene)
                        }
                    } else {
                        Ok(())
                    }
                }),
                ProjectRequest::ApplyPlayTransforms => self.apply_play_transforms(),
                ProjectRequest::TransitionPlay(id) => {
                    if self
                        .current_project
                        .as_ref()
                        .is_some_and(|project| project.scenes.contains_key(&id))
                    {
                        let progress = self.stop_play_with_progress();
                        self.start_play_scene(&id).and_then(|_| {
                            if let (Some(progress), Some(play)) = (progress, self.play.as_mut()) {
                                play.slice.set_game_progress(progress)?;
                            }
                            Ok(())
                        })
                    } else {
                        Err(format!("Unknown scene transition '{id}'"))
                    }
                }
                ProjectRequest::PausePlay => {
                    if let Some(play) = self.play.as_mut() {
                        play.clock.set_paused(!play.clock.paused());
                        play.last_tick = Instant::now();
                    }
                    Ok(())
                }
                ProjectRequest::StepPlay => {
                    if let Some(play) = self.play.as_mut() {
                        play.clock.request_step();
                    }
                    Ok(())
                }
                ProjectRequest::RestartPlay => {
                    if let Some(scene_id) = self.play.as_ref().map(|play| play.scene_id.clone()) {
                        self.stop_play();
                        self.start_play_scene(&scene_id)
                    } else {
                        Ok(())
                    }
                }
                ProjectRequest::PlayProject => {
                    if self.play.is_some() {
                        self.stop_play();
                    }
                    match self
                        .current_project
                        .as_ref()
                        .and_then(Project::default_scene_id)
                    {
                        Some(id) => self.start_play_scene(&id),
                        None => Err("The project has no startup scene".into()),
                    }
                }
                ProjectRequest::Component(command) => self.apply_component_command(command),
                ProjectRequest::Entity(command) => self.apply_entity_command(command),
                ProjectRequest::ApplyPrefabInstance(entity_id) => {
                    self.apply_prefab_instance(&entity_id)
                }
                ProjectRequest::UndoPrefabApply => self.undo_prefab_apply(),
                ProjectRequest::RedoPrefabApply => self.redo_prefab_apply(),
                ProjectRequest::RenameAsset { old_path, new_path } => {
                    self.rename_project_asset(&old_path, &new_path)
                }
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
            self.update_play();
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

        if !project_blocked
            && ctx.input_mut(|input| {
                input.consume_shortcut(&egui::KeyboardShortcut::new(
                    egui::Modifiers::COMMAND,
                    egui::Key::K,
                ))
            })
        {
            self.shell.quick_actions.toggle();
        }
        if project_blocked {
            self.shell.quick_actions.open = false;
        }

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
                        self.dock_state = default_dock_layout();
                        ui.close_menu();
                    }
                });

                if ui
                    .button("Quick Actions  Ctrl+K")
                    .on_hover_text("Find commands and jump to panels")
                    .clicked()
                {
                    self.shell.quick_actions.toggle();
                }
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
                    ui.weak(format!(
                        "Kiln — {}",
                        self.current_project
                            .as_ref()
                            .map(|p| p.name.as_str())
                            .unwrap_or("Untitled")
                    ));
                    if project_dirty {
                        ui.label("Unsaved changes");
                    }
                });
            });
        });

        if !self.show_welcome {
            egui::TopBottomPanel::top("authoring_toolbar")
                .exact_height(38.0)
                .frame(
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(31, 34, 40))
                        .inner_margin(egui::Margin::symmetric(8.0, 4.0)),
                )
                .show(ctx, |ui| {
                    if project_blocked {
                        ui.disable();
                    }
                    let previous_mode = self.gizmo.mode;
                    let actions = self.shell.toolbar(
                        ui,
                        &mut self.gizmo.mode,
                        &mut self.grid_snap_enabled,
                        &mut self.grid_snap_size,
                        self.is_playing,
                        self.play.as_ref().is_some_and(|play| play.clock.paused()),
                    );
                    if self.gizmo.mode != previous_mode {
                        if let Some(edit) = self.transform_edit.take() {
                            edit.commit(&mut self.scenes.history);
                        }
                        self.gizmo.drag = None;
                    }
                    if actions.toggle_play {
                        self.play_toggle_requested = true;
                    }
                    if actions.pause {
                        self.project_requests.push(ProjectRequest::PausePlay);
                    }
                    if actions.step {
                        self.project_requests.push(ProjectRequest::StepPlay);
                    }
                    if actions.reset_layout {
                        self.dock_state = default_dock_layout();
                    }
                    if actions.show_scene {
                        if let Some(location) = self.dock_state.find_tab(&Tab::SceneView) {
                            self.dock_state.set_active_tab(location);
                        } else {
                            self.dock_state.push_to_focused_leaf(Tab::SceneView);
                        }
                    }
                });
            show_status_bar(ctx, fps, entity_count, tier, is_playing, project_dirty);
        }

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
                    dock_style.tab_bar.bg_fill = egui::Color32::from_gray(36);
                    dock_style.tab_bar.height = 28.0;
                    dock_style.tab.tab_body.bg_fill = egui::Color32::from_gray(43);
                    dock_style.dock_area_padding = Some(egui::Margin::same(0.0));
                    dock_style.tab_bar.hline_color = egui::Color32::from_gray(23);
                    dock_style.separator.color_idle = egui::Color32::from_gray(23);
                    dock_style.separator.width = 1.0;
                    dock_style.separator.color_hovered = egui::Color32::from_rgb(80, 80, 90);
                    dock_style.separator.color_dragged = egui::Color32::from_rgb(100, 100, 110);
                    dock_style.separator.extra = 4.0;

                    let tree = egui_dock::DockArea::new(&mut self.dock_state).style(dock_style);

                    let mut tab_viewer = TabViewer {
                        world: &mut self.world,
                        shell: &mut self.shell,
                        selected_entity: &mut self.selected_entity,
                        rename_input: &mut self.rename_input,
                        role_input: &mut self.role_input,
                        sprite_reference_input: &mut self.sprite_reference_input,
                        is_playing: &mut self.is_playing,
                        graph: &mut self.graph,
                        render_tier: &mut self.render_tier,
                        pass_timings: &pass_timings,
                        profiler_supported,
                        prefab_editor: &mut self.prefab_editor,
                        gameplay_editor: &mut self.gameplay_editor,
                        asset_browser: &mut self.asset_browser,
                        tilemap_editor: &mut self.tilemap_editor,
                        hierarchy_panel: &mut self.hierarchy_panel,
                        camera_state: &mut self.camera,
                        current_project: &mut self.current_project,
                        project_requests: &mut self.project_requests,
                        project_blocked,
                        play: self.play.as_mut(),
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
                        asset_rename: &mut self.asset_rename,
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
                    tree.show_inside(ui, &mut tab_viewer);
                });
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

        use crate::quick_actions::QuickAction;
        let can_save = self.current_project.is_some() && self.play.is_none();
        let can_undo = self.scenes.history.can_undo() && self.play.is_none();
        let can_redo = self.scenes.history.can_redo() && self.play.is_none();
        if let Some(action) = self.shell.quick_actions.show(ctx, |action| match action {
            QuickAction::Save => can_save,
            QuickAction::Undo => can_undo,
            QuickAction::Redo => can_redo,
            _ => true,
        }) {
            menu_actions.push(match action {
                QuickAction::OpenProject => MenuAction::OpenProject,
                QuickAction::Save => MenuAction::SaveProject,
                QuickAction::Undo => MenuAction::Undo,
                QuickAction::Redo => MenuAction::Redo,
                QuickAction::Scene => MenuAction::FocusPanel(Tab::SceneView),
                QuickAction::Game => MenuAction::FocusPanel(Tab::GameView),
                QuickAction::Hierarchy => MenuAction::FocusPanel(Tab::Hierarchy),
                QuickAction::Inspector => MenuAction::FocusPanel(Tab::Inspector),
                QuickAction::Assets => MenuAction::FocusPanel(Tab::Assets),
                QuickAction::Console => MenuAction::FocusPanel(Tab::Console),
                QuickAction::Animation => MenuAction::FocusPanel(Tab::AnimationEditor),
                QuickAction::Graph => MenuAction::FocusPanel(Tab::GraphEditor),
                QuickAction::Export => MenuAction::FocusPanel(Tab::BuildPanel),
                QuickAction::Preferences => MenuAction::FocusPanel(Tab::Preferences),
                QuickAction::ResetLayout => MenuAction::ResetLayout,
            });
        }

        // Text fields retain their own editing shortcuts. Scene shortcuts apply
        // after widgets have had a chance to consume keyboard input.
        if !project_blocked
            && !self.shell.quick_actions.open
            && !ctx.wants_keyboard_input()
            && self.play.is_none()
        {
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
                            r: 0.01,
                            g: 0.01,
                            b: 0.01,
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
            let viewport =
                scene_view::camera_viewport(panel, full_output.pixels_per_point, &scene_camera);
            if !viewport.is_empty() {
                let queue = gpu.renderer.queue();
                let origin_x = viewport.x;
                let origin_y = viewport.y;

                if let Some(project) = &self.current_project {
                    let previous = gpu
                        .scene_view_sprite_renderer
                        .material_error()
                        .map(str::to_owned);
                    if let Err(error) = pollster::block_on(
                        gpu.scene_view_sprite_renderer.prepare_project_materials(
                            gpu.renderer.device(),
                            project,
                            |p| gpu.scene_textures.get(p).map(|t| t.id),
                        ),
                    ) {
                        if previous.as_ref() != Some(&error) {
                            log::warn!("{error}");
                        }
                    }
                }
                // Queue the world: background, grid, tilemaps, sprites.
                gpu.scene_view_sprite_renderer.begin_frame();
                gpu.scene_view_sprite_renderer.push(scene_view::background(
                    world.0,
                    world.1,
                    BACKGROUND_COLOR,
                ));
                if self.shell.grid && !self.is_playing {
                    for line in scene_view::grid_lines(world.0, world.1, GRID_SPACING, GRID_COLOR) {
                        gpu.scene_view_sprite_renderer.push(line);
                    }
                }
                let camera = &scene_camera;
                let view_bounds = camera.view_bounds();
                let _ = kiln_runtime::update_global_transforms(&mut self.world);
                for (texture, instance) in kiln_runtime::visual::collect_scene_draws_with_materials(
                    &self.world,
                    view_bounds,
                    |name| gpu.scene_view_sprite_renderer.material_id(name),
                ) {
                    gpu.scene_view_sprite_renderer
                        .push_textured(texture, instance);
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
                    let ui_runtime = play.slice.host_ui();
                    let ui_viewport = kiln_ui::authored::UiViewport::new(
                        [0.0, 0.0, viewport.width as f32, viewport.height as f32],
                        ui_runtime.document().reference_size,
                    );
                    for quad in ui_runtime.draw(&gpu.ui_fonts, ui_viewport) {
                        gpu.scene_view_overlay_renderer
                            .push_textured(quad.texture, quad.instance);
                    }
                    let text = kiln_ui::TextRenderer::new();
                    let mut lines = if ui_runtime.document().screens.is_empty() {
                        play.slice.hud_lines()
                    } else {
                        Vec::new()
                    };
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
                MenuAction::FocusPanel(tab) => {
                    if let Some(location) = self.dock_state.find_tab(&tab) {
                        self.dock_state.set_active_tab(location);
                    } else {
                        self.dock_state.push_to_focused_leaf(tab);
                    }
                    self.show_welcome = false;
                }
                MenuAction::ResetLayout => self.dock_state = default_dock_layout(),
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
                        log::error!("Could not start Play: {message}");
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
            project_dirty: bool,
        ) {
            egui::TopBottomPanel::bottom("status_bar")
                .exact_height(22.0)
                .frame(
                    egui::Frame::none()
                        .fill(egui::Color32::from_gray(36))
                        .inner_margin(egui::Margin::symmetric(10.0, 2.0)),
                )
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        let (status, color) = if is_playing {
                            ("Playing", egui::Color32::from_rgb(111, 204, 155))
                        } else if project_dirty {
                            ("Unsaved changes", egui::Color32::from_rgb(236, 187, 105))
                        } else {
                            ("All changes saved", egui::Color32::from_rgb(152, 166, 184))
                        };
                        ui.colored_label(color, status);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.weak(format!(
                                "{} · {:.0} FPS · {} entities · {:?} · wgpu",
                                if is_playing { "Play mode" } else { "Edit mode" },
                                fps,
                                entity_count,
                                tier
                            ));
                        });
                    });
                });
        }
    }
}
struct TabViewer<'a> {
    world: &'a mut GameWorld,
    shell: &'a mut crate::editor_shell::EditorShell,
    selected_entity: &'a mut Option<hecs::Entity>,
    rename_input: &'a mut Option<(String, String)>,
    role_input: &'a mut Option<(String, String)>,
    sprite_reference_input: &'a mut Option<(String, SpriteReferenceDraft)>,
    is_playing: &'a mut bool,
    graph: &'a mut Graph,
    render_tier: &'a mut RenderTier,
    pass_timings: &'a [PassTiming],
    profiler_supported: bool,
    prefab_editor: &'a mut PrefabEditorPanel,
    gameplay_editor: &'a mut GameplayEditorState,
    asset_browser: &'a mut AssetBrowser,
    tilemap_editor: &'a mut TilemapEditorState,
    hierarchy_panel: &'a mut HierarchyPanel,
    camera_state: &'a mut CameraState,
    current_project: &'a mut Option<Project>,
    project_requests: &'a mut Vec<ProjectRequest>,
    project_blocked: bool,
    /// The running game session, drawn over the scene view.
    play: Option<&'a mut PlaySession>,
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
    /// Pending source and destination paths in the asset move dialog.
    asset_rename: &'a mut Option<(String, String)>,
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
        if self.editing_locked() || self.project_blocked {
            return;
        }
        let ppp = ui.ctx().pixels_per_point();
        let viewport = scene_view::camera_viewport(panel, ppp, self.scene_camera);
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
        if let Some(asset) = response.dnd_release_payload::<crate::assets::SpriteAssetPayload>() {
            let point = ui.input(|input| input.pointer.interact_pos());
            let image = self
                .asset_browser
                .root()
                .ok_or("Open a project")
                .and_then(|root| {
                    std::fs::read(root.join(&asset.path)).map_err(|_| "Cannot read dropped image")
                })
                .and_then(|bytes| {
                    kiln_assets::decode_png(&bytes).map_err(|_| "Cannot decode dropped image")
                });
            match (point, self.scenes.active_id(), image) {
                (Some(point), Some(scene_id), Ok(image)) => {
                    let position = scene_view::pointer_to_world(
                        [point.x, point.y],
                        [canvas.left(), canvas.top(), canvas.right(), canvas.bottom()],
                        self.scene_camera.position,
                        units,
                    );
                    self.project_requests.push(ProjectRequest::Entity(
                        crate::entity_command::EntityCommand {
                            scene_id: scene_id.into(),
                            edit: crate::entity_command::EntityEdit::CreateSprite {
                                texture: asset.path.clone(),
                                position,
                                size: [image.width, image.height],
                            },
                        },
                    ));
                }
                (_, _, Err(error)) => self.console_log.push(format!("[ERROR] {error}")),
                _ => {}
            }
        }
        if let Some(prefab_id) = response.dnd_release_payload::<String>() {
            let pointer = ui.input(|input| input.pointer.interact_pos());
            if let (Some(pointer), Some(scene_id)) = (pointer, self.scenes.active_id()) {
                let position = scene_view::pointer_to_world(
                    [pointer.x, pointer.y],
                    [canvas.left(), canvas.top(), canvas.right(), canvas.bottom()],
                    self.scene_camera.position,
                    units,
                );
                self.project_requests.push(ProjectRequest::Entity(
                    crate::entity_command::EntityCommand {
                        scene_id: scene_id.to_owned(),
                        edit: crate::entity_command::EntityEdit::InstantiatePrefab {
                            prefab_id: prefab_id.as_str().to_owned(),
                            position,
                        },
                    },
                ));
            }
        }
        if self.gizmo.mode == GizmoMode::None || !self.shell.gizmos {
            return;
        }
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
        if let Some(project) = self.current_project.as_mut() {
            project.prefabs.insert(id.clone(), prefab.clone());
        }
        self.prefab_editor.state.add_prefab(prefab);
        self.prefab_editor.state.select_prefab(&id);
        self.log(&format!("[INFO] Created the prefab {name}"));
    }

    /// Add an instance of the selected prefab to the scene.
    fn instantiate_selected_prefab(&mut self) {
        self.finish_transform_edit();
        if self.editing_locked() || self.project_blocked {
            return;
        }
        let Some(prefab) = self.prefab_editor.state.selected_prefab().cloned() else {
            self.log("[INFO] Select a prefab to instantiate it");
            return;
        };
        let Some(scene_id) = self.scenes.active_id().map(str::to_owned) else {
            self.log("[INFO] Open a scene before instantiating a prefab");
            return;
        };
        if !self
            .current_project
            .as_ref()
            .is_some_and(|project| project.prefabs.contains_key(&prefab.id))
        {
            self.log("[ERROR] The selected prefab is not stored in the current project");
            return;
        }
        let size = self.world_size();
        self.project_requests.push(ProjectRequest::Entity(
            crate::entity_command::EntityCommand {
                scene_id,
                edit: crate::entity_command::EntityEdit::InstantiatePrefab {
                    prefab_id: prefab.id,
                    position: [size.0 / 2.0, size.1 / 2.0],
                },
            },
        ));
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
        let mut candidates: Vec<_> = project.scenes.keys().cloned().collect();
        candidates.sort();
        let already_nested: std::collections::HashSet<_> = project.scenes[&parent_id]
            .nested_scenes
            .iter()
            .map(|nested| nested.scene_id.as_str())
            .collect();
        let Some(child_id) = candidates
            .into_iter()
            .find(|id| id != &parent_id && !already_nested.contains(id.as_str()))
        else {
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
        let entries = self.asset_browser.entries().to_vec();
        let mut results = Vec::new();
        for entry in entries {
            let Some(project) = self.current_project.as_mut() else {
                return;
            };
            let asset_id = project
                .asset_ids
                .entry(entry.path.clone())
                .or_insert_with(|| kiln_core::next_id().to_string())
                .clone();
            let path = dir.join(&entry.path);
            let settings = project
                .asset_import_settings
                .get(&asset_id)
                .copied()
                .unwrap_or_default();
            let settings = kiln_assets::ImportSettings {
                filtering: if settings.nearest_filter {
                    kiln_assets::TextureFilter::Nearest
                } else {
                    kiln_assets::TextureFilter::Linear
                },
                generate_mipmaps: settings.generate_mipmaps,
                compress: !settings.is_sprite,
                max_size: 0,
                is_sprite: settings.is_sprite,
            };
            cache.set_import_settings(&path, settings);
            match cache.import(&path) {
                Ok(result) => results.push(result),
                Err(error) => {
                    self.log(&format!(
                        "[ERROR] Asset '{}' could not be imported: {error}",
                        entry.path
                    ));
                    return;
                }
            }
        }
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
            Tab::SceneView | Tab::GameView => {
                if matches!(tab, Tab::GameView) {
                    self.shell.set_preview(false);
                }
                if self.shell.preview_3d && matches!(tab, Tab::SceneView) && !*self.is_playing {
                    ui.horizontal(|ui| {
                        ui.toggle_value(&mut self.shell.gizmos, "Gizmos");
                        ui.toggle_value(&mut self.shell.grid, "Grid");
                        ui.weak("3D preview · 2D project unchanged");
                    });
                    self.shell
                        .preview
                        .viewport(ui, self.shell.grid, self.shell.gizmos);
                    return;
                }
                ui.horizontal(|ui| {
                    ui.toggle_value(&mut self.shell.gizmos, "Gizmos");
                    ui.toggle_value(&mut self.shell.grid, "Grid");
                    if ui
                        .add_enabled(!self.editing_locked(), egui::Button::new("+ Entity"))
                        .clicked()
                    {
                        self.add_entity();
                    }
                    ui.checkbox(&mut self.physics_editor.show_scene_colliders, "Colliders");
                    if ui.button("Frame (F)").clicked()
                        || (ui.input(|i| i.key_pressed(egui::Key::F))
                            && !ui.ctx().wants_keyboard_input())
                    {
                        self.frame_selection();
                    }
                    if *self.is_playing {
                        ui.menu_button("Play", |ui| {
                            if ui.button("Restart").clicked() {
                                self.project_requests.push(ProjectRequest::RestartPlay);
                                ui.close_menu();
                            }
                            if ui
                                .add_enabled(
                                    self.play.as_ref().is_some_and(|p| p.clock.paused()),
                                    egui::Button::new("Apply Runtime Transforms and Stop"),
                                )
                                .clicked()
                            {
                                self.project_requests
                                    .push(ProjectRequest::ApplyPlayTransforms);
                                ui.close_menu();
                            }
                        });
                    } else if ui.button("Play Project Start").clicked() {
                        self.project_requests.push(ProjectRequest::PlayProject);
                    }
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

                let rect = ui
                    .available_rect_before_wrap()
                    .intersect(ui.clip_rect())
                    .intersect(ui.ctx().screen_rect());

                // The engine draws the scene here, so the panel must not
                // paint over it. The rectangle is handed to the render
                // loop, which converts it to pixels and draws the scene
                // with the engine's sprite renderer.
                *self.scene_view_rect = Some(rect);
                self.translation_canvas(ui, rect);
                ui.painter().with_clip_rect(rect).text(
                    rect.right_top() + egui::vec2(-10.0, 8.0),
                    egui::Align2::RIGHT_TOP,
                    "2D · Orthographic",
                    egui::FontId::proportional(11.0),
                    egui::Color32::from_gray(154),
                );
                if self.physics_editor.show_scene_colliders {
                    let ppp = ui.ctx().pixels_per_point();
                    let viewport = scene_view::camera_viewport(rect, ppp, self.scene_camera);
                    let canvas = egui::Rect::from_min_size(
                        egui::pos2(viewport.x as f32 / ppp, viewport.y as f32 / ppp),
                        egui::vec2(viewport.width as f32 / ppp, viewport.height as f32 / ppp),
                    );
                    let painter = ui.painter().with_clip_rect(canvas);
                    for (collider, pose) in self
                        .world
                        .world_ref()
                        .query::<(
                            &kiln_runtime::world::Collider2D,
                            &kiln_runtime::GlobalTransform,
                        )>()
                        .iter()
                    {
                        let points: Vec<_> =
                            crate::physics_inspector::collider_outline(pose.0, collider.shape)
                                .into_iter()
                                .filter_map(|point| {
                                    self.scene_camera.world_to_screen(
                                        point,
                                        [
                                            canvas.left(),
                                            canvas.top(),
                                            canvas.width(),
                                            canvas.height(),
                                        ],
                                    )
                                })
                                .map(|point| egui::pos2(point[0], point[1]))
                                .collect();
                        let color = if collider.is_sensor {
                            egui::Color32::YELLOW
                        } else {
                            egui::Color32::LIGHT_GREEN
                        };
                        painter.add(egui::Shape::line(points, egui::Stroke::new(1.5_f32, color)));
                    }
                }

                // Draw gizmo mode indicator in the top-left of the scene view
                if self.shell.gizmos && self.gizmo.mode != GizmoMode::None {
                    if let Some(entity) = self.selected_entity {
                        let gizmo_color = match self.gizmo.mode {
                            GizmoMode::Translate => egui::Color32::from_rgb(255, 0, 0),
                            GizmoMode::Rotate => egui::Color32::from_rgb(0, 255, 0),
                            GizmoMode::Scale => egui::Color32::from_rgb(0, 0, 255),
                            GizmoMode::None => egui::Color32::from_rgb(128, 128, 128),
                        };
                        if let Ok(transform) = self
                            .world
                            .world_ref()
                            .get::<&kiln_runtime::GlobalTransform>(*entity)
                        {
                            let ppp = ui.ctx().pixels_per_point();
                            let viewport =
                                scene_view::camera_viewport(rect, ppp, self.scene_camera);
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
                if self.shell.preview_3d && !*self.is_playing {
                    self.shell.preview.inspector(ui);
                    return;
                }
                ui.vertical(|ui| {

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
                            ui.horizontal_wrapped(|ui| {
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
                            ui.horizontal_wrapped(|ui| {
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
                                .fill(egui::Color32::from_gray(38))
                                .rounding(4.0)
                                .inner_margin(12.0)
                                .show(ui, |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.strong("Transform");
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.menu_button("...", |ui| {
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

                                    ui.horizontal_wrapped(|ui| {
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
                                    ui.horizontal_wrapped(|ui| {
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
                                .fill(egui::Color32::from_gray(38))
                                .rounding(4.0)
                                .inner_margin(12.0)
                                .show(ui, |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.strong("Sprite");
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.menu_button("...", |ui| {
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

                                    ui.horizontal_wrapped(|ui| {
                                        ui.label("Color");
                                        let response = ui.color_edit_button_rgba_unmultiplied(&mut sprite.color);
                                        self.sprite_visual_response(entity, &sprite, &response);
                                    });

                                    ui.horizontal_wrapped(|ui| {
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

                                    ui.horizontal_wrapped(|ui| {
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
                                        ui.horizontal_wrapped(|ui| {
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
                                    ui.horizontal_wrapped(|ui| {
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
                                    ui.horizontal_wrapped(|ui| {
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
                        if has_sprite {
                            let mut settings=authored.components.iter().find_map(|component|match component{kiln_core::Component::SpriteLayout {settings}=>Some(*settings),_=>None}).unwrap_or_default();
                            ui.collapsing("Sprite Pivot / Pixel Scale",|ui|{
                                if crate::sprite_editor::layout_ui(ui,&mut settings) {self.queue_component(entity,ComponentEdit::SetVisual(kiln_core::Component::SpriteLayout {settings}));}
                            });
                        }
                        if authored.components.iter().any(|component|matches!(component,kiln_core::Component::Particles {..})) {
                            ui.label("Saved particle effect: edit in VFX Editor.");
                            if ui.button("Remove Particles").clicked() {self.remove_component(entity,ComponentKind::Particles);}
                        } else if ui.button("+ Particles").clicked() {self.add_component(entity,ComponentKind::Particles);}
                        if let Some((mut body, mut collider)) = authored.components.iter().find_map(|component| match component {
                            kiln_core::Component::Physics { body, collider } => Some((*body,*collider)), _=>None,
                        }) {
                            ui.group(|ui| {
                                ui.strong("Physics Body / Collider");
                                if crate::physics_inspector::body_ui(ui,&mut body,&mut collider) { self.queue_component(entity,ComponentEdit::SetPhysics {body,collider}); }
                                if ui.button("Remove Physics").clicked() { self.remove_component(entity,ComponentKind::Physics); }
                            });
                        } else if ui.button("+ Physics Body / Collider").clicked() { self.add_component(entity,ComponentKind::Physics); }
                        if let Some(mut joint)=authored.components.iter().find_map(|component| match component {kiln_core::Component::Joint {joint}=>Some(joint.clone()),_=>None}) {
                            ui.group(|ui| {
                                ui.strong("Physics Joint");
                                if crate::physics_inspector::joint_ui(ui,&mut joint) {self.queue_component(entity,ComponentEdit::SetJoint {joint:joint.clone()});}
                                if ui.button("Remove Joint").clicked() {self.remove_component(entity,ComponentKind::Joint);}
                            });
                        } else {
                            let targets: Vec<_>=self.current_project.as_ref().and_then(|project| self.scenes.active_id().and_then(|id| project.scenes.get(id))).map(|scene| scene.entities.iter().filter(|candidate| candidate.id!=authored.id).map(|candidate| candidate.id.clone()).collect()).unwrap_or_default();
                            ui.menu_button("+ Joint to Entity",|ui| {
                                for target in targets {if ui.button(&target).clicked() {
                                    self.queue_component(entity,ComponentEdit::Add(kiln_core::Component::Joint {joint:kiln_core::physics_data::Joint2D {
                                        entity_a:authored.id.clone(),entity_b:target,joint_type:kiln_core::physics_data::JointType::Distance,anchor_a:[0.0;2],anchor_b:[0.0;2],reference_angle:0.0,length:32.0,lower_translation:-32.0,upper_translation:32.0,enable_limit:false,motor_speed:0.0,max_motor_torque:0.0,enable_motor:false,
                                    }}));ui.close_menu();
                                }}
                            });
                        }
                        if let Some((path, properties)) = authored.components.iter().find_map(|component| match component {
                            kiln_core::Component::Script { path, properties } => Some((path.clone(), properties.clone())), _ => None,
                        }) {
                            ui.group(|ui| {
                                ui.strong(format!("Script: {path}"));
                                if !self.asset_browser.entries().iter().any(|entry| entry.path == path) {
                                    ui.colored_label(egui::Color32::YELLOW, "Script resource is missing");
                                }
                                let mut edited = properties.clone();
                                let mut names: Vec<_> = properties.keys().cloned().collect(); names.sort();
                                let mut changed = false;
                                for name in names {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(&name);
                                        if let Some(value) = edited.get_mut(&name) { changed |= ui.add(egui::DragValue::new(value)).changed(); }
                                    });
                                }
                                let widget_id = ui.id().with(("script-property-name", &authored.id));
                                let mut name = ui.data_mut(|data| data.get_temp::<String>(widget_id).unwrap_or_default());
                                ui.horizontal_wrapped(|ui| {
                                    ui.text_edit_singleline(&mut name);
                                    if ui.button("Add Property").clicked() && !name.trim().is_empty() {
                                        edited.entry(name.trim().into()).or_insert(0.0); changed = true;
                                    }
                                });
                                ui.data_mut(|data| data.insert_temp(widget_id, name));
                                if changed { self.queue_component(entity, ComponentEdit::SetScript { path: path.clone(), properties: edited }); }
                                if ui.button("Remove Script").clicked() { self.remove_component(entity, ComponentKind::Script); }
                            });
                        } else {
                            let scripts: Vec<_> = self.asset_browser.entries().iter().filter(|entry| entry.path.ends_with(".ks")).map(|entry| entry.path.clone()).collect();
                            ui.menu_button("Attach Script", |ui| {
                                if scripts.is_empty() { ui.label("Add a .ks file to the project and refresh Assets."); }
                                for path in scripts {
                                    if ui.button(&path).clicked() {
                                        self.queue_component(entity, ComponentEdit::Add(kiln_core::Component::Script { path, properties: HashMap::new() }));
                                        ui.close_menu();
                                    }
                                }
                            });
                        }
                        if authored.components.iter().any(|component| matches!(component, kiln_core::Component::PrefabInstance { .. })) {
                            ui.horizontal_wrapped(|ui| {
                                ui.label("Prefab instance");
                                let can_revert = !self.editing_locked() && !self.project_blocked;
                                if ui.add_enabled(can_revert, egui::Button::new("Apply Overrides to Prefab")).clicked() {
                                    self.project_requests.push(ProjectRequest::ApplyPrefabInstance(authored.id.clone()));
                                }
                                if ui.add_enabled(can_revert, egui::Button::new("Revert Overrides")).clicked() {
                                    if let Some(scene_id) = self.scenes.active_id().map(str::to_owned) {
                                        self.project_requests.push(ProjectRequest::Entity(
                                            crate::entity_command::EntityCommand {
                                                scene_id,
                                                edit: crate::entity_command::EntityEdit::RevertPrefabOverrides {
                                                    entity_id: authored.id.clone(),
                                                },
                                            },
                                        ));
                                    }
                                }
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
                        ui.vertical(|ui| {
                            ui.label("No entity selected");
                            ui.add_space(8.0);
                            ui.label(
                                "Select an entity in the hierarchy or scene to inspect it",
                            );
                        });
                    }
                });
                crate::editor_shell::EditorShell::future_components(ui);
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
                    let rename_source = self.asset_browser.preview_source().map(str::to_owned);
                    ui.horizontal(|ui| {
                        ui.weak("Assets");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Import").clicked() {
                                self.import_assets();
                            }
                            if ui
                                .add_enabled(
                                    rename_source.is_some(),
                                    egui::Button::new("Move / Rename"),
                                )
                                .clicked()
                            {
                                if let Some(source) = rename_source.as_ref() {
                                    *self.asset_rename = Some((source.clone(), source.clone()));
                                }
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
                        ui.add(egui::TextEdit::singleline(self.asset_filter).hint_text("Search assets").desired_width(180.0));
                        if self.asset_browser.filter() != self.asset_filter {
                            self.asset_browser.set_filter(self.asset_filter.clone());
                        }
                    });

                    ui.add_space(2.0);

                    let mut scene_ids: Vec<_> = self.current_project.as_ref().map(|p| p.scenes.keys().cloned().collect()).unwrap_or_default();
                    scene_ids.sort();
                    match crate::asset_shelf::show(ui, self.asset_browser, &mut self.shell.asset_category, &mut self.shell.thumbnails, &scene_ids) {
                        Some(crate::asset_shelf::ShelfAction::SelectImage(path)) => {
                            if let Err(error) = self.asset_browser.select(&path) { self.console_log.push(format!("[ERROR] {error}")); }
                        }
                        Some(crate::asset_shelf::ShelfAction::OpenScene(id)) => {
                            self.project_requests.push(ProjectRequest::Scene(SceneAction::Open(id)));
                        }
                        None => {}
                    }

                    if let Some((old_path, new_path)) = self.asset_rename.as_mut() {
                        let mut commit = false;
                        let mut cancel = false;
                        egui::Window::new("Move or Rename Asset")
                            .collapsible(false)
                            .resizable(false)
                            .show(ui.ctx(), |ui| {
                                ui.label(format!("Current path: {old_path}"));
                                ui.label("New project-relative path:");
                                ui.text_edit_singleline(new_path);
                                ui.horizontal(|ui| {
                                    commit = ui.button("Move").clicked();
                                    cancel = ui.button("Cancel").clicked();
                                });
                            });
                        if commit {
                            let (old_path, new_path) = self.asset_rename.take().unwrap();
                            self.project_requests
                                .push(ProjectRequest::RenameAsset { old_path, new_path });
                        } else if cancel {
                            *self.asset_rename = None;
                        }
                    }

                    // PNG preview of the selected asset.
                    if let Some(source) = self.asset_browser.preview_source() {
                        ui.collapsing("Selected texture · import settings and preview", |ui| {
                        ui.add_space(8.0);
                        ui.label(format!("Preview: {source}"));
                        if let Some(project) = self.current_project.as_mut() {
                            if let Some(asset_id) = project.asset_ids.get(source).cloned() {
                                ui.small(format!("Asset ID: {asset_id}"));
                                let mut settings = project.asset_import_settings
                                    .get(&asset_id).copied().unwrap_or_default();
                                let mut changed = false;
                                ui.horizontal(|ui| {
                                    changed |= ui.checkbox(&mut settings.is_sprite, "Sprite").changed();
                                    changed |= ui.checkbox(&mut settings.nearest_filter, "Nearest filtering").changed();
                                    changed |= ui.checkbox(&mut settings.generate_mipmaps, "Mipmaps").changed();
                                });
                                if changed {
                                    project.asset_import_settings.insert(asset_id, settings);
                                    self.project_requests.push(ProjectRequest::RefreshVisuals);
                                }
                            } else {
                                ui.label("Import assets to assign a stable ID and configure import settings.");
                            }
                        }
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
                        });
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
                        .fill(egui::Color32::from_gray(38))
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
                        .fill(egui::Color32::from_gray(38))
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
                        .fill(egui::Color32::from_gray(38))
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
                        .fill(egui::Color32::from_gray(38))
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
            Tab::UIDesigner | Tab::UiLayoutEditor => {
                if self.editing_locked() {
                    ui.disable();
                }
                if let Some(project) = self.current_project.as_mut() {
                    let fonts = project.ui.fonts.len();
                    self.ui_layout_editor
                        .render(ui, project, self.asset_browser.root());
                    if fonts != project.ui.fonts.len() {
                        if let Some(root) = self.asset_browser.root().map(Path::to_path_buf) {
                            self.asset_browser.scan(&root);
                        }
                    }
                } else {
                    ui.label("Open a project to author saved UI screens.");
                }
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

                        if ui.button("Undo Prefab Apply").clicked() {
                            self.project_requests.push(ProjectRequest::UndoPrefabApply);
                        }
                        if ui.button("Redo Prefab Apply").clicked() {
                            self.project_requests.push(ProjectRequest::RedoPrefabApply);
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
                    let applied = self.prefab_editor.render(ui);
                    if let Some(project) = self.current_project.as_mut() {
                        project.prefabs = self.prefab_editor.state.prefabs().clone();
                    }
                    if applied {
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
                            let saved = self.current_project.as_mut().map_or(0, |project| {
                                crate::editing::propagate_prefab_in_scenes(project, &prefab)
                            });
                            if updated > 0 || saved > 0 {
                                self.log(&format!(
                                    "[INFO] Updated {} live and {} saved instances of prefab '{}'",
                                    updated, saved, prefab.name
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

                    let locked = self.editing_locked();
                    let scene_id = self.scenes.active_id().map(str::to_owned);
                    if let (Some(project), Some(id)) = (self.current_project.as_mut(), scene_id) {
                        let mut settings = project.cameras.get(&id).cloned().unwrap_or_default();
                        let mut ids: Vec<_> = project
                            .scenes
                            .get(&id)
                            .map(|scene| {
                                scene
                                    .entities
                                    .iter()
                                    .map(|entity| entity.id.clone())
                                    .collect()
                            })
                            .unwrap_or_default();
                        ids.sort();
                        ui.collapsing("Gameplay Camera", |ui| {
                            ui.add_enabled_ui(!locked, |ui| {
                                if crate::physics_inspector::camera_ui(
                                    ui,
                                    &mut settings,
                                    &ids,
                                    [project.window.width as f32, project.window.height as f32],
                                ) && settings.validate().is_ok()
                                {
                                    project.cameras.insert(id, settings);
                                }
                            });
                        });
                    }

                    ui.add_space(8.0);

                    // Scene composition info
                    egui::Frame::none()
                        .fill(egui::Color32::from_gray(38))
                        .rounding(4.0)
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.strong("Nested Scenes");
                            ui.add_space(8.0);

                            let active_id = self.scenes.active_id().map(str::to_owned);
                            if let (Some(project), Some(parent_id)) =
                                (self.current_project.as_mut(), active_id)
                            {
                                let nested = project
                                    .scenes
                                    .get(&parent_id)
                                    .map(|scene| scene.nested_scenes.clone())
                                    .unwrap_or_default();
                                if nested.is_empty() {
                                    ui.label(
                                        "No nested scenes. Use Add Nested Scene to compose scenes.",
                                    );
                                }
                                for link in nested {
                                    let name = project
                                        .scenes
                                        .get(&link.scene_id)
                                        .map(|scene| scene.name.as_str())
                                        .unwrap_or("Missing scene");
                                    let mut offset = [link.offset.x, link.offset.y];
                                    let mut additive = link.additive;
                                    let mut remove = false;
                                    let mut changed = false;
                                    ui.horizontal(|ui| {
                                        ui.label(format!("{} ({})", name, link.scene_id));
                                        changed |= ui
                                            .add(egui::DragValue::new(&mut offset[0]).prefix("X "))
                                            .changed();
                                        changed |= ui
                                            .add(egui::DragValue::new(&mut offset[1]).prefix("Y "))
                                            .changed();
                                        changed |= ui.checkbox(&mut additive, "Additive").changed();
                                        remove = ui.small_button("Remove").clicked();
                                    });
                                    if changed {
                                        let _ = crate::editing::configure_nested_scene(
                                            project,
                                            &parent_id,
                                            &link.scene_id,
                                            offset,
                                            additive,
                                        );
                                    }
                                    if remove {
                                        let _ = crate::editing::unnest_scene(
                                            project,
                                            &parent_id,
                                            &link.scene_id,
                                        );
                                    }
                                }
                            } else {
                                ui.label("Open a project scene to edit its composition.");
                            }
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
                    if self.editing_locked() {
                        ui.disable();
                    }
                    if let Some(project) = self.current_project.as_mut() {
                        let before = serde_json::to_value((
                            &project.tilemaps,
                            &project.tilesets,
                            &project.terrains,
                        ))
                        .ok();
                        crate::tilemap_editor::show_with_root(
                            ui,
                            self.tilemap_editor,
                            project,
                            self.asset_browser.root(),
                        );
                        if before
                            != serde_json::to_value((
                                &project.tilemaps,
                                &project.tilesets,
                                &project.terrains,
                            ))
                            .ok()
                        {
                            self.project_requests.push(ProjectRequest::RefreshVisuals);
                        }
                    } else {
                        ui.heading("Tilemap Editor");
                        ui.add_space(8.0);
                        ui.label("Open a project to author tilemaps.");
                    }
                });
            }

            Tab::AnimationEditor => {
                ui.vertical(|ui| {
                    if self.editing_locked() {
                        ui.disable();
                    }
                    if let Some(project) = self.current_project.as_mut() {
                        let before = serde_json::to_value(&project.animation_clips).ok();
                        AnimationEditorState::render_with_root(
                            ui,
                            self.anim_editor,
                            &mut project.animation_clips,
                            ui.input(|input| input.stable_dt),
                            self.asset_browser.root(),
                        );
                        if before != serde_json::to_value(&project.animation_clips).ok() {
                            self.project_requests.push(ProjectRequest::RefreshVisuals);
                        }
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
                        if let Some(config) = self.build_panel.render(ui, project) {
                            self.project_requests.push(ProjectRequest::Export(config));
                        }
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
                        ui.heading("Live Gameplay Debugger");
                        ui.label(format!("Scene: {}", play.scene_id));
                        ui.horizontal(|ui| {
                            if ui
                                .button(if play.clock.paused() {
                                    "Resume"
                                } else {
                                    "Pause"
                                })
                                .clicked()
                            {
                                self.project_requests.push(ProjectRequest::PausePlay);
                            }
                            if ui.button("Step Frame").clicked() {
                                self.project_requests.push(ProjectRequest::StepPlay);
                            }
                            if ui.button("Restart").clicked() {
                                self.project_requests.push(ProjectRequest::RestartPlay);
                            }
                        });
                        if let Some(error) = &play.error {
                            ui.colored_label(egui::Color32::LIGHT_RED, error);
                        }
                        ui.checkbox(&mut self.debugger_panel.show_locals, "Inspect entity state");
                        if self.debugger_panel.show_locals {
                            for entity in play.slice.entity_script_debug() {
                                ui.collapsing(
                                    format!(
                                        "{} — {} ({})",
                                        entity.entity_id, entity.path, entity.phase
                                    ),
                                    |ui| {
                                        for (name, value) in entity.values {
                                            ui.monospace(format!("{name} = {value:?}"));
                                        }
                                    },
                                );
                            }
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
                    if self.editing_locked() {
                        ui.disable();
                    }
                    if let Some(entity) = *self.selected_entity {
                        if let Some(authored) = self.authored_entity(entity) {
                            let mut effect = authored
                                .components
                                .iter()
                                .find_map(|component| match component {
                                    kiln_core::Component::Particles { effect } => {
                                        Some(effect.clone())
                                    }
                                    _ => None,
                                })
                                .unwrap_or_default();
                            let attached = authored.components.iter().any(|component| {
                                matches!(component, kiln_core::Component::Particles { .. })
                            });
                            let changed =
                                self.vfx_editor.render_effect(ui, &authored.id, &mut effect);
                            if changed
                                || (!attached && ui.button("Attach Effect to Entity").clicked())
                            {
                                self.queue_component(
                                    entity,
                                    ComponentEdit::SetVisual(kiln_core::Component::Particles {
                                        effect,
                                    }),
                                );
                            }
                        }
                    } else {
                        ui.label("Select a scene entity to author a saved particle effect.");
                    }
                });
            }
            Tab::AudioEditor => {
                if self.editing_locked() {
                    ui.disable();
                }
                if let Some(project) = self.current_project.as_mut() {
                    let before = project.audio_assets.len();
                    self.audio_editor
                        .render(ui, project, self.asset_browser.root());
                    if before != project.audio_assets.len() {
                        if let Some(root) = self.asset_browser.root().map(Path::to_path_buf) {
                            self.asset_browser.scan(&root);
                        }
                    }
                } else {
                    ui.label("Open a project to author audio.");
                }
            }
            Tab::PhysicsEditor => {
                ui.vertical(|ui| {
                    self.physics_editor.render(ui);
                });
            }
            Tab::ShaderEditor => {
                if self.editing_locked() {
                    ui.disable();
                }
                ui.vertical(|ui| {
                    if let Some(project) = self.current_project.as_mut() {
                        let before =
                            serde_json::to_value((&project.shaders, &project.materials)).ok();
                        crate::shader_editor::render_shader_editor(ui, self.shader_editor, project);
                        if before
                            != serde_json::to_value((&project.shaders, &project.materials)).ok()
                        {
                            self.project_requests.push(ProjectRequest::RefreshVisuals);
                        }
                        if let Some(error) = self.gpu.scene_view_sprite_renderer.material_error() {
                            ui.colored_label(egui::Color32::RED, error);
                        }
                    } else {
                        ui.heading("Shader Editor");
                        ui.add_space(8.0);
                        ui.label("Open a project to edit shaders.");
                    }
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
                    if self.editing_locked() {
                        ui.disable();
                    }
                    if let Some(project) = self.current_project.as_mut() {
                        let before = project.scenes.clone();
                        let resources = serde_json::to_value(&project.animation_clips).ok();
                        self.sprite_editor
                            .render_with_root(ui, project, self.asset_browser.root());
                        let changed = serde_json::to_value(&before).ok()
                            != serde_json::to_value(&project.scenes).ok();
                        if changed {
                            for (id, scene) in before {
                                if serde_json::to_value(&scene).ok()
                                    != project
                                        .scenes
                                        .get(&id)
                                        .and_then(|scene| serde_json::to_value(scene).ok())
                                {
                                    self.scenes.record_external_edit(scene);
                                }
                            }
                        }
                        if changed
                            || resources != serde_json::to_value(&project.animation_clips).ok()
                        {
                            self.project_requests.push(ProjectRequest::RefreshVisuals);
                        }
                    } else {
                        ui.heading("Sprite Editor");
                        ui.add_space(8.0);
                        ui.label("Open a project to edit sprites.");
                    }
                });
            }
            Tab::Hierarchy => {
                if self.shell.preview_3d && !*self.is_playing {
                    self.shell.preview.hierarchy(ui);
                    return;
                }
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
        !matches!(tab, Tab::SceneView | Tab::GameView)
    }

    fn scroll_bars(&self, tab: &Self::Tab) -> [bool; 2] {
        [!matches!(tab, Tab::SceneView | Tab::GameView); 2]
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            Tab::SceneView => "Scene".into(),
            Tab::GameView => "Game".into(),
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
                sprite.world_bounds(transform.0)
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
}

#[cfg(test)]
mod play_tests {
    use super::*;

    #[test]
    fn play_input_reads_quick_edges_with_precomputed_display_scale() {
        let ctx = egui::Context::default();
        let pointer = |pressed| egui::Event::PointerButton {
            pos: egui::pos2(10.0, 20.0),
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        ctx.begin_pass(egui::RawInput {
            events: vec![
                pointer(true),
                pointer(false),
                egui::Event::Key {
                    key: egui::Key::ArrowRight,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::Key {
                    key: egui::Key::Tab,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::SHIFT,
                },
            ],
            ..Default::default()
        });
        // The extractor runs inside the real context lock. It must use the
        // supplied scale rather than re-entering Context::pixels_per_point.
        let (keys, points) = ctx.input(|input| play_input_edges(input, 2.0, false));
        assert_eq!(keys, ["ArrowRight", "BackTab"]);
        assert_eq!(points, [([20.0, 40.0], true), ([20.0, 40.0], false)]);
        let (keys, points) = ctx.input(|input| play_input_edges(input, 2.0, true));
        assert!(keys.is_empty());
        assert_eq!(points.len(), 2);
        let _ = ctx.end_pass();
    }

    #[test]
    fn default_layout_keeps_scene_between_sidebars_and_above_full_width_shelf() {
        struct Probe(Vec<(Tab, egui::Rect)>);
        impl egui_dock::TabViewer for Probe {
            type Tab = Tab;
            fn title(&mut self, tab: &mut Tab) -> egui::WidgetText {
                format!("{tab:?}").into()
            }
            fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Tab) {
                self.0.push((*tab, ui.available_rect_before_wrap()));
            }
        }
        for width in [1000.0, 1440.0, 1920.0] {
            let ctx = egui::Context::default();
            crate::editor_shell::apply_theme(&ctx);
            let mut dock = default_dock_layout();
            let mut probe = Probe(Vec::new());
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 900.0));
            for _ in 0..3 {
                probe.0.clear();
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(screen),
                        ..Default::default()
                    },
                    |ctx| {
                        egui::TopBottomPanel::top("toolbar")
                            .exact_height(62.0)
                            .show(ctx, |_| {});
                        egui::TopBottomPanel::bottom("status")
                            .exact_height(22.0)
                            .show(ctx, |_| {});
                        egui::CentralPanel::default()
                            .frame(egui::Frame::none())
                            .show(ctx, |ui| {
                                egui_dock::DockArea::new(&mut dock).show_inside(ui, &mut probe);
                            });
                    },
                );
            }
            let rect = |tab| probe.0.iter().find(|(t, _)| *t == tab).unwrap().1;
            let scene = rect(Tab::SceneView);
            let hierarchy = rect(Tab::Hierarchy);
            let inspector = rect(Tab::Inspector);
            let shelf = rect(Tab::Assets);
            assert!(hierarchy.right() < scene.left());
            assert!(scene.right() < inspector.left());
            assert!(scene.width() > width * 0.55);
            assert!(shelf.width() > width * 0.98);
            assert!(shelf.top() > scene.bottom());
            for (_, panel) in &probe.0 {
                assert!(screen.contains_rect(*panel), "{panel:?} outside {screen:?}");
                assert!(panel.top() >= 62.0 && panel.bottom() <= 878.0);
            }
        }
    }

    #[test]
    fn play_launch_uses_requested_scene_and_reports_script_location() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("first.ks"), "add_score(1);").unwrap();
        std::fs::write(root.path().join("second.ks"), "add_score(7);").unwrap();
        let mut project = Project::default();
        for (id, script) in [("a", "first.ks"), ("b", "second.ks")] {
            project.scenes.insert(
                id.into(),
                Scene {
                    id: id.into(),
                    name: id.into(),
                    script: Some(script.into()),
                    entities: vec![],
                    nested_scenes: vec![],
                },
            );
        }
        let (mut session, id, mut world) =
            start_play_session(&project, "b", root.path(), &SceneTextures::default()).unwrap();
        assert_eq!(id, "b");
        session
            .step(&mut world, (0.0, 0.0), kiln_runtime::play_clock::PLAY_STEP)
            .unwrap();
        assert_eq!(session.state().score, 7);
        std::fs::write(root.path().join("second.ks"), "\nunknown_call();").unwrap();
        let error = match start_play_session(&project, "b", root.path(), &SceneTextures::default())
        {
            Ok(_) => panic!("Expected a compiler error"),
            Err(error) => error,
        };
        assert!(error.contains("second.ks:2:1:"), "{error}");
    }
}
