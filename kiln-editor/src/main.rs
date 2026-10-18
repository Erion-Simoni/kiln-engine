//! Kiln Editor — native desktop editor app entry point.

use kiln_editor::EditorApp;
use winit::event_loop::EventLoop;

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Warn)
        .init();

    let event_loop = EventLoop::new().expect("event loop");
    let mut app = EditorApp::new(&event_loop).expect("failed to create editor");
    let _ = event_loop.run_app(&mut app);
}
