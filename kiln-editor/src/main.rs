//! Kiln Editor — native desktop editor app entry point.

use kiln_editor::EditorApp;
use winit::event_loop::EventLoop;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let event_loop = EventLoop::new()?;
    let mut app = EditorApp::new(&event_loop)?;
    event_loop.run_app(&mut app)?;
    Ok(())
}
