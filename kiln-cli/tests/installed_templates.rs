//! The installed CLI produces editable/exportable scenes and preserves existing projects.
use kiln_core::Project;
use std::{fs, process::Command};
#[test]
fn template_initialization_creates_a_scene_and_refuses_nonempty_output() {
    let root = std::env::temp_dir().join(format!("kiln-template-{}", uuid::Uuid::new_v4()));
    fs::create_dir(&root).unwrap();
    for template in ["empty_2d", "platformer", "topdown"] {
        let output = root.join(template);
        let run = || {
            Command::new(env!("CARGO_BIN_EXE_kiln-cli"))
                .args([
                    "init",
                    "--name",
                    "Installed starter",
                    "--template",
                    template,
                    "--output",
                ])
                .arg(&output)
                .output()
                .unwrap()
        };
        let result = run();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let path = output.join("kiln.toml");
        let before = fs::read_to_string(&path).unwrap();
        let project = Project::from_toml(&before).unwrap();
        assert_eq!(project.name, "Installed starter");
        assert!(!project.scenes.is_empty());
        for scene in project.scenes.values() {
            if let Some(script) = &scene.script {
                assert!(output.join(script).is_file());
            }
        }
        assert!(!run().status.success());
        assert_eq!(before, fs::read_to_string(&path).unwrap());
    }
    fs::remove_dir_all(root).unwrap();
}
