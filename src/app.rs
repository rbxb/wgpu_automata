use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize, event::*,
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId}
};
use std::time::Instant;
use crate::render::RenderState;

#[derive(Default)]
pub struct App<'a> {
    window: Option<Arc<Window>>,
    state: Option<RenderState<'a>>,
    last_now: Option<Instant>,
    sum_frame_time: u128,
    frame_count: u128,
}

impl ApplicationHandler for App<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        println!("App resumed");
        if self.window.is_none() {
            println!("Creating window and renderer");

            let window = Arc::new(event_loop.create_window(Window::default_attributes()
                .with_title("wgpu automata")
                .with_inner_size(PhysicalSize {
                    width: 1024u32,
                    height: 1024u32,
                })
            ).unwrap());
            self.window = Some(window.clone());

            let mut state = pollster::block_on(RenderState::new(window.clone()));
            state.create_pipelines();
            state.randomize();
            self.state = Some(state);
        }
    }
    
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if id != self.window.as_ref().unwrap().id() {
            return;
        }

        let window = self.window.as_ref().unwrap();
        let state = self.state.as_mut().unwrap();

        match event {
            WindowEvent::CloseRequested => {
                println!("Close requested");
                event_loop.exit()
            },
            WindowEvent::Resized(physical_size) => {
                println!("Resize requested");
                state.resize(physical_size);
            },
            WindowEvent::RedrawRequested => {
                state.draw();
                if self.last_now.is_some() {
                    self.sum_frame_time += self.last_now.unwrap().elapsed().as_micros();
                    self.frame_count += 1;
                    if self.frame_count == 100 {
                        println!("{} fps", 1e+6f32 / (self.sum_frame_time / self.frame_count) as f32);
                        self.sum_frame_time = 0;
                        self.frame_count = 0;
                    }
                }
                self.last_now = Some(Instant::now());
                window.request_redraw();
            },
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        state: ElementState::Pressed,
                        repeat: false,
                        ..
                    },
                ..
            } => {
                println!("Close requested");
                event_loop.exit();
            },
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::Space),
                        state: ElementState::Pressed,
                        repeat: false,
                        ..
                    },
                ..
            } => {
                println!("Space key pressed!");
                state.randomize();
            },
            _ => {},
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        println!("App suspended");
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        println!("App exiting");
        // shut down
    }
}
