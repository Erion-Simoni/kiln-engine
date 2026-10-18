use kiln_core::Vec2;
use kiln_runtime::{GameLoop, GameWorld, InputState};
use kiln_render::Renderer;
use winit::event_loop::EventLoop;

#[derive(Clone, Copy)]
struct Position {
    x: f32,
    y: f32,
}

fn main() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let mut app = App::default();

    let _ = event_loop.run_app(&mut app);
}

struct App {
    renderer: Option<Renderer>,
    game_loop: GameLoop,
    world: GameWorld,
    input: InputState,
    marker_pos: Vec2,
}

impl Default for App {
    fn default() -> Self {
        let mut world = GameWorld::new();
        let marker_pos = Vec2::new(0.0, 0.0);
        world.spawn((Position {
            x: marker_pos.x,
            y: marker_pos.y,
        },));

        Self {
            renderer: None,
            game_loop: GameLoop::new(),
            world,
            input: InputState::new(),
            marker_pos,
        }
    }
}

impl App {
    async fn init_renderer(&mut self, window: &winit::window::Window) {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance
            .create_surface(window)
            .expect("Failed to create surface");

        let renderer = Renderer::new(&surface, size.width, size.height).await;
        self.renderer = Some(renderer);
    }

    fn update(&mut self) {
        self.game_loop.tick(|_dt| {
            for pos in self.world.world().query_mut::<&mut Position>() {
                pos.x = self.marker_pos.x;
                pos.y = self.marker_pos.y;
            }
        });
    }

    fn render(&mut self) {
        // Render placeholder - will be replaced with proper rendering in M2
    }
}

impl winit::application::ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = event_loop
            .create_window(winit::window::WindowAttributes::default())
            .expect("Failed to create window");
        
        pollster::block_on(self.init_renderer(&window));
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            winit::event::WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        self.update();
        self.render();
    }
}
