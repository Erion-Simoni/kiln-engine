//! Main editor application.

use kiln_core::{project::Project, Transform};
use kiln_gameplay::{DataTable, Dialogue, Quest};
use kiln_graph::{ArithmeticOp, ConstantValue, Graph, Node, NodeType};
use kiln_render::{
    camera::Camera, clear_view, execute_passes, Frame2dGraph, PassTiming, RenderTier,
    SpriteInstance, SpriteRenderer, TextureId,
};
use kiln_runtime::{GameWorld, SourceId, Sprite};
use kiln_ui::{Button, Label, List, Panel, Widget, WidgetId};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use winit::{
    event_loop::{ActiveEventLoop, EventLoop},
    window::Window,
};

use crate::assets::{AssetBrowser, AssetKind};
use crate::history::History;
use crate::prefab_editor::PrefabEditorState;
use crate::scene_ops::{self, ComponentKind, Scene, SceneEntity};
use crate::tilemap_editor::TilemapEditorState;

/// The platforms the editor can export a project for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportPlatform {
    /// A WebAssembly build that runs in the browser.
    Web,
    /// A native build for the current desktop.
    Native,
    /// An Android package.
    Android,
}

impl ExportPlatform {
    /// The name shown in the export dialog.
    pub fn label(self) -> &'static str {
        match self {
            ExportPlatform::Web => "Web (WASM)",
            ExportPlatform::Native => "Native (desktop)",
            ExportPlatform::Android => "Android",
        }
    }

    /// The `--platform` argument the command line tool expects.
    pub fn cli_name(self) -> &'static str {
        match self {
            ExportPlatform::Web => "web",
            ExportPlatform::Native => "native",
            ExportPlatform::Android => "android",
        }
    }

    /// All platforms, in the order the dialog lists them.
    pub fn all() -> [ExportPlatform; 3] {
        [
            ExportPlatform::Web,
            ExportPlatform::Native,
            ExportPlatform::Android,
        ]
    }
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
    About,
    Export,
}

pub struct EditorApp {
    gpu: Option<GpuState>,
    egui_winit: egui_winit::State,
    dock_state: egui_dock::DockState<Tab>,
    world: GameWorld,
    /// The id of the selected scene entity.
    ///
    /// The selection is an id rather than a runtime entity, because the world
    /// is rebuilt after every edit.
    selected_id: Option<String>,
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
    /// Pixel size of every scene texture, keyed by asset path.
    texture_sizes: HashMap<String, (u32, u32)>,
    /// Texture IDs in the scene view SpriteRenderer, keyed by asset path.
    scene_view_texture_ids: HashMap<String, TextureId>,
    /// Screen-space rectangle of the scene view panel from the last frame.
    scene_view_rect: Option<egui::Rect>,
    graph: Graph,
    render_tier: RenderTier,
    show_welcome: bool,
    frame_count: u32,
    ui_designer: UIDesignerState,
    prefab_editor: PrefabEditorState,
    gameplay_editor: GameplayEditorState,
    tilemap_editor: TilemapEditorState,
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
    /// The undo and redo stacks of the editor.
    history: History,
    /// The scene being edited, as data the user can change.
    ///
    /// The runtime world is rebuilt from this after every edit, so the
    /// inspector, the undo stack and the saved file all agree.
    scene: Scene,
    /// The name of the next entity added to the scene.
    next_entity_name: u32,
    /// Set when the scene was changed by a panel, so the world is rebuilt.
    scene_dirty: bool,
    /// The scene as it was before the edit in progress.
    ///
    /// The inspector clones the scene before it changes a value, so the
    /// change can be undone. The clone is only made when something changes.
    scene_before_edit: Option<Scene>,
    /// Set by the scene view, handled after the UI pass.
    add_entity_requested: bool,
    /// Set by the inspector, handled after the UI pass.
    remove_entity_requested: bool,
    /// Set by the inspector, handled after the UI pass.
    remove_component_requested: Option<ComponentKind>,
    /// Set by the inspector, handled after the UI pass.
    add_component_requested: bool,
    /// The path typed into the asset import field.
    import_path: String,
    /// Whether the "About Kiln" window is open.
    show_about: bool,
    /// Whether the export dialog is open.
    show_export: bool,
    /// The platform the export dialog exports for.
    export_platform: ExportPlatform,
    /// The result channel of a running export.
    export_result: Option<std::sync::mpsc::Receiver<String>>,
    /// Set by the export dialog, handled after the UI pass.
    export_requested: bool,
    /// The last frame time of the editor, in seconds.
    frame_time: f32,
    /// The time of the previous frame, used to measure the frame time.
    last_frame: Option<std::time::Instant>,
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
    TilemapEditor,
}

struct GpuState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    renderer: kiln_render::Renderer,
    egui_renderer: egui_wgpu::Renderer,
    /// Sprite batch renderer for the scene view. Draws scene entities using the
    /// same GPU pipeline as the standalone player.
    scene_view_sprite_renderer: SpriteRenderer,
    /// Sprite batch renderer for the scene view's HUD.
    ///
    /// The HUD uses its own renderer because it has its own camera: the scene
    /// is drawn in world space, the HUD in screen space.
    scene_view_hud_renderer: SpriteRenderer,
    /// Camera for the scene view (world space).
    scene_view_camera: Camera,
    /// Camera for the scene view's HUD (screen space, top-left origin).
    scene_view_hud_camera: Camera,
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
    sizes: &HashMap<String, (u32, u32)>,
    entities: &[kiln_core::project::Entity],
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

    // The game starts from the edited scene, so the editor shows what the
    // player will see, even with unsaved changes.
    let mut world = GameWorld::new();
    let mut make_sprite =
        |_entity: &kiln_core::project::Entity, color: [f32; 4], texture: Option<&str>| {
            let size = texture
                .and_then(|path| sizes.get(path))
                .copied()
                .unwrap_or((64, 64));
            kiln_runtime::Sprite::new(color[0], color[1], color[2], color[3])
                .with_size(size.0 as f32, size.1 as f32)
        };
    kiln_runtime::spawn_entities(
        &mut world,
        entities,
        kiln_core::project::SceneOffset::default(),
        &mut make_sprite,
    );

    Ok((slice, scene_id, world))
}

/// Read the pixel size of every texture a project uses.
///
/// Sprites are as big as their texture, so the sizes are needed before the
/// scene is spawned.
fn collect_texture_sizes(dir: &Path, project: &Project) -> HashMap<String, (u32, u32)> {
    let mut sizes = HashMap::new();
    let mut paths: Vec<String> = Vec::new();
    for scene in project.scenes.values() {
        for entity in &scene.entities {
            for component in &entity.components {
                if let kiln_core::project::Component::Sprite {
                    texture: Some(path),
                    ..
                } = component
                {
                    if !paths.contains(path) {
                        paths.push(path.clone());
                    }
                }
            }
        }
    }
    for path in paths {
        let Ok(bytes) = std::fs::read(dir.join(&path)) else {
            continue;
        };
        if let Ok(image) = kiln_assets::decode_png(&bytes) {
            sizes.insert(path, (image.width, image.height));
        }
    }
    sizes
}

/// Record the scene before an inspector change, so the change can be undone.
///
/// The key names the edited value, so a slider drag is one undo step. The
/// history and the snapshot are passed separately, because the inspector
/// holds a borrow of the scene at the same time.
fn record_edit(history: &mut History, snapshot: &mut Option<Scene>, key: &str) {
    let Some(before) = snapshot.take() else {
        return;
    };
    history.record_with_key(Some(key), &before);
}

/// Open a URL in the user's browser.
///
/// Uses the platform's opener, so no extra dependency is needed. Fails when
/// no opener is available, which is fine: the caller shows the URL instead.
fn open_in_browser(url: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    let opener = "xdg-open";
    #[cfg(target_os = "macos")]
    let opener = "open";
    #[cfg(target_os = "windows")]
    let opener = "explorer";

    std::process::Command::new(opener)
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("could not run {opener}: {e}"))
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
            selected_id: None,
            is_playing: false,
            play: None,
            console_log: Vec::new(),
            project_root: None,
            project_path_input: "kiln.toml".to_string(),
            entity_textures: HashMap::new(),
            entity_roles: HashMap::new(),
            scene_view_texture_ids: HashMap::new(),
            scene_view_rect: None,
            texture_sizes: HashMap::new(),
            graph: Graph::new(),
            render_tier: RenderTier::default(),
            show_welcome: true,
            frame_count: 0,
            ui_designer: UIDesignerState::new(),
            prefab_editor: PrefabEditorState::new(),
            gameplay_editor: GameplayEditorState::new(),
            tilemap_editor: TilemapEditorState::new(),
            project_path: None,
            current_project: None,
            asset_browser: AssetBrowser::new(),
            open_project_requested: false,
            play_toggle_requested: false,
            scene_save_requested: false,
            scene_load_requested: false,
            history: History::new(),
            scene: Scene::new(),
            next_entity_name: 1,
            scene_dirty: false,
            scene_before_edit: None,
            add_entity_requested: false,
            remove_entity_requested: false,
            remove_component_requested: None,
            add_component_requested: false,
            import_path: String::new(),
            show_about: false,
            show_export: false,
            export_platform: ExportPlatform::Web,
            export_result: None,
            export_requested: false,
            frame_time: 1.0 / 60.0,
            last_frame: None,
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
        let (scene_view_sprite_renderer, scene_view_hud_renderer, egui_renderer) = {
            let device = renderer.device();
            let queue = renderer.queue();
            let format = renderer.format();
            let sr = SpriteRenderer::new(device, queue, format);
            let hud = SpriteRenderer::new(device, queue, format);
            hud.upload_quad(queue);
            let er = egui_wgpu::Renderer::new(device, format, None, 1, false);
            (sr, hud, er)
        };

        self.gpu = Some(GpuState {
            window,
            surface,
            renderer,
            egui_renderer,
            scene_view_sprite_renderer,
            scene_view_hud_renderer,
            scene_view_camera: Camera::new(size.width as f32, size.height as f32),
            scene_view_hud_camera: Camera::new_top_left(size.width as f32, size.height as f32),
        });
    }

    /// Save the current project to its file path.
    pub fn save_project(&mut self) -> Result<(), String> {
        // The edited scene is written into the project first, so the file
        // holds what the user sees in the editor.
        self.sync_scene_into_project()?;
        self.write_project_file()
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
            if let Some(project) = &self.current_project {
                self.texture_sizes = collect_texture_sizes(dir, project);
            }
        }

        // Clear current world and load scene entities
        self.load_project_scene();

        Ok(())
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

        let entities = self.scene.to_project_entities();
        let (slice, scene_id, world) =
            start_play_session(&project, &root, &self.texture_sizes, &entities)?;

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
        // The edited scene is kept, so unsaved edits are not lost. The world
        // is rebuilt from it, because the game changed it.
        self.rebuild_world();
    }

    /// Reload the edited scene into the editor world.
    fn load_project_scene(&mut self) {
        self.scene = Scene::new();
        self.history.clear();
        self.selected_id = None;

        let Some(project) = self.current_project.clone() else {
            self.world = GameWorld::new();
            self.entity_textures.clear();
            self.entity_roles.clear();
            return;
        };
        let Some(scene_id) = project.default_scene_id() else {
            self.world = GameWorld::new();
            self.entity_textures.clear();
            self.entity_roles.clear();
            return;
        };

        let Some(scene) = project.scenes.get(&scene_id) else {
            self.world = GameWorld::new();
            self.entity_textures.clear();
            self.entity_roles.clear();
            return;
        };

        // The scene data is the source of truth; the world is built from it.
        self.scene = Scene::from_project_entities(&scene.entities);
        self.rebuild_world();
    }

    /// The id of the scene the editor is editing.
    fn edited_scene_id(&self) -> String {
        self.current_project
            .as_ref()
            .and_then(|p| p.default_scene_id())
            .unwrap_or_else(|| "main_scene".to_string())
    }

    /// Rebuild the runtime world from the edited scene.
    ///
    /// The world is what the renderer draws, so it has to follow every edit.
    fn rebuild_world(&mut self) {
        let textures =
            scene_ops::world_from_scene(&mut self.world, &self.scene, &self.texture_sizes);
        self.entity_textures = textures;

        self.entity_roles.clear();
        for entity in &self.scene.entities {
            if let Some(role) = &entity.extras.role {
                self.entity_roles.insert(entity.id.clone(), role.clone());
            }
        }
    }

    /// Record the current scene so the next edit can be undone.
    fn record_history(&mut self) {
        self.history.record(&self.scene);
    }

    /// Apply an edit to the scene and rebuild the world.
    ///
    /// The edit runs on the scene data; the world is rebuilt afterwards, so
    /// the inspector, the world and the undo stack never disagree.
    ///
    /// Edits are ignored while the game is playing, because the world then
    /// belongs to the running game.
    fn apply_edit(&mut self, edit: impl FnOnce(&mut Scene)) {
        if self.is_playing {
            self.console_log
                .push("[WARN] Stop the game before editing the scene".to_string());
            return;
        }
        self.record_history();
        edit(&mut self.scene);
        self.rebuild_world();
    }

    /// Undo the last edit.
    fn undo(&mut self) {
        let current = self.scene.clone();
        let Some(previous) = self.history.undo(&current) else {
            return;
        };
        self.scene = previous;
        self.rebuild_world();
    }

    /// Redo the last undone edit.
    fn redo(&mut self) {
        let current = self.scene.clone();
        let Some(next) = self.history.redo(&current) else {
            return;
        };
        self.scene = next;
        self.rebuild_world();
    }

    /// Add a new entity to the edited scene and select it.
    fn add_entity(&mut self) {
        let name = format!("Entity_{}", self.next_entity_name);
        self.next_entity_name += 1;

        let mut id = String::new();
        self.apply_edit(|scene| {
            id = scene.add_entity(SceneEntity::new(&name, &name));
        });

        // Select the new entity so the inspector shows it right away.
        self.selected_id = Some(id);
    }

    /// Remove the selected entity from the scene.
    fn remove_selected_entity(&mut self) {
        let Some(id) = self.selected_id.clone() else {
            return;
        };
        self.apply_edit(|scene| {
            scene.remove_entity(&id);
        });
        self.selected_id = None;
    }

    /// Remove a component from the selected entity.
    fn remove_component(&mut self, kind: ComponentKind) {
        let Some(id) = self.selected_id.clone() else {
            return;
        };
        self.apply_edit(|scene| {
            scene.remove_component(&id, kind);
        });
    }

    /// Add a component to the selected entity.
    ///
    /// The component that is added is the one the entity has the least of:
    /// a role first, then a sprite. The transform is always there.
    fn add_component(&mut self) {
        let Some(id) = self.selected_id.clone() else {
            return;
        };
        self.apply_edit(|scene| {
            let Some(entity) = scene.entity_mut(&id) else {
                return;
            };
            if entity.extras.role.is_none() {
                entity.extras.role = Some("entity".to_string());
            } else if entity.color.is_none() {
                entity.color = Some([1.0, 1.0, 1.0, 1.0]);
            }
        });
    }

    /// Start an export of the project in the background.
    ///
    /// The command line tool does the work in a thread, so the editor stays
    /// responsive. The result is read back into the console panel.
    fn start_export(&mut self) {
        let platform = self.export_platform;
        self.show_export = false;

        // The project has to be saved first, so the export has the latest
        // version of the scene.
        if let Err(e) = self.save_project() {
            self.console_log
                .push(format!("[ERROR] Could not save the project: {e}"));
            return;
        }

        self.console_log.push(format!(
            "[INFO] Exporting for {} (see the terminal for progress)",
            platform.label()
        ));

        let output = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join("export");

        let (sender, receiver) = std::sync::mpsc::channel();
        self.export_result = Some(receiver);

        // The thread does the work; a failure to start is rare, and the
        // result channel stays empty in that case.
        drop(std::thread::spawn(move || {
            let status = std::process::Command::new("cargo")
                .args([
                    "run",
                    "--release",
                    "--bin",
                    "kiln-cli",
                    "--",
                    "export",
                    "--platform",
                    platform.cli_name(),
                    "--output",
                    &output.to_string_lossy(),
                ])
                .status();

            let message = match status {
                Ok(status) if status.success() => {
                    format!("[INFO] Export for {} finished", platform.label())
                }
                Ok(status) => {
                    format!("[ERROR] Export failed with status {status}")
                }
                Err(e) => format!("[ERROR] Could not run the export: {e}"),
            };
            let _ = sender.send(message);
        }));
    }

    /// Read the result of a finished export into the console.
    fn poll_export_result(&mut self) {
        let Some(receiver) = &self.export_result else {
            return;
        };
        match receiver.try_recv() {
            Ok(message) => {
                self.console_log.push(message);
                self.export_result = None;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.export_result = None;
            }
        }
    }

    /// The HUD of the scene view, as text drawn with the engine's renderer.
    ///
    /// The player draws the same information the same way, so the editor and
    /// the game show the same pixels.
    fn scene_view_hud_text(&self) -> Vec<SpriteInstance> {
        let Some(play) = self.play.as_ref() else {
            return Vec::new();
        };

        let state = play.slice.state();
        let text = kiln_ui::TextRenderer::new();
        let mut lines = vec![format!("{} - SCORE {}", play.scene_id, state.score)];
        if !state.status.is_empty() {
            lines.push(state.status.clone());
        }
        lines.push("ARROWS MOVE   F5 SAVE   F9 LOAD".to_string());

        let mut instances = Vec::new();
        for (index, line) in lines.iter().enumerate() {
            let y = 16.0 + index as f32 * 20.0;
            instances.extend(text.render(line, 16.0, y, 16.0, [1.0, 1.0, 1.0, 1.0]));
        }
        instances
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

    /// Write the edited scene into the project data.
    ///
    /// The scene keeps its id, name, script and nested scenes, so saving does
    /// not break the game.
    fn sync_scene_into_project(&mut self) -> Result<(), String> {
        // The scene keeps its id and script, so saving does not break the game.
        let scene_id = self.edited_scene_id();
        let script = self
            .current_project
            .as_ref()
            .and_then(|p| p.scenes.get(&scene_id))
            .and_then(|s| s.script.clone());
        let name = self
            .current_project
            .as_ref()
            .and_then(|p| p.scenes.get(&scene_id))
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "Main Scene".to_string());
        // Nested scenes belong to the scene, so they survive a save.
        let nested_scenes = self
            .current_project
            .as_ref()
            .and_then(|p| p.scenes.get(&scene_id))
            .map(|s| s.nested_scenes.clone())
            .unwrap_or_default();

        if let Some(project) = &mut self.current_project {
            use kiln_core::project::Scene;
            let scene = Scene {
                id: scene_id.clone(),
                name,
                entities: self.scene.to_project_entities(),
                nested_scenes,
                script,
            };

            project.scenes.insert(scene_id, scene);
            Ok(())
        } else {
            Err("No project loaded".to_string())
        }
    }

    /// Save the current world state as a scene.
    ///
    /// The scene is written into the project and the project file is written
    /// to disk, because a scene lives inside the project file.
    pub fn save_scene(&mut self) -> Result<(), String> {
        self.sync_scene_into_project()?;
        self.write_project_file()
    }

    /// Write the project data to the project file.
    fn write_project_file(&mut self) -> Result<(), String> {
        let path = self
            .project_path
            .as_ref()
            .ok_or("No project file open. Use Save As first.")?
            .clone();

        let Some(project) = &mut self.current_project else {
            return Err("No project loaded".to_string());
        };

        let toml_str = project.to_toml().map_err(|e| e.to_string())?;
        std::fs::write(&path, toml_str).map_err(|e| e.to_string())?;
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
        let fps = if self.frame_time > 0.0 {
            1.0 / self.frame_time
        } else {
            0.0
        };
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

        // The HUD text is built before the GPU is borrowed, because the
        // renderer owns the mutable borrow of the editor.
        let hud_text = self.scene_view_hud_text();

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
                        menu_actions.push(MenuAction::Export);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        std::process::exit(0);
                    }
                });

                ui.menu_button("Edit", |ui| {
                    if ui.button("Undo").clicked() {
                        menu_actions.push(MenuAction::Undo);
                        ui.close_menu();
                    }
                    if ui.button("Redo").clicked() {
                        menu_actions.push(MenuAction::Redo);
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
                    if ui.button("Tilemap Editor").clicked() {
                        self.dock_state.add_window(vec![Tab::TilemapEditor]);
                        ui.close_menu();
                    }
                });

                ui.menu_button("Project", |ui| {
                    if ui.button("Build Settings").clicked() {
                        self.dock_state.add_window(vec![Tab::Settings]);
                        ui.close_menu();
                    }
                    if ui.button("Package").clicked() {
                        menu_actions.push(MenuAction::Export);
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
                        // The documentation lives in the repository.
                        let url = "https://github.com/kiln-engine/kiln/tree/master/docs";
                        if let Err(e) = open_in_browser(url) {
                            log::warn!("could not open the documentation: {e}");
                            self.console_log
                                .push(format!("[INFO] Documentation: {url}"));
                        }
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

                // The scene view writes its rectangle during the pass. It is
                // cleared first, so a hidden scene view does not draw the
                // scene over the panel that replaced it.
                self.scene_view_rect = None;

                let mut tab_viewer = TabViewer {
                    scene: &mut self.scene,
                    history: &mut self.history,
                    scene_before_edit: &mut self.scene_before_edit,
                    scene_dirty: &mut self.scene_dirty,
                    add_entity_requested: &mut self.add_entity_requested,
                    remove_entity_requested: &mut self.remove_entity_requested,
                    remove_component_requested: &mut self.remove_component_requested,
                    add_component_requested: &mut self.add_component_requested,
                    selected_id: &mut self.selected_id,
                    is_playing: &mut self.is_playing,
                    graph: &mut self.graph,
                    render_tier: &mut self.render_tier,
                    pass_timings: &pass_timings,
                    profiler_supported,
                    ui_designer: &mut self.ui_designer,
                    prefab_editor: &mut self.prefab_editor,
                    gameplay_editor: &mut self.gameplay_editor,
                    asset_browser: &mut self.asset_browser,
                    project_root: self.project_root.as_deref(),
                    import_path: &mut self.import_path,
                    tilemap_editor: &mut self.tilemap_editor,
                    current_project: &mut self.current_project,
                    play_toggle_requested: &mut self.play_toggle_requested,
                    scene_save_requested: &mut self.scene_save_requested,
                    scene_load_requested: &mut self.scene_load_requested,
                    console_log: &mut self.console_log,
                    scene_view_rect: &mut self.scene_view_rect,
                    fps,
                };
                tree.show(ctx, &mut tab_viewer);
            });

            // Status bar
            show_status_bar(ctx, fps, entity_count, tier, is_playing);

            // The "About Kiln" window.
            if self.show_about {
                egui::Window::new("About Kiln")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                    .show(ctx, |ui| {
                        ui.heading("Kiln");
                        ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                        ui.add_space(8.0);
                        ui.label("A small, fast game engine for 2D games.");
                        ui.label("Rust · wgpu · egui");
                        ui.add_space(8.0);
                        ui.hyperlink_to(
                            "Documentation",
                            "https://github.com/kiln-engine/kiln/tree/master/docs",
                        );
                        ui.add_space(8.0);
                        if ui.button("Close").clicked() {
                            self.show_about = false;
                        }
                    });
            }

            // The export dialog.
            if self.show_export {
                egui::Window::new("Export")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                    .show(ctx, |ui| {
                        ui.heading("Export");
                        ui.add_space(8.0);

                        ui.label("Platform");
                        ui.horizontal(|ui| {
                            for platform in ExportPlatform::all() {
                                ui.selectable_value(
                                    &mut self.export_platform,
                                    platform,
                                    platform.label(),
                                );
                            }
                        });

                        ui.add_space(8.0);
                        ui.label("The export runs the command line tool in the background.");
                        ui.label("Its output is shown in the console panel.");

                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui.button("Export").clicked() {
                                self.export_requested = true;
                            }
                            if ui.button("Close").clicked() {
                                self.show_export = false;
                            }
                        });
                    });
            }
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

        // Render the scene into the scene view panel using the real GPU
        // pipeline, the same one the standalone player uses.
        //
        // The viewport is set to the panel rectangle, so the world is drawn
        // exactly where the panel is, and the camera uses the player's
        // convention (world Y grows upwards).
        if let Some(rect) = self.scene_view_rect {
            let device = gpu.renderer.device();
            let queue = gpu.renderer.queue();

            // egui works in points; the framebuffer works in pixels.
            let scale = full_output.pixels_per_point;
            let viewport_x = (rect.min.x * scale).round() as u32;
            let viewport_y = (rect.min.y * scale).round() as u32;
            let viewport_w = (rect.width() * scale).round().max(1.0) as u32;
            let viewport_h = (rect.height() * scale).round().max(1.0) as u32;

            // One world unit is one pixel, like in the player.
            gpu.scene_view_camera.viewport_size = [viewport_w as f32, viewport_h as f32];
            gpu.scene_view_camera.position = [0.0, 0.0];
            gpu.scene_view_hud_camera.viewport_size = [viewport_w as f32, viewport_h as f32];
            gpu.scene_view_hud_camera.position = [0.0, 0.0];

            // Upload textures and push sprites to the sprite renderer.
            gpu.scene_view_sprite_renderer.begin_frame();
            for (source_id, transform, sprite) in self
                .world
                .world()
                .query::<(&SourceId, &Transform, &Sprite)>()
                .iter()
            {
                let path = match self.entity_textures.get(&source_id.0) {
                    Some(p) => p.clone(),
                    None => continue,
                };
                let texture_id = *self
                    .scene_view_texture_ids
                    .entry(path.clone())
                    .or_insert_with(|| {
                        let Some(root) = self.project_root.as_ref() else {
                            return TextureId::WHITE;
                        };
                        let full_path = root.join(&path);
                        let bytes = match std::fs::read(&full_path) {
                            Ok(b) => b,
                            Err(_) => return TextureId::WHITE,
                        };
                        match kiln_assets::decode_png(&bytes) {
                            Ok(image) => gpu.scene_view_sprite_renderer.create_texture_rgba(
                                device,
                                queue,
                                &path,
                                image.width,
                                image.height,
                                &image.pixels,
                            ),
                            Err(_) => TextureId::WHITE,
                        }
                    });
                gpu.scene_view_sprite_renderer.push_textured(
                    texture_id,
                    SpriteInstance {
                        position: [transform.position.x, transform.position.y],
                        size: sprite.size,
                        color: sprite.color,
                        uv_rect: sprite.uv_rect,
                    },
                );
            }

            // The HUD is drawn in screen space with the engine's own text
            // renderer, so the editor shows the same pixels as the player.
            gpu.scene_view_hud_renderer.begin_frame();
            for instance in hud_text {
                gpu.scene_view_hud_renderer.push(instance);
            }

            // Render the scene and its HUD into the scene view rectangle.
            //
            // The viewport maps the scene onto the panel, and the scissor
            // keeps the drawing inside it.
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene_view_pass"),
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
                viewport_x as f32,
                viewport_y as f32,
                viewport_w as f32,
                viewport_h as f32,
                0.0,
                1.0,
            );
            pass.set_scissor_rect(viewport_x, viewport_y, viewport_w, viewport_h);
            gpu.scene_view_sprite_renderer
                .flush(&mut pass, &gpu.scene_view_camera, queue);
            gpu.scene_view_hud_renderer
                .flush(&mut pass, &gpu.scene_view_hud_camera, queue);
            drop(pass);
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
                    self.undo();
                }
                MenuAction::Redo => {
                    self.redo();
                }
                MenuAction::About => {
                    self.show_about = true;
                }
                MenuAction::Export => {
                    self.show_export = true;
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

        // Undo and redo shortcuts, as advertised in the status bar.
        {
            let ctx = self.egui_winit.egui_ctx();
            let mut undo = false;
            let mut redo = false;
            ctx.input(|i| {
                if i.modifiers.ctrl && !i.modifiers.shift {
                    if i.key_pressed(egui::Key::Z) {
                        undo = true;
                    }
                    if i.key_pressed(egui::Key::Y) {
                        redo = true;
                    }
                }
            });
            if undo {
                self.undo();
            }
            if redo {
                self.redo();
            }
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

        // Handle the entity and component edits the panels asked for.
        if std::mem::take(&mut self.add_entity_requested) {
            self.add_entity();
        }
        if std::mem::take(&mut self.remove_entity_requested) {
            self.remove_selected_entity();
        }
        if let Some(kind) = self.remove_component_requested.take() {
            self.remove_component(kind);
        }
        if std::mem::take(&mut self.add_component_requested) {
            self.add_component();
        }

        // A panel changed the scene, so the world has to follow. While the
        // game is playing the world belongs to the game, so it is left alone.
        if std::mem::take(&mut self.scene_dirty) {
            if self.is_playing {
                self.console_log
                    .push("[WARN] Stop the game before editing the scene".to_string());
            } else {
                self.rebuild_world();
            }
        }

        // Start an export the dialog asked for, and read the result of a
        // running one.
        if std::mem::take(&mut self.export_requested) {
            self.start_export();
        }
        self.poll_export_result();

        // Update the FPS counter from the real frame time.
        self.frame_count += 1;
        let now = std::time::Instant::now();
        if let Some(last) = self.last_frame {
            let elapsed = now.duration_since(last).as_secs_f32();
            // Ignore huge gaps (e.g. after the window was minimised) so the
            // average does not collapse.
            if elapsed > 0.0 && elapsed < 1.0 {
                self.frame_time = self.frame_time * 0.9 + elapsed * 0.1;
            }
        }
        self.last_frame = Some(now);
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
    /// The scene being edited; the inspector changes it directly.
    scene: &'a mut Scene,
    /// The undo stack, so an edit can be recorded before it happens.
    history: &'a mut History,
    /// The scene as it was before the edit in progress.
    scene_before_edit: &'a mut Option<Scene>,
    /// Set when the scene was changed, so the world can be rebuilt.
    scene_dirty: &'a mut bool,
    /// Set when the user asked to add an entity to the scene.
    add_entity_requested: &'a mut bool,
    /// Set when the user asked to remove the selected entity.
    remove_entity_requested: &'a mut bool,
    /// Set when the user asked to remove a component of the selected entity.
    remove_component_requested: &'a mut Option<ComponentKind>,
    /// Set when the user asked to add a component to the selected entity.
    add_component_requested: &'a mut bool,
    selected_id: &'a mut Option<String>,
    is_playing: &'a mut bool,
    graph: &'a mut Graph,
    render_tier: &'a mut RenderTier,
    pass_timings: &'a [PassTiming],
    profiler_supported: bool,
    ui_designer: &'a mut UIDesignerState,
    prefab_editor: &'a mut PrefabEditorState,
    gameplay_editor: &'a mut GameplayEditorState,
    asset_browser: &'a mut AssetBrowser,
    /// Directory of the open project, where the assets live.
    project_root: Option<&'a Path>,
    /// The path typed into the asset import field.
    import_path: &'a mut String,
    tilemap_editor: &'a mut TilemapEditorState,
    current_project: &'a mut Option<Project>,
    play_toggle_requested: &'a mut bool,
    scene_save_requested: &'a mut bool,
    scene_load_requested: &'a mut bool,
    /// Lines shown in the console panel.
    console_log: &'a mut Vec<String>,
    /// Screen-space rectangle of the scene view panel.
    ///
    /// The scene view writes its canvas rectangle here, so the renderer can
    /// draw the scene into the right part of the window.
    scene_view_rect: &'a mut Option<egui::Rect>,
    /// The real frame rate of the editor, shown in the scene view toolbar.
    fps: f32,
}

impl<'a> TabViewer<'a> {
    /// Take the snapshot of the scene before the inspector changes it.
    ///
    /// The snapshot is only made when there is none, so an idle editor does
    /// not copy the scene every frame.
    fn snapshot_before_edit(&mut self) {
        if self.scene_before_edit.is_none() {
            *self.scene_before_edit = Some(self.scene.clone());
        }
    }

    /// Make a prefab out of the selected entity.
    fn create_prefab_from_selection(&mut self) {
        let Some(id) = self.selected_id.clone() else {
            self.console_log
                .push("[WARN] Select an entity before creating a prefab".to_string());
            return;
        };
        let Some(entity) = self.scene.entity(&id) else {
            return;
        };

        let project_entity = entity.to_project_entity();
        let name = entity.name.clone();
        let prefab = kiln_core::Prefab::new(name.clone(), project_entity);
        let prefab_id = prefab.id.clone();
        self.prefab_editor.add_prefab(prefab);
        self.prefab_editor.select_prefab(&prefab_id);
        self.console_log
            .push(format!("[INFO] Created prefab '{name}'"));
    }

    /// Put an instance of the selected prefab into the scene.
    fn instantiate_selected_prefab(&mut self) {
        let Some(prefab_id) = self.prefab_editor.selected_prefab().map(str::to_string) else {
            self.console_log
                .push("[WARN] Select a prefab before instantiating it".to_string());
            return;
        };
        let Some(instance) = self.prefab_editor.instantiate_prefab(&prefab_id, None) else {
            return;
        };

        let entity = SceneEntity::from_project_entity(&instance);
        let name = entity.name.clone();
        let id = self.scene.add_entity(entity);
        *self.selected_id = Some(id);
        *self.scene_dirty = true;
        self.console_log
            .push(format!("[INFO] Instantiated prefab '{name}'"));
    }

    /// Copy an image into the project and refresh the asset list.
    ///
    /// The path can be absolute or relative to the project directory. The
    /// file is copied into the project's `art` folder, so the project stays
    /// self-contained.
    fn import_asset(&mut self) {
        let Some(root) = self.project_root.map(|p| p.to_path_buf()) else {
            self.console_log
                .push("[WARN] Open a project before importing an asset".to_string());
            return;
        };

        let source = self.import_path.trim().to_string();
        if source.is_empty() {
            self.console_log
                .push("[WARN] Type the path of the file to import".to_string());
            return;
        }

        let source_path = if Path::new(&source).is_absolute() {
            PathBuf::from(&source)
        } else {
            root.join(&source)
        };

        if !source_path.is_file() {
            self.console_log
                .push(format!("[ERROR] '{source}' is not a file"));
            return;
        }

        let Some(file_name) = source_path.file_name() else {
            self.console_log
                .push(format!("[ERROR] '{source}' has no file name"));
            return;
        };

        let target_dir = root.join("art");
        if let Err(e) = std::fs::create_dir_all(&target_dir) {
            self.console_log
                .push(format!("[ERROR] Could not create the art folder: {e}"));
            return;
        }
        let target = target_dir.join(file_name);

        match std::fs::copy(&source_path, &target) {
            Ok(_) => {
                let relative = target
                    .strip_prefix(&root)
                    .unwrap_or(&target)
                    .to_string_lossy()
                    .replace('\\', "/");
                self.console_log
                    .push(format!("[INFO] Imported '{relative}'"));
                self.import_path.clear();
                self.asset_browser.scan(&root);
            }
            Err(e) => self
                .console_log
                .push(format!("[ERROR] Could not import the file: {e}")),
        }
    }

    /// The id of the scene the editor is editing.
    fn current_scene_nested_scenes(&self) -> Vec<kiln_core::project::NestedScene> {
        let Some(project) = self.current_project.as_ref() else {
            return Vec::new();
        };
        let Some(scene_id) = project.default_scene_id() else {
            return Vec::new();
        };
        project
            .scenes
            .get(&scene_id)
            .map(|scene| scene.nested_scenes.clone())
            .unwrap_or_default()
    }

    /// Add an empty scene to the project.
    fn create_scene(&mut self) {
        let Some(project) = self.current_project.as_mut() else {
            self.console_log
                .push("[WARN] Open a project before creating a scene".to_string());
            return;
        };

        // The id has to be unique inside the project.
        let mut counter = project.scenes.len() + 1;
        let mut id = format!("scene_{counter}");
        while project.scenes.contains_key(&id) {
            counter += 1;
            id = format!("scene_{counter}");
        }

        let scene = kiln_core::project::Scene {
            id: id.clone(),
            name: format!("Scene {counter}"),
            entities: Vec::new(),
            nested_scenes: Vec::new(),
            script: None,
        };
        project.scenes.insert(id.clone(), scene);
        self.console_log
            .push(format!("[INFO] Created scene '{id}'"));
    }

    /// Make a scene the one the game starts in.
    fn set_start_scene(&mut self, id: &str) {
        let Some(project) = self.current_project.as_mut() else {
            return;
        };
        if !project.scenes.contains_key(id) {
            return;
        }
        // The start scene is the first one by id, so the id is renamed to
        // sort before the others.
        let mut ids: Vec<String> = project.scenes.keys().cloned().collect();
        ids.sort();
        if ids.first().map(String::as_str) == Some(id) {
            return;
        }
        let scene = project.scenes.remove(id).expect("the scene exists");
        project.scenes.insert(format!("0_{id}"), scene);
    }

    /// Remove a scene from the project.
    fn delete_scene(&mut self, id: &str) {
        let Some(project) = self.current_project.as_mut() else {
            return;
        };
        if project.scenes.len() <= 1 {
            self.console_log
                .push("[WARN] A project needs at least one scene".to_string());
            return;
        }
        if project.scenes.remove(id).is_some() {
            self.console_log
                .push(format!("[INFO] Deleted scene '{id}'"));
        }
    }

    /// Add a nested scene to the scene being edited.
    fn add_nested_scene(&mut self, scene_id: &str) {
        let Some(project) = self.current_project.as_mut() else {
            return;
        };
        let Some(current_id) = project.default_scene_id() else {
            return;
        };
        let Some(current) = project.scenes.get_mut(&current_id) else {
            return;
        };
        // A scene cannot contain itself.
        if scene_id == current_id {
            return;
        }
        if current.nested_scenes.iter().any(|n| n.scene_id == scene_id) {
            return;
        }
        current.nested_scenes.push(kiln_core::project::NestedScene {
            scene_id: scene_id.to_string(),
            additive: false,
            offset: kiln_core::project::SceneOffset::default(),
        });
    }

    /// Remove a nested scene from the scene being edited.
    fn remove_nested_scene(&mut self, scene_id: &str) {
        let Some(project) = self.current_project.as_mut() else {
            return;
        };
        let Some(current_id) = project.default_scene_id() else {
            return;
        };
        let Some(current) = project.scenes.get_mut(&current_id) else {
            return;
        };
        current.nested_scenes.retain(|n| n.scene_id != scene_id);
    }

    /// The button that adds a nested scene, with a list of the other scenes.
    fn add_nested_scene_button(&mut self, ui: &mut egui::Ui) {
        let candidates: Vec<String> = self
            .current_project
            .as_ref()
            .map(|project| {
                let current = project.default_scene_id();
                project
                    .scenes
                    .keys()
                    .filter(|id| Some((*id).clone()) != current)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        if candidates.is_empty() {
            ui.label("Create another scene to nest it here.");
            return;
        }

        let mut selected = candidates.first().cloned().unwrap_or_default();
        egui::ComboBox::from_label("Scene")
            .selected_text(selected.clone())
            .show_ui(ui, |ui| {
                for candidate in &candidates {
                    ui.selectable_value(&mut selected, candidate.clone(), candidate);
                }
            });

        if ui.button("Add Nested Scene").clicked() {
            self.add_nested_scene(&selected);
        }
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
                            ui.label(format!("FPS: {:.0}", self.fps));
                        });
                    });

                    ui.add_space(8.0);

                    // Game view canvas
                    let desired_size =
                        egui::vec2(ui.available_width(), ui.available_height() * 0.65);
                    let (rect, _response) =
                        ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());

                    // The renderer draws the scene into this rectangle.
                    *self.scene_view_rect = Some(rect);

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

                    // Entity list for selection
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.heading("Entities");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("+ Add Entity").clicked() {
                                *self.add_entity_requested = true;
                            }
                        });
                    });

                    ui.add_space(4.0);

                    egui::ScrollArea::vertical()
                        .max_height(ui.available_height())
                        .show(ui, |ui| {
                            let entities: Vec<(String, String, [f32; 2])> = self
                                .scene
                                .entities
                                .iter()
                                .map(|e| {
                                    (
                                        e.id.clone(),
                                        e.name.clone(),
                                        [e.transform.position.x, e.transform.position.y],
                                    )
                                })
                                .collect();
                            for (id, name, position) in entities {
                                let label =
                                    format!("{} ({:.0}, {:.0})", name, position[0], position[1]);
                                let selected = self.selected_id.as_deref() == Some(&id);
                                if ui.selectable_label(selected, label).clicked() {
                                    *self.selected_id = Some(id);
                                }
                            }
                        });
                });
            }
            Tab::Inspector => {
                // The scene is snapshotted before it is changed, so the
                // change can be undone.
                self.snapshot_before_edit();

                ui.vertical(|ui| {
                    ui.heading("Inspector");

                    // The scene data is the source of truth, so the inspector
                    // edits it directly. The world is rebuilt afterwards.
                    let Some(id) = self.selected_id.clone() else {
                        ui.add_space(32.0);
                        ui.centered_and_justified(|ui| {
                            ui.label("No entity selected");
                            ui.add_space(8.0);
                            ui.label(
                                "Select an entity from the Scene View to inspect its components",
                            );
                        });
                        return;
                    };

                    let Some(entity) = self.scene.entity_mut(&id) else {
                        ui.add_space(32.0);
                        ui.centered_and_justified(|ui| {
                            ui.label("The selected entity no longer exists");
                        });
                        return;
                    };

                    ui.label(format!("Entity: {}", entity.name));
                    ui.add_space(8.0);

                    // Transform component
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
                                            *self.remove_component_requested =
                                                Some(ComponentKind::Transform);
                                        }
                                    },
                                );
                            });
                            ui.add_space(8.0);

                            let transform = &mut entity.transform;
                            ui.horizontal(|ui| {
                                ui.label("Position");
                                ui.add_space(16.0);
                                ui.label("X:");
                                if ui
                                    .add(egui::DragValue::new(&mut transform.position.x).speed(1.0))
                                    .changed()
                                {
                                    record_edit(
                                        &mut *self.history,
                                        &mut *self.scene_before_edit,
                                        "transform.x",
                                    );
                                    *self.scene_dirty = true;
                                }
                                ui.add_space(8.0);
                                ui.label("Y:");
                                if ui
                                    .add(egui::DragValue::new(&mut transform.position.y).speed(1.0))
                                    .changed()
                                {
                                    record_edit(
                                        &mut *self.history,
                                        &mut *self.scene_before_edit,
                                        "transform.y",
                                    );
                                    *self.scene_dirty = true;
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.label("Rotation");
                                ui.add_space(16.0);
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut transform.rotation)
                                            .speed(0.01)
                                            .range(-std::f32::consts::PI..=std::f32::consts::PI),
                                    )
                                    .changed()
                                {
                                    record_edit(
                                        &mut *self.history,
                                        &mut *self.scene_before_edit,
                                        "transform.rotation",
                                    );
                                    *self.scene_dirty = true;
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.label("Scale");
                                ui.add_space(16.0);
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut transform.scale.x)
                                            .speed(0.01)
                                            .range(0.01..=100.0),
                                    )
                                    .changed()
                                {
                                    record_edit(
                                        &mut *self.history,
                                        &mut *self.scene_before_edit,
                                        "transform.scale",
                                    );
                                    transform.scale.y = transform.scale.x;
                                    *self.scene_dirty = true;
                                }
                            });
                        });

                    // Sprite component
                    if let Some(color) = entity.color.as_mut() {
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
                                                *self.remove_component_requested =
                                                    Some(ComponentKind::Sprite);
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
                                            egui::DragValue::new(&mut color[0])
                                                .speed(0.01)
                                                .range(0.0..=1.0),
                                        )
                                        .changed()
                                    {
                                        record_edit(
                                            &mut *self.history,
                                            &mut *self.scene_before_edit,
                                            "color.r",
                                        );
                                        *self.scene_dirty = true;
                                    }
                                    ui.add_space(8.0);
                                    ui.label("G:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut color[1])
                                                .speed(0.01)
                                                .range(0.0..=1.0),
                                        )
                                        .changed()
                                    {
                                        record_edit(
                                            &mut *self.history,
                                            &mut *self.scene_before_edit,
                                            "color.g",
                                        );
                                        *self.scene_dirty = true;
                                    }
                                    ui.add_space(8.0);
                                    ui.label("B:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut color[2])
                                                .speed(0.01)
                                                .range(0.0..=1.0),
                                        )
                                        .changed()
                                    {
                                        record_edit(
                                            &mut *self.history,
                                            &mut *self.scene_before_edit,
                                            "color.b",
                                        );
                                        *self.scene_dirty = true;
                                    }
                                    ui.add_space(8.0);
                                    ui.label("A:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut color[3])
                                                .speed(0.01)
                                                .range(0.0..=1.0),
                                        )
                                        .changed()
                                    {
                                        record_edit(
                                            &mut *self.history,
                                            &mut *self.scene_before_edit,
                                            "color.a",
                                        );
                                        *self.scene_dirty = true;
                                    }
                                });

                                ui.add_space(8.0);

                                // Color preview
                                let color_rect = egui::Rect::from_min_size(
                                    ui.cursor().left_top(),
                                    egui::vec2(40.0, 24.0),
                                );
                                let preview_color = egui::Color32::from_rgba_premultiplied(
                                    (color[0] * 255.0) as u8,
                                    (color[1] * 255.0) as u8,
                                    (color[2] * 255.0) as u8,
                                    (color[3] * 255.0) as u8,
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

                    // Role component
                    ui.add_space(8.0);
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(35, 35, 40))
                        .rounding(4.0)
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.strong("Role");
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.button("⋮").clicked() {
                                            *self.remove_component_requested =
                                                Some(ComponentKind::Role);
                                        }
                                    },
                                );
                            });
                            ui.add_space(8.0);

                            let mut role = entity.extras.role.clone().unwrap_or_default();
                            ui.horizontal(|ui| {
                                ui.label("Name");
                                ui.add_space(16.0);
                                if ui.text_edit_singleline(&mut role).changed() {
                                    record_edit(
                                        &mut *self.history,
                                        &mut *self.scene_before_edit,
                                        "role",
                                    );
                                    entity.extras.role = if role.is_empty() {
                                        None
                                    } else {
                                        Some(role.clone())
                                    };
                                    *self.scene_dirty = true;
                                }
                            });
                            ui.label("A role lets a script find this entity, e.g. \"player\".");
                        });

                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        if ui.button("+ Add Component").clicked() {
                            *self.add_component_requested = true;
                        }
                        if ui.button("Delete Entity").clicked() {
                            *self.remove_entity_requested = true;
                        }
                    });
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

                    // Import an image into the project.
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(35, 35, 40))
                        .rounding(4.0)
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.strong("Import");
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                ui.label("File");
                                ui.add_space(16.0);
                                ui.add(
                                    egui::TextEdit::singleline(self.import_path)
                                        .desired_width(220.0)
                                        .hint_text("path/to/image.png"),
                                );
                                if ui.button("Import").clicked() {
                                    self.import_asset();
                                }
                            });
                            ui.label("The file is copied into the project's art folder.");
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
                                        ui.label(line);
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
                                    let selected = self.prefab_editor.selected_prefab().map(str::to_string);

                                    for (prefab_id, prefab_name) in prefab_ids {
                                        ui.horizontal(|ui| {
                                            let is_selected =
                                                selected.as_deref() == Some(prefab_id.as_str());
                                            if ui
                                                .selectable_label(is_selected, &prefab_name)
                                                .clicked()
                                            {
                                                self.prefab_editor.select_prefab(&prefab_id);
                                            }
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if ui.button("Delete").clicked() {
                                                        self.prefab_editor.remove_prefab(&prefab_id);
                                                    }
                                                },
                                            );
                                        });
                                        ui.add_space(4.0);
                                    }

                                    if is_empty {
                                        ui.label("No prefabs created yet");
                                        ui.label("Select an entity and click 'Create Prefab from Selection'");
                                    }
                                });
                        });

                    // The properties of the selected prefab.
                    if let Some(prefab_id) = self.prefab_editor.selected_prefab().map(str::to_string)
                    {
                        let name = self
                            .prefab_editor
                            .get_prefab(&prefab_id)
                            .map(|p| p.name.clone())
                            .unwrap_or_default();

                        ui.add_space(8.0);
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(35, 35, 40))
                            .rounding(4.0)
                            .inner_margin(12.0)
                            .show(ui, |ui| {
                                ui.strong("Prefab");
                                ui.add_space(8.0);

                                ui.horizontal(|ui| {
                                    ui.label("Name");
                                    ui.add_space(16.0);
                                    let mut edit_name = name.clone();
                                    if ui.text_edit_singleline(&mut edit_name).changed() {
                                        self.prefab_editor.rename_prefab(&prefab_id, &edit_name);
                                    }
                                });

                                let entity = self
                                    .prefab_editor
                                    .get_prefab(&prefab_id)
                                    .map(|p| p.root_entity.clone());
                                if let Some(entity) = entity {
                                    ui.add_space(8.0);
                                    ui.label(format!("Entity id: {}", entity.id));
                                    ui.label(format!(
                                        "Components: {}",
                                        entity.components.len()
                                    ));
                                }
                            });
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

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label("Scene Composition");
                        });
                    });

                    ui.add_space(8.0);

                    // The scenes of the project.
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(35, 35, 40))
                        .rounding(4.0)
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.strong("Scenes");
                            ui.add_space(8.0);

                            let scenes: Vec<(String, String, usize)> = self
                                .current_project
                                .as_ref()
                                .map(|project| {
                                    project
                                        .scenes
                                        .iter()
                                        .map(|(id, scene)| {
                                            (id.clone(), scene.name.clone(), scene.entities.len())
                                        })
                                        .collect()
                                })
                                .unwrap_or_default();

                            if scenes.is_empty() {
                                ui.label("The project has no scenes yet.");
                            }

                            let start_scene = self
                                .current_project
                                .as_ref()
                                .and_then(|project| project.default_scene_id());

                            for (id, name, entity_count) in scenes {
                                ui.horizontal(|ui| {
                                    let is_start = start_scene.as_deref() == Some(&id);
                                    let label = if is_start {
                                        format!("{name} (start)")
                                    } else {
                                        name.clone()
                                    };
                                    ui.label(label);
                                    ui.label(format!("{entity_count} entities"));

                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui.button("Set as Start").clicked() {
                                                self.set_start_scene(&id);
                                            }
                                            if ui.button("Delete").clicked() {
                                                self.delete_scene(&id);
                                            }
                                        },
                                    );
                                });
                                ui.add_space(4.0);
                            }
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

                            let nested = self.current_scene_nested_scenes();

                            if nested.is_empty() {
                                ui.label("Current scene has no nested scenes.");
                                ui.label("Use 'Add Nested Scene' to compose scenes.");
                            } else {
                                for nested_scene in nested {
                                    ui.horizontal(|ui| {
                                        ui.label(&nested_scene.scene_id);
                                        if nested_scene.additive {
                                            ui.label("(additive)");
                                        }
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui.button("Remove").clicked() {
                                                    self.remove_nested_scene(
                                                        &nested_scene.scene_id,
                                                    );
                                                }
                                            },
                                        );
                                    });
                                    ui.add_space(4.0);
                                }
                            }

                            ui.add_space(8.0);
                            self.add_nested_scene_button(ui);
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
    }
}
