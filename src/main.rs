#![allow(unused)]

mod vulkan;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{self, Window, WindowId};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new();
    event_loop.run_app(&mut app)?;

    Ok(())
}

struct App {
    context: Option<vulkan::Context>,
    window: Option<Window>,
}

impl App {
    fn new() -> Self {
        Self {
            context: None,
            window: None,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        match (&self.window, &self.context) {
            (None, None) => {
                let window = event_loop
                    .create_window(winit::window::WindowAttributes::default())
                    .expect("Failed to create window");

                let context = vulkan::Context::new(&window);

                self.window = Some(window);
                self.context = Some(context);
            }
            (None, Some(_)) => {
                panic!(
                    "Vulkan context without window you should probably create both (treat it as both are None)"
                );
                self.window = Some(
                    event_loop
                        .create_window(winit::window::WindowAttributes::default())
                        .expect("Failed to create window"),
                )
            }
            (Some(window), None) => {
                self.context = Some(vulkan::Context::new(window));
            }

            (Some(_), Some(_)) => (),
        };

        println!("Resumed - Success");
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                println!("Close - requirested");
                event_loop.exit();
            }

            WindowEvent::RedrawRequested => {
                println!("Redraw - requirested");
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::Resized(_new_size) => {
                // Handle window resize - recreate swapchain
                println!("Window resized - need to recreate swapchain");
            }
            _ => {}
        }
    }
}
