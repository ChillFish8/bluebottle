//! End-to-end demonstration of `bluebottle-window`.
//!
//! `create_overlay` returns a handle to the main (parent) surface, which this
//! example renders into itself with wgpu — an animated colour fill standing in
//! for whatever the caller would draw there (e.g. video via libmpv's render
//! API). The Iced overlay is composited on top with a transparent background,
//! so the animated fill shows through everywhere the UI does not paint.
//!
//! It also exercises input (click "increment"), an async `Task` (each click
//! kicks off a 500ms "ping"), and a `Subscription` (a 1s timer).

use std::time::{Duration, Instant};

use iced::widget::{button, column, text, text_input};

/// How long the demo runs before closing itself.
const RUN_FOR: Duration = Duration::from_secs(30);

/// Clamp to wgpu's default `max_texture_dimension_2d` (8192).
fn clamp_dimension(value: u32) -> u32 {
    value.clamp(1, 8192)
}

/// Identifies the text input so a [`Message::FocusInput`] can target it.
const INPUT_ID: &str = "demo-input";

#[derive(Debug, Default)]
struct App {
    count: u32,
    ticks: u32,
    input: String,
}

#[derive(Debug, Clone)]
enum Message {
    Increment,
    Pinged,
    Tick,
    InputChanged(String),
    FocusInput,
}

fn update(state: &mut App, message: Message) -> iced::Task<Message> {
    match message {
        Message::Increment => {
            state.count += 1;
            return iced::Task::perform(
                async { tokio::time::sleep(Duration::from_millis(500)).await },
                |()| Message::Pinged,
            );
        },
        Message::Pinged => tracing::info!("async ping completed"),
        Message::Tick => state.ticks += 1,
        Message::InputChanged(value) => state.input = value,
        // Exercises a widget operation: focus is driven by a Task, not input.
        Message::FocusInput => return iced::widget::operation::focus(INPUT_ID),
    }
    iced::Task::none()
}

fn view(state: &App) -> iced::Element<'_, Message> {
    column![
        text(format!("count: {}", state.count)).size(32),
        text(format!("ticks: {}", state.ticks)).size(20),
        button("increment").on_press(Message::Increment),
        text_input("type here…", &state.input)
            .id(INPUT_ID)
            .on_input(Message::InputChanged),
        button("focus input").on_press(Message::FocusInput),
    ]
    .spacing(16)
    .padding(24)
    .into()
}

fn subscription(_state: &App) -> iced::Subscription<Message> {
    iced::time::every(Duration::from_secs(1)).map(|_| Message::Tick)
}

fn main() {
    tracing_subscriber::fmt::init();

    let window = bluebottle_window::create_overlay(|| {
        iced::application(App::default, update, view).subscription(subscription)
    })
    .expect("create overlay window");

    // Render the main surface for the lifetime of `render_main_surface`. All
    // wgpu resources are dropped when it returns, before we close the window:
    // they reference the `wl_display`, which `join` tears down.
    render_main_surface(&window);

    window.request_close();
    window.join().expect("overlay loop exited cleanly");
}

/// Drive the caller-owned main surface with an animated wgpu fill.
fn render_main_surface(window: &bluebottle_window::Window) {
    // The caller owns the main surface: drive it with wgpu here. Restrict to
    // Vulkan to match the overlay (the GLES backend deadlocks on the shared
    // Wayland display).
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..Default::default()
    });
    let surface = unsafe {
        instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
            raw_display_handle: window.raw_display_handle(),
            raw_window_handle: window.raw_window_handle(),
        })
    }
    .expect("create main surface");

    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }))
        .expect("request adapter");

    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("request device");

    let (width, height) = window.physical_size();
    let mut config = surface
        .get_default_config(&adapter, clamp_dimension(width), clamp_dimension(height))
        .expect("surface configuration");
    surface.configure(&device, &config);

    let start = Instant::now();
    while window.is_open() && start.elapsed() < RUN_FOR {
        // Follow the window if the compositor resizes or rescales it.
        let (width, height) = window.physical_size();
        let (width, height) = (clamp_dimension(width), clamp_dimension(height));
        if (width, height) != (config.width, config.height) {
            config.width = width;
            config.height = height;
            surface.configure(&device, &config);
        }

        let time = start.elapsed().as_secs_f64();
        let color = wgpu::Color {
            r: 0.5 + 0.5 * time.sin(),
            g: 0.5 + 0.5 * (time * 0.7).sin(),
            b: 0.5 + 0.5 * (time * 1.3).sin(),
            a: 1.0,
        };

        match surface.get_current_texture() {
            Ok(frame) => {
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let mut encoder = device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("main fill"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(color),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                queue.submit([encoder.finish()]);
                frame.present();
            },
            Err(_) => surface.configure(&device, &config),
        }

        std::thread::sleep(Duration::from_millis(33));
    }
}
