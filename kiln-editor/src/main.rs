//! Kiln Editor — native desktop editor app entry point.

use kiln_editor::EditorApp;
use winit::event_loop::EventLoop;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() == 1 && args[0] == "--tool-info" {
        let mut features = Vec::new();
        if cfg!(feature = "audio") {
            features.push("audio".into());
        }
        if cfg!(feature = "fonts") {
            features.push("fonts".into());
        }
        if cfg!(feature = "gamepad") {
            features.push("gamepad".into());
        }
        let info = kiln_core::distribution::ToolInfo::current(
            "kiln-editor",
            features,
            cfg!(debug_assertions),
        );
        println!("{}", serde_json::to_string(&info)?);
        return Ok(());
    }
    if !args.is_empty() {
        return Err(
            "Use kiln-editor without arguments, or --tool-info for installation metadata".into(),
        );
    }
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let event_loop = EventLoop::new()?;
    let mut app = EditorApp::new(&event_loop)?;
    event_loop.run_app(&mut app)?;
    Ok(())
}
