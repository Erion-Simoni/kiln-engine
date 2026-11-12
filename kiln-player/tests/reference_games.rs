//! Replays the projects authored through the native editor, from source and KAR.
use kiln_core::Transform;
use kiln_ir::Value;
use kiln_player::loader::{self, LoadedGame};
use kiln_render::TextureId;
use kiln_runtime::{GameWorld, SliceSession, SourceId, Sprite};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

fn game_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../games")
        .join(name)
}
fn files(root: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.file_name().unwrap().to_string_lossy().starts_with('.') {
            continue;
        }
        if path.is_dir() {
            files(&path, out);
        } else {
            out.push(path);
        }
    }
}
fn variants(name: &str, scene: &str, check: impl Fn(LoadedGame)) {
    let root = game_dir(name);
    check(loader::load(&root, Some(scene)).unwrap());
    let temp = tempfile::tempdir().unwrap();
    let archive = temp.path().join("game.kar");
    let mut paths = vec![];
    files(&root, &mut paths);
    kiln_assets::Archive::new(archive.clone())
        .pack(&paths)
        .unwrap();
    check(loader::load(&archive, Some(scene)).unwrap());
}
struct Game {
    loaded: LoadedGame,
    world: GameWorld,
    session: SliceSession,
    bindings: HashMap<String, (TextureId, [u32; 2])>,
    _save: tempfile::TempDir,
}
impl Game {
    fn new(loaded: LoadedGame) -> Self {
        let bindings: HashMap<_, _> = loaded
            .assets
            .iter()
            .filter(|(name, _)| name.ends_with(".png"))
            .enumerate()
            .map(|(index, (name, bytes))| {
                let image = kiln_assets::decode_png(bytes).unwrap();
                (
                    name.clone(),
                    (TextureId(index as u32 + 1), [image.width, image.height]),
                )
            })
            .collect();
        let mut world = GameWorld::new();
        kiln_runtime::spawn_scene(
            &mut world,
            &loaded.project,
            &loaded.scene_id,
            &mut |_, c, path| {
                let mut sprite = Sprite::new(c[0], c[1], c[2], c[3]);
                if let Some((texture, _)) = path.and_then(|path| bindings.get(path)) {
                    sprite.texture = *texture;
                }
                sprite
            },
        );
        let mut session =
            SliceSession::with_scene_sources(&loaded.project, &loaded.scene_id, |path| {
                String::from_utf8(loaded.assets[path].clone()).map_err(|e| e.to_string())
            })
            .unwrap();
        let save = tempfile::tempdir().unwrap();
        session.host_mut().set_save_dir(save.path());
        let mut game = Self {
            loaded,
            world,
            session,
            bindings,
            _save: save,
        };
        game.tick((0.0, 0.0));
        game
    }
    fn tick(&mut self, axis: (f64, f64)) {
        self.session
            .step(&mut self.world, axis, 1.0 / 60.0)
            .unwrap();
        self.session
            .service_saves(
                &mut self.world,
                &self.loaded.project,
                |path| {
                    String::from_utf8(self.loaded.assets[path].clone()).map_err(|e| e.to_string())
                },
                |path| self.bindings.get(path).copied(),
            )
            .unwrap();
    }
    fn click(&mut self, x: f32, y: f32) {
        for down in [true, false] {
            self.session
                .host_mut()
                .queue_ui_pointer([0.0, 0.0, 640.0, 360.0], [x, y], down);
            self.session
                .host_mut()
                .set_ui_pointer([0.0, 0.0, 640.0, 360.0], Some([x, y]), down);
            self.tick((0.0, 0.0));
        }
        self.tick((0.0, 0.0));
    }
    fn value(&mut self, key: &str) -> Value {
        self.session.host_mut().game_value(key).unwrap().clone()
    }
    fn role(&self, role: &str) -> bool {
        self.world
            .world_ref()
            .query::<&kiln_runtime::Role>()
            .iter()
            .any(|item| item.0 == role)
    }
    fn position(&self, id: &str) -> [f32; 2] {
        self.world
            .world_ref()
            .query::<(&SourceId, &Transform)>()
            .iter()
            .find(|(source, _)| source.0 == id)
            .unwrap()
            .1
            .position
            .to_array()
    }
}

#[test]
fn switchyard_three_circuits_are_solvable_and_progress_survives_a_real_save() {
    variants("switchyard", "main", |loaded| {
        let mut game = Game::new(loaded);
        game.click(320.0, 180.0);
        assert_eq!(game.session.host_ui().active_screen(), "hud");
        // A, B, C; then B, C; then A, C. No winning state is injected.
        for (stage, solution) in [(1, vec![0, 1, 2]), (2, vec![1, 2]), (3, vec![0, 2])] {
            for switch in solution.iter().copied() {
                while game.value("selected") != Value::I64(switch) {
                    game.tick((1.0, 0.0));
                    game.tick((0.0, 0.0));
                }
                game.click(320.0, 276.0);
            }
            assert_eq!(game.value("stage"), Value::I64(stage));
            assert_eq!(game.value("outcome"), Value::Str("won".into()));
            assert_eq!(game.value("moves"), Value::I64(solution.len() as i64));
            assert_eq!(game.session.host_ui().active_screen(), "game_over");
            if stage < 3 {
                game.click(320.0, 180.0);
            }
        }
        let before = game.session.snapshot(&game.world).unwrap();
        game.session.save_game(&game.world, 0).unwrap();
        game.session
            .load_game(
                &mut game.world,
                0,
                &game.loaded.project,
                |path| {
                    String::from_utf8(game.loaded.assets[path].clone()).map_err(|e| e.to_string())
                },
                |path| game.bindings.get(path).copied(),
            )
            .unwrap();
        assert_eq!(game.value("stage"), Value::I64(3));
        assert_eq!(game.value("moves"), Value::I64(2));
        let after = game.session.snapshot(&game.world).unwrap();
        assert_eq!(before.game_data, after.game_data);
        assert_eq!(
            serde_json::to_value(before.scene).unwrap(),
            serde_json::to_value(after.scene).unwrap()
        );
    });
}

#[test]
fn emberwatch_projectiles_defeat_all_enemies_and_the_relay_trigger_wins() {
    variants("emberwatch", "main", |loaded| {
        let mut game = Game::new(loaded);
        game.click(320.0, 180.0);
        for _ in 0..10 {
            if game.value("cleared") == Value::I64(3) {
                break;
            }
            game.click(320.0, 276.0);
            for _ in 0..66 {
                game.tick((0.0, 0.0));
            }
        }
        assert_eq!(game.value("cleared"), Value::I64(3));
        assert!(!game.role("scout") && !game.role("guard") && !game.role("brute"));
        for _ in 0..180 {
            game.tick((1.0, 0.0));
        }
        assert_eq!(game.value("outcome"), Value::Str("won".into()));
        assert_eq!(game.session.host_ui().active_screen(), "game_over");
    });
}

#[test]
fn emberwatch_damage_loses_and_restart_restores_a_playable_round() {
    variants("emberwatch", "main", |loaded| {
        let mut game = Game::new(loaded);
        game.click(320.0, 180.0);
        for _ in 0..1800 {
            game.tick((0.0, 0.0));
            if game.value("outcome") == Value::Str("lost".into()) {
                break;
            }
        }
        assert_eq!(game.value("outcome"), Value::Str("lost".into()));
        game.click(320.0, 180.0);
        assert_eq!(game.value("health"), Value::I64(5));
        assert_eq!(game.value("outcome"), Value::Str("playing".into()));
        assert!(game.role("scout") && game.role("guard") && game.role("brute"));
    });
}

#[test]
fn skybridge_grounds_jumps_animates_and_crosses_both_physical_level_gates() {
    for scene in ["main", "scene_1"] {
        variants("skybridge", scene, |loaded| {
            let mut game = Game::new(loaded);
            for _ in 0..60 {
                game.tick((0.0, 0.0));
            }
            assert!((game.position("hero")[1] - 32.0).abs() < 2.0);
            let mut frames = std::collections::BTreeSet::new();
            let mut peak = 0.0f32;
            let mut transition = None;
            for frame in 0..900 {
                game.tick((1.0, if frame % 60 == 0 { 1.0 } else { 0.0 }));
                peak = peak.max(game.position("hero")[1]);
                let uv = game
                    .world
                    .world_ref()
                    .query::<(&SourceId, &Sprite)>()
                    .iter()
                    .find(|(id, _)| id.0 == "hero")
                    .unwrap()
                    .1
                    .uv_rect;
                frames.insert(uv.map(f32::to_bits));
                transition = game.session.take_scene_request();
                if transition.is_some() || game.value("won") == Value::Bool(true) {
                    break;
                }
            }
            assert!(peak > 65.0, "jump did not leave the floor");
            assert!(frames.len() > 1, "run animation did not advance");
            if scene == "main" {
                assert_eq!(transition.as_deref(), Some("scene_1"));
            } else {
                assert_eq!(game.value("won"), Value::Bool(true));
            }
        });
    }
}
