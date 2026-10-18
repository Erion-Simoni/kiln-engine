use kiln_core::Vec2;
use kiln_runtime::{GameLoop, GameWorld, InputState};
use kiln_render::Renderer;
use winit::{
    dpi::PhysicalSize,
    event::{Event, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::Window,
};

#[derive(Clone, Copy)]
struct Position {
    x: f32,
    y: f32,
}

fn main() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let window = Window::new(&event_loop).expect("Failed to create window");

    let mut app = pollster::block_on(App::new(&window));

    event_loop
        .run(move |event, elwt| {
            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => elwt.exit(),
                Event::WindowEvent {
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    app.resize(size);
                }
                Event::WindowEvent {
                    event: WindowEvent::KeyboardInput { event, .. },
                    ..
                } => {
                    app.handle_key(event);
                }
                Event::AboutToWait => {
                    app.update();
                    app.render();
                }
                _ => {}
            }
        })
        .expect("Event loop error");
}

struct App {
    renderer: Renderer,
    game_loop: GameLoop,
    world: GameWorld,
    input: InputState,
    marker_pos: Vec2,
}

impl App {
    async fn new(window: &Window) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = unsafe {
            instance
                .create_surface(window)
                .expect("Failed to create surface")
        };

        let renderer = Renderer::new(&surface, size.width, size.height).await;
        let mut world = GameWorld::new();

        let marker_pos = Vec2::new(0.0, 0.0);
        world.spawn(Position {
            x: marker_pos.x,
            y: marker_pos.y,
        });

        Self {
            renderer,
            game_loop: GameLoop::new(),
            world,
            input: InputState::new(),
            marker_pos,
        }
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        // Resize will be implemented with proper surface management
        // For now, placeholder
    }

    fn handle_key(&mut self, event: KeyEvent) {
        use winit::keyboard::PhysicalKey;

        match event.physical_key {
            PhysicalKey::Named(NamedKey::ArrowUp) => {
                self.marker_pos.y += 10.0;
            }
            PhysicalKey::Named(NamedKey::ArrowDown) => {
                self.marker_pos.y -= 10.0;
            }
            PhysicalKey::Named(NamedKey::ArrowLeft) => {
                self.marker_pos.x -= 10.0;
            }
            PhysicalKey::Named(NamedKey::ArrowRight) => {
                self.marker_pos.x += 10.0;
            }
            _ => {}
        }
    }

    fn update(&mut self) {
        self.game_loop.tick(|_dt| {
            for (_, mut pos) in self.world.world().query::<&mut Position>().iter() {
                pos.x = self.marker_pos.x;
                pos.y = self.marker_pos.y;
            }
        });
    }

    fn render(&mut self) {
        // Render placeholder - will be replaced with proper rendering in M2
    }
}
