//! Stand alone preview of the splash.
//!
//! Opens a plain `winit` window and renders the splash into it forever, so the
//! logo and the spinning ring can be eyeballed without wiring up a whole overlay
//! (where the splash only shows for a beat before the UI takes over). The logo is
//! generated in code, so the example needs no assets.
//!
//! Run with `cargo run -p bluebottle-splash --example preview`.

use std::sync::Arc;

use bluebottle_splash::{Splash, SplashRenderer};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

/// The brand background the real app uses.
const BACKGROUND: iced::Color = iced::color!(0x101828);

fn main() {
    let event_loop = EventLoop::new().expect("create event loop");
    // Drive a continuous redraw so the spinner keeps animating; the renderer
    // itself paces on vblank.
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App {
        renderer: None,
        window: None,
    };
    event_loop.run_app(&mut app).expect("run event loop");
}

// Renderer before window so it drops first: it holds a surface that references
// the window's handles.
struct App {
    renderer: Option<SplashRenderer>,
    window: Option<Arc<Window>>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title("bluebottle-splash preview")
            .with_inner_size(winit::dpi::LogicalSize::new(960.0, 600.0));
        let window =
            Arc::new(event_loop.create_window(attributes).expect("create window"));

        let size = window.inner_size();
        let splash = Splash::new(placeholder_logo(), BACKGROUND);
        let renderer = SplashRenderer::new(
            window.as_ref(),
            (size.width.max(1), size.height.max(1)),
            &splash,
        )
        .expect("build splash renderer");

        window.request_redraw();
        self.renderer = Some(renderer);
        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(size.width, size.height);
                }
            },
            WindowEvent::RedrawRequested => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.render();
                }
                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            },
            _ => {},
        }
    }
}

/// Draw a simple filled disc so the example needs no logo asset.
fn placeholder_logo() -> image::DynamicImage {
    const SIZE: u32 = 256;
    let centre = SIZE as f32 / 2.0;
    let radius = SIZE as f32 * 0.42;
    let tint = [0x3b, 0xa3, 0xc7];

    let mut pixels = image::RgbaImage::new(SIZE, SIZE);
    for (x, y, pixel) in pixels.enumerate_pixels_mut() {
        let dx = x as f32 + 0.5 - centre;
        let dy = y as f32 + 0.5 - centre;
        let distance = (dx * dx + dy * dy).sqrt();
        // Soft one pixel edge on the disc.
        let alpha =
            (1.0 - smoothstep(radius - 1.0, radius + 1.0, distance)).clamp(0.0, 1.0);
        *pixel = image::Rgba([tint[0], tint[1], tint[2], (alpha * 255.0) as u8]);
    }
    image::DynamicImage::ImageRgba8(pixels)
}

/// The usual smooth Hermite step between two edges.
fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
