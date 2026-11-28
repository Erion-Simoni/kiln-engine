use kiln_core::{Component, Entity, Project, Scene, Transform};
use kiln_ir::{HostContext, Value, VmError};
use kiln_runtime::{GameWorld, SliceSession, SourceId};
fn fixture() -> (SliceSession, GameWorld) {
    let mut project = Project::default();
    project.scenes.insert(
        "main".into(),
        Scene {
            id: "main".into(),
            name: "Main".into(),
            script: Some("main.ks".into()),
            nested_scenes: vec![],
            entities: vec![Entity {
                id: "actor".into(),
                name: "Actor".into(),
                components: vec![Component::Script {
                    path: "actor.ks".into(),
                    properties: Default::default(),
                }],
            }],
        },
    );
    let session=SliceSession::with_scene_sources(&project,"main",|_|Ok("state_set(\"valid\",mouse_world_valid());\nstate_set(\"x\",get_mouse_world_x());\nstate_set(\"y\",get_mouse_world_y());".into())).unwrap();
    let mut world = GameWorld::new();
    world.spawn((SourceId::new("actor"), Transform::default()));
    (session, world)
}
#[test]
fn invalid_world_pointers_clear_coordinates_and_attached_callback_input() {
    let (mut s, mut world) = fixture();
    for (point, valid, xy) in [
        (Some([12.0, 34.0]), true, [12.0, 34.0]),
        (None, false, [0.0; 2]),
        (Some([f64::NAN, 0.0]), false, [0.0; 2]),
    ] {
        s.host_mut().set_world_pointer(point);
        s.step(&mut world, (0.0, 0.0), 0.016).unwrap();
        assert_eq!(s.root_script_state("valid"), Some(&Value::Bool(valid)));
        assert_eq!(
            s.entity_script_state("actor", "valid"),
            Some(&Value::Bool(valid))
        );
        assert_eq!(
            s.entity_script_state("actor", "x"),
            Some(&Value::F64(xy[0]))
        );
        assert_eq!(
            s.entity_script_state("actor", "y"),
            Some(&Value::F64(xy[1]))
        );
    }
    assert!(s.host_mut().call_host(128, &[Value::I64(1)]).is_err());
}
#[test]
fn debugger_resume_keeps_pointer_validity_and_coordinates_from_the_frozen_tick() {
    let (mut s, mut world) = fixture();
    s.add_source_breakpoint("main.ks", 2).unwrap();
    for point in [Some([12.0, 34.0]), None] {
        s.host_mut().set_world_pointer(point);
        assert!(matches!(
            s.step(&mut world, (0.0, 0.0), 0.016),
            Err(VmError::DebugSuspended)
        ));
        s.host_mut().set_world_pointer(if point.is_some() {
            None
        } else {
            Some([999.0, 999.0])
        });
        s.debug_continue();
        s.step(&mut world, (0.0, 0.0), 0.016).unwrap();
        assert_eq!(
            s.entity_script_state("actor", "valid"),
            Some(&Value::Bool(point.is_some()))
        );
        assert_eq!(
            s.root_script_state("x"),
            Some(&Value::F64(point.unwrap_or([0.0; 2])[0]))
        );
    }
}
