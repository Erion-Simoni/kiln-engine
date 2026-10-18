//! Main editor application.

use std::sync::Arc;
use winit::window::Window;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EditorError {
    #[error("Window error: {0}")]
    Window(String),
}

pub struct EditorApp {
    window: Arc<Window>,
}

impl EditorApp {
    pub fn new(window: Arc<Window>) -> Result<Self, EditorError> {
        Ok(Self {
            window,
        })
    }

    pub fn run(&mut self) -> Result<(), EditorError> {
        // Main render loop would go here
        Ok(())
    }
}
