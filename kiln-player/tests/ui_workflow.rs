//! UI authoring -> save/reopen -> real KAR -> shared native script/render acceptance.
use kiln_core::{ui::*, Component, Entity, Project, Scene, Transform};
use kiln_editor::{
    project_files,
    ui_layout_editor::{add_screen, add_widget, import_font, UiLayoutEditorState},
};
use kiln_ir::{HostContext, Value};
use kiln_runtime::{GameWorld, SliceSession, SourceId};
use kiln_ui::authored::{UiFonts, UiViewport};
use std::{collections::HashMap, path::Path};

fn item(id: &str, text: &str, y: f32, action: UiAction) -> UiWidget {
    UiWidget {
        id: id.into(),
        text: text.into(),
        font: "body".into(),
        event: id.into(),
        action,
        layout: UiLayout {
            anchor: [0.5, 0.5],
            pivot: [0.5, 0.5],
            offset: [0.0, y],
            size: [240.0, 36.0],
            ..Default::default()
        },
        ..Default::default()
    }
}
fn menu(name: &str, widgets: Vec<UiWidget>) -> UiScreen {
    let mut out = vec![
        UiWidget {
            id: "background".into(),
            kind: UiKind::Panel,
            layout: UiLayout {
                offset: [0.0; 2],
                size: [0.0; 2],
                stretch: [true; 2],
                ..Default::default()
            },
            color: [0.03, 0.04, 0.08, 0.97],
            ..Default::default()
        },
        UiWidget {
            id: "heading".into(),
            kind: UiKind::Label,
            text: name.into(),
            font: "body".into(),
            font_size: 30.0,
            layout: UiLayout {
                anchor: [0.5, 0.0],
                pivot: [0.5, 0.0],
                offset: [0.0, 50.0],
                size: [460.0, 50.0],
                ..Default::default()
            },
            ..Default::default()
        },
    ];
    let focus = widgets.first().unwrap().id.clone();
    out.extend(widgets);
    UiScreen {
        name: name.into(),
        default_focus: focus,
        widgets: out,
        ..Default::default()
    }
}
fn author(root: &Path) -> Project {
    let mut p = Project {
        id: "saved-ui-2d".into(),
        name: "Saved UI demo".into(),
        ..Default::default()
    };
    p.window.width = 640;
    p.window.height = 360;
    let mut editor = UiLayoutEditorState::new();
    editor
        .edit(&mut p, |doc| {
            let title = add_screen(doc, "Title")?;
            add_widget(doc.screens.get_mut(&title).unwrap(), UiKind::Button);
            Ok(())
        })
        .unwrap();
    assert!(editor.undo(&mut p));
    assert!(p.ui.screens.is_empty());
    assert!(editor.redo(&mut p));
    assert_eq!(p.ui.screens["title"].widgets.len(), 1);
    editor
        .edit(&mut p, |doc| {
            import_font(
                doc,
                root,
                Path::new(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../samples/ui2d_editor/assets/fonts/KilnSampleSans.ttf"
                )),
                "body",
            )?;
            doc.initial_screen = "title".into();
            doc.pause_screen = "pause".into();
            doc.screens.insert(
                "title".into(),
                menu(
                    "Kiln UI Demo",
                    vec![
                        item("start", "Start", 0.0, UiAction::Close),
                        item(
                            "settings",
                            "Settings",
                            48.0,
                            UiAction::ShowScreen("settings".into()),
                        ),
                    ],
                ),
            );
            doc.screens.get_mut("title").unwrap().allow_cancel = false;
            doc.screens.insert(
                "pause".into(),
                menu(
                    "Paused",
                    vec![
                        item("resume", "Resume", -20.0, UiAction::Close),
                        item(
                            "settings",
                            "Settings",
                            28.0,
                            UiAction::ShowScreen("settings".into()),
                        ),
                        item(
                            "gameover",
                            "Game over",
                            76.0,
                            UiAction::ShowScreen("game_over".into()),
                        ),
                    ],
                ),
            );
            doc.screens.insert(
                "settings".into(),
                menu(
                    "Settings",
                    vec![
                        item("sound", "Sound 25%", 0.0, UiAction::Event),
                        item("back", "Back", 48.0, UiAction::Back),
                    ],
                ),
            );
            doc.screens.insert(
                "game_over".into(),
                menu(
                    "Game Over",
                    vec![item("resume", "Resume demo", 0.0, UiAction::Close)],
                ),
            );
            Ok(())
        })
        .unwrap();
    std::fs::create_dir_all(root.join("scripts")).unwrap();
    std::fs::write(
        root.join("scripts/main.ks"),
        r#"
if script_phase() == "update" {
    move_player(120.0 * get_delta_time(), 0.0);
    state_set("root_updates", state_get("root_updates", 0) + 1);
}
if script_phase() == "ui_event" {
    state_set("root_event", ui_event_name());
    state_set("event_screen", ui_event_screen());
}
"#,
    )
    .unwrap();
    std::fs::write(root.join("scripts/ui.ks"),r#"
if script_phase() == "start" { state_set("starts", state_get("starts", 0) + 1); play_music("theme"); }
if script_phase() == "ui_event" {
    state_set("event", ui_event_name());
    state_set("widget", ui_event_widget());
    state_set("screen", ui_event_screen());
    state_set("clicked", ui_clicked(ui_event_widget()));
    if ui_event_name() == "sound" {
        set_volume(0.25); setting_set("menu_setting", "saved");
        ui_set_text("sound", "Sound saved 25%"); play_sound("hit");
    }
}
if script_phase() == "update" { state_set("ticks", state_get("ticks", 0) + 1); }
"#).unwrap();
    p.scenes.insert(
        "main".into(),
        Scene {
            id: "main".into(),
            name: "Main".into(),
            script: Some("scripts/main.ks".into()),
            nested_scenes: vec![],
            entities: vec![
                Entity {
                    id: "player".into(),
                    name: "Player".into(),
                    components: vec![
                        Component::Transform {
                            x: 40.0,
                            y: 60.0,
                            rotation: 0.0,
                            scale: 1.0,
                        },
                        Component::Role {
                            role: "player".into(),
                        },
                        Component::Sprite {
                            color: [0.2, 0.5, 1.0, 1.0],
                            width: 20.0,
                            height: 20.0,
                            texture: None,
                            material: None,
                            animation_clip: None,
                            layer: 0.0,
                        },
                    ],
                },
                Entity {
                    id: "ui".into(),
                    name: "UI controller".into(),
                    components: vec![Component::Script {
                        path: "scripts/ui.ks".into(),
                        properties: HashMap::new(),
                    }],
                },
            ],
        },
    );
    // Reuse the previously tested synthesized tones, without external media dependencies.
    for (name, music, bytes) in [
        (
            "theme",
            true,
            include_bytes!("../../samples/audio2d_editor/assets/audio/theme.wav").as_slice(),
        ),
        (
            "hit",
            false,
            include_bytes!("../../samples/audio2d_editor/assets/audio/hit.wav").as_slice(),
        ),
    ] {
        std::fs::create_dir_all(root.join("assets/audio")).unwrap();
        let path = format!("assets/audio/{name}.wav");
        std::fs::write(root.join(&path), bytes).unwrap();
        p.audio_assets.push(kiln_core::project::AudioAsset {
            id: name.into(),
            name: name.into(),
            path,
            is_music: music,
            ..Default::default()
        });
    }
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../samples/ui2d_editor/assets/fonts/LICENSE.txt"
        ),
        root.join("assets/fonts/LICENSE.txt"),
    )
    .unwrap();
    project_files::save(&p, &root.join("kiln.toml")).unwrap();
    p
}
fn setup(
    game: &kiln_player::loader::LoadedGame,
    settings: &Path,
) -> (SliceSession, GameWorld, UiFonts) {
    let mut session = SliceSession::with_scene_sources(&game.project, "main", |path| {
        String::from_utf8(game.assets[path].clone()).map_err(|e| e.to_string())
    })
    .unwrap();
    session.host_mut().set_save_dir(settings);
    #[cfg(feature = "audio")]
    assert!(session
        .host_mut()
        .configure_audio(&game.project, |path| Ok(game.assets[path].clone()))
        .is_empty());
    let (fonts, errors) = UiFonts::load(
        &game.project.ui,
        |path| Ok(game.assets[path].clone()),
        |_, _, _| kiln_render::TextureId(11),
    );
    assert!(errors.is_empty());
    let mut world = GameWorld::new();
    kiln_runtime::spawn_scene(&mut world, &game.project, "main", &mut |_, c, _| {
        kiln_runtime::Sprite::new(c[0], c[1], c[2], c[3])
    });
    (session, world, fonts)
}
fn tick(session: &mut SliceSession, world: &mut GameWorld, keys: &[&str]) {
    session
        .host_mut()
        .set_pressed_keys(&keys.iter().map(|k| k.to_string()).collect::<Vec<_>>());
    session
        .host_mut()
        .set_ui_pointer([0.0, 0.0, 640.0, 360.0], None, false);
    session.step(world, (1.0, 0.0), 1.0 / 60.0).unwrap();
}
fn x(world: &GameWorld) -> f32 {
    world
        .world_ref()
        .query::<(&SourceId, &Transform)>()
        .iter()
        .find(|(id, _)| id.0 == "player")
        .unwrap()
        .1
        .position
        .x
}
#[derive(Debug, PartialEq)]
struct UiDraw {
    texture: u32,
    position: [f32; 2],
    size: [f32; 2],
    uv: [f32; 4],
    color: [f32; 4],
}
fn quads(session: &SliceSession, fonts: &UiFonts) -> Vec<UiDraw> {
    session
        .host_ui()
        .draw(
            fonts,
            UiViewport::new([0.0, 0.0, 1280.0, 720.0], [640.0, 360.0]),
        )
        .iter()
        .map(|q| UiDraw {
            texture: q.texture.0,
            position: q.instance.position,
            size: q.instance.size,
            uv: q.instance.uv_rect,
            color: q.instance.color,
        })
        .collect()
}
#[test]
fn authored_screens_survive_restart_archive_and_editor_player_gameplay() {
    let root = tempfile::tempdir().unwrap();
    let authored = author(root.path());
    let reopened = project_files::load(root.path()).unwrap().project;
    assert_eq!(reopened.ui, authored.ui);
    let direct = kiln_player::loader::load(root.path(), None).unwrap();
    let output = tempfile::tempdir().unwrap();
    let archive = output.path().join("ui.kar");
    let files = [
        "kiln.toml",
        "scripts/main.ks",
        "scripts/ui.ks",
        "assets/fonts/KilnSampleSans.ttf",
        "assets/fonts/LICENSE.txt",
        "assets/audio/theme.wav",
        "assets/audio/hit.wav",
    ];
    kiln_assets::Archive::new(archive.clone())
        .pack(&files.map(|f| root.path().join(f)))
        .unwrap();
    let packed = kiln_player::loader::load(&archive, None).unwrap();
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let (mut editor, mut ew, ef) = setup(&direct, a.path());
    let (mut player, mut pw, pf) = setup(&packed, b.path());
    if std::env::var_os("KILN_UI_PROFILE").is_some() {
        let mut ui = kiln_ui::authored::UiRuntime::new(&authored.ui, "main").unwrap();
        let viewport = UiViewport::new([0.0, 0.0, 1280.0, 720.0], [640.0, 360.0]);
        let mut samples = Vec::new();
        let mut count = 0;
        for _ in 0..1000 {
            let start = std::time::Instant::now();
            ui.update(Default::default(), viewport.logical_size)
                .unwrap();
            count = std::hint::black_box(ui.draw(&ef, viewport).len());
            samples.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        samples.sort_by(f64::total_cmp);
        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        eprintln!(
            "UI_CPU_PROFILE quads={count} mean_ms={mean:.6} p95_ms={:.6}",
            samples[949]
        );
    }

    for keys in [
        vec![],
        vec![],
        vec!["GamepadDown"],
        vec![],
        vec!["GamepadAccept"],
        vec!["GamepadAccept"],
        vec![],
        vec!["Enter"],
        vec![],
        vec!["Escape"],
        vec![],
        vec!["ArrowUp"],
        vec![],
        vec!["Enter"],
        vec![],
        vec!["Enter"],
        vec![],
        vec!["Escape"],
        vec![],
        vec!["ArrowDown"],
        vec![],
        vec!["ArrowDown"],
        vec![],
        vec!["Enter"],
    ] {
        tick(&mut editor, &mut ew, &keys);
        tick(&mut player, &mut pw, &keys);
        assert_eq!(quads(&editor, &ef), quads(&player, &pf));
        assert_eq!(
            editor.host_ui().active_screen(),
            player.host_ui().active_screen()
        );
        assert_eq!(x(&ew), x(&pw));
    }
    assert_eq!(editor.host_ui().active_screen(), "game_over");
    assert_eq!(
        editor.entity_script_state("ui", "event"),
        Some(&Value::Str("gameover".into()))
    );
    assert_eq!(
        editor.entity_script_state("ui", "clicked"),
        Some(&Value::Bool(true))
    );
    assert_eq!(
        editor
            .host_mut()
            .call_host(57, &[Value::Str("root_event".into()), Value::I64(0)])
            .unwrap(),
        Value::Str("gameover".into())
    );
    assert_eq!(
        editor.entity_script_state("ui", "starts"),
        Some(&Value::I64(1))
    );
    let frozen = x(&ew);
    let elapsed = editor.state().elapsed;
    for _ in 0..20 {
        tick(&mut editor, &mut ew, &[]);
    }
    assert_eq!(x(&ew), frozen);
    assert_eq!(editor.state().elapsed, elapsed);
    assert!(frozen > 40.0); // Resumed gameplay actually advanced between menus.
    let (mut restart, _, _) = setup(&direct, a.path());
    assert_eq!(restart.host_ui().active_screen(), "title");
    assert_eq!(
        restart.host_mut().player_setting("menu_setting"),
        Some(Value::Str("saved".into()))
    );
    #[cfg(feature = "audio")]
    assert_eq!(
        restart.host_mut().audio_mut().unwrap().master_volume(),
        0.25
    );
    if let Some(destination) = std::env::var_os("KILN_TASK7_UI_SAMPLE_OUT") {
        let destination = Path::new(&destination);
        let destination = if destination.is_absolute() {
            destination.to_owned()
        } else {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join(destination)
        };
        for file in files {
            let target = destination.join(file);
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::copy(root.path().join(file), target).unwrap();
        }
    }
}

#[test]
fn ui_transactions_are_atomic_and_deleting_parents_cleans_focus_links() {
    let mut p = Project::default();
    let mut editor = UiLayoutEditorState::new();
    editor
        .edit(&mut p, |doc| {
            doc.screens.insert(
                "main".into(),
                menu(
                    "Main",
                    vec![
                        item("parent", "Parent", 0.0, UiAction::Event),
                        item("child", "Child", 40.0, UiAction::Event),
                    ],
                ),
            );
            doc.fonts.insert(
                "body".into(),
                UiFont {
                    path: "body.ttf".into(),
                    ..Default::default()
                },
            );
            doc.screens
                .get_mut("main")
                .unwrap()
                .widgets
                .last_mut()
                .unwrap()
                .parent = "parent".into();
            Ok(())
        })
        .unwrap();
    let before = p.ui.clone();
    assert!(editor
        .edit(&mut p, |doc| {
            doc.screens
                .get_mut("main")
                .unwrap()
                .widgets
                .iter_mut()
                .find(|w| w.id == "parent")
                .unwrap()
                .parent = "child".into();
            Ok(())
        })
        .is_err());
    assert_eq!(p.ui, before);
    editor
        .edit(&mut p, |doc| {
            kiln_editor::ui_layout_editor::delete_widget(
                doc.screens.get_mut("main").unwrap(),
                "parent",
            );
            Ok(())
        })
        .unwrap();
    assert!(!p.ui.screens["main"].widgets.iter().any(|w| w.id == "child"));
    assert!(p.ui.screens["main"].default_focus.is_empty());
    assert!(editor.undo(&mut p));
    assert_eq!(p.ui, before);
}

#[test]
fn opening_the_saved_ui_panel_without_input_does_not_dirty_the_project() {
    let mut p = Project::default();
    p.ui.screens.insert("main".into(), UiScreen::default());
    let before = p.ui.clone();
    let mut state = UiLayoutEditorState::new();
    let ctx = egui::Context::default();
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| state.render(ui, &mut p, None));
    });
    assert_eq!(p.ui, before);
}
#[test]
fn moving_a_font_keeps_its_stable_id_and_runtime_glyphs_after_reopen() {
    let root = tempfile::tempdir().unwrap();
    let mut project = author(root.path());
    let mut browser = kiln_editor::AssetBrowser::new();
    browser.scan(root.path());
    browser
        .rename_asset(
            "assets/fonts/KilnSampleSans.ttf",
            "assets/fonts/MovedSans.ttf",
        )
        .unwrap();
    assert_eq!(
        kiln_editor::editing::rewrite_asset_references(
            &mut project,
            "assets/fonts/KilnSampleSans.ttf",
            "assets/fonts/MovedSans.ttf"
        ),
        1
    );
    project_files::save(&project, &root.path().join("kiln.toml")).unwrap();
    let game = kiln_player::loader::load(root.path(), None).unwrap();
    assert_eq!(
        game.project.ui.fonts["body"].path,
        "assets/fonts/MovedSans.ttf"
    );
    let (fonts, errors) = UiFonts::load(
        &game.project.ui,
        |path| Ok(game.assets[path].clone()),
        |_, _, _| kiln_render::TextureId(3),
    );
    assert!(errors.is_empty());
    assert!(fonts.fonts["body"].glyphs.contains_key(&'A'));
}
