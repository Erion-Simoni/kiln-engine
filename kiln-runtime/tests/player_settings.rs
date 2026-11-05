//! Player settings are independent of sample score/pickups and native devices.
use kiln_core::{Component, Entity, Project, Scene, Transform};
use kiln_ir::{HostContext, Value};
use kiln_runtime::{GameWorld, SliceHost, SliceSession, SourceId};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "kiln-setting-{}-{}",
            std::process::id(),
            kiln_core::next_id()
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn script_settings_survive_restarting_with_typed_values() {
    let dir = Directory::new();
    let mut session = SliceSession::new(
        r#"
setting_set("difficulty", 9007199254740993);
setting_set("language", "sq");
setting_set("inverted", 1 == 1);
setting_set("bindings", array_push(array_new(), "Jump"));
"#,
    )
    .unwrap();
    session.host_mut().set_save_dir(&dir.0);
    session
        .step(&mut GameWorld::new(), (0.0, 0.0), 0.0)
        .unwrap();
    let mut reopened = SliceHost::default();
    reopened.set_save_dir(&dir.0);
    assert_eq!(
        reopened.player_setting("difficulty"),
        Some(Value::I64(9_007_199_254_740_993))
    );
    assert_eq!(
        reopened.player_setting("language"),
        Some(Value::Str("sq".into()))
    );
    assert_eq!(reopened.player_setting("inverted"), Some(Value::Bool(true)));
    assert_eq!(
        reopened.player_setting("bindings"),
        Some(Value::Array(vec![Value::Str("Jump".into())]))
    );
    assert_eq!(
        reopened
            .call_host(78, &[Value::Str("absent".into()), Value::I64(42)])
            .unwrap(),
        Value::I64(42)
    );
    assert!(reopened
        .set_player_setting("bad-number", Value::F64(f64::NAN))
        .is_err());
}

#[test]
fn corrupt_future_or_unwritable_preferences_report_errors_without_replacing_data() {
    let dir = Directory::new();
    for bytes in ["{broken", "{\"version\":999}"] {
        std::fs::write(dir.0.join("preferences.json"), bytes).unwrap();
        let mut host = SliceHost::default();
        host.set_save_dir(&dir.0);
        assert!(host.preferences_error().is_some());
        assert!(host
            .set_player_setting("language", Value::Str("en".into()))
            .is_err());
        assert_eq!(
            std::fs::read_to_string(dir.0.join("preferences.json")).unwrap(),
            bytes
        );
    }
    let mut host = SliceHost::default();
    assert!(host
        .set_player_setting("language", Value::Str("en".into()))
        .is_err());
    std::fs::remove_file(dir.0.join("preferences.json")).unwrap();
    host.set_save_dir(&dir.0);
    // Turn the configured directory into a regular file after a successful load.
    std::fs::remove_dir(&dir.0).unwrap();
    std::fs::write(&dir.0, "keep").unwrap();
    assert!(host
        .set_player_setting("language", Value::Str("en".into()))
        .is_err());
    assert_eq!(host.player_setting("language"), None);
    assert_eq!(std::fs::read_to_string(&dir.0).unwrap(), "keep");
    std::fs::remove_file(&dir.0).unwrap();
}

#[test]
fn entity_callbacks_share_settings_and_platform_audio_even_after_vm_failure() {
    let dir = Directory::new();
    let mut project = Project::default();
    project.scenes.insert(
        "main".into(),
        Scene {
            id: "main".into(),
            name: "Main".into(),
            script: None,
            nested_scenes: vec![],
            entities: vec![Entity {
                id: "speaker".into(),
                name: "Speaker".into(),
                components: vec![Component::Script {
                    path: "speaker.ks".into(),
                    properties: HashMap::new(),
                }],
            }],
        },
    );
    let mut session = SliceSession::with_scene_sources(&project, "main", |_| {
        Ok(r#"
if script_phase() == "start" { play_sound("hit"); setting_set("language", "sq"); }
if script_phase() == "update" { array_get(array_new(), 5); }
"#
        .into())
    })
    .unwrap();
    let received = Arc::new(Mutex::new(Vec::new()));
    let capture = received.clone();
    session.host_mut().set_audio_player(Box::new(move |name| {
        capture.lock().unwrap().push(name.to_owned())
    }));
    session.host_mut().set_save_dir(&dir.0);
    let mut world = GameWorld::new();
    world.spawn((SourceId::new("speaker"), Transform::default()));
    assert!(session.step(&mut world, (0.0, 0.0), 0.0).is_err());
    assert_eq!(
        session.host_mut().player_setting("language"),
        Some(Value::Str("sq".into()))
    );
    session
        .host_mut()
        .call_host(12, &[Value::Str("after-error".into())])
        .unwrap();
    assert_eq!(*received.lock().unwrap(), vec!["hit", "after-error"]);
    session
        .host_mut()
        .set_player_setting("language", Value::Str("en".into()))
        .unwrap();
    let mut reopened = SliceHost::default();
    reopened.set_save_dir(&dir.0);
    assert_eq!(
        reopened.player_setting("language"),
        Some(Value::Str("en".into()))
    );
}
