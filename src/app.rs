use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::ActiveEventLoop,
    window::{Window, WindowId}
};

use crate::render::RenderState;

#[derive(Default)]
pub struct App<'a> {
    window: Option<Arc<Window>>,
    state: Option<RenderState<'a>>,
}

impl ApplicationHandler for App<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        println!("App resumed");
        if self.window.is_none() {
            println!("Creating window and renderer");

            let window = Arc::new(event_loop.create_window(Window::default_attributes()).unwrap());
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

        match event {
            WindowEvent::CloseRequested => {
                println!("Close requested");
                event_loop.exit()
            },
            WindowEvent::Resized(physical_size) => {
                println!("Resize requested");
                self.state.as_mut().unwrap().resize(physical_size);
            },
            WindowEvent::RedrawRequested => {
                let state = self.state.as_mut().unwrap();
                state.transition();
                state.draw();
                self.window.as_ref().unwrap().request_redraw();
            },
            _ => {},
        }
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        println!("App suspended");
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        println!("App exiting");
        // shut down
    }
}
