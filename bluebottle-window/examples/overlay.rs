//! End-to-end demonstration of `bluebottle-window`.
//!
//! `create_overlay` lays an Iced overlay over a library-owned main surface (an
//! opaque black backdrop). The overlay clears transparently, so the backdrop
//! shows through everywhere the UI does not paint.
//!
//! It exercises input (click "increment"), an async `Task` (each click kicks off
//! a 500ms "ping"), a `Subscription` (a 1s timer), and the window controls the
//! overlay drives as the window's chrome.

use std::time::{Duration, Instant};

use iced::widget::{button, column, mouse_area, text, text_input};

/// How long the demo runs before closing itself.
const RUN_FOR: Duration = Duration::from_secs(30);

/// Identifies the text input so a [`Message::FocusInput`] can target it.
const INPUT_ID: &str = "demo-input";

#[derive(Debug, Default)]
struct App {
    count: u32,
    ticks: u32,
    input: String,
    window_id: Option<iced::window::Id>,
}

#[derive(Debug, Clone)]
enum Message {
    Increment,
    Pinged,
    Tick,
    InputChanged(String),
    FocusInput,
    Window(iced::window::Id, String),
    Minimize,
    ToggleMaximize,
    StartDrag,
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
        Message::Window(id, label) => {
            state.window_id = Some(id);
            tracing::info!("window event: {label}");
        },
        // Window controls: the overlay is the chrome, so these drive the
        // real toplevel.
        Message::Minimize => {
            if let Some(id) = state.window_id {
                return iced::window::minimize(id, true);
            }
        },
        Message::ToggleMaximize => {
            if let Some(id) = state.window_id {
                return iced::window::toggle_maximize(id);
            }
        },
        Message::StartDrag => {
            if let Some(id) = state.window_id {
                return iced::window::drag(id);
            }
        },
    }

    iced::Task::none()
}

/// Maps the window lifecycle events into loggable [`Message`]s.
fn window_event(
    event: iced::Event,
    _status: iced::event::Status,
    id: iced::window::Id,
) -> Option<Message> {
    use iced::window::Event;

    let label = match event {
        iced::Event::Window(Event::Opened { .. }) => "opened".to_owned(),
        iced::Event::Window(Event::Focused) => "focused".to_owned(),
        iced::Event::Window(Event::Unfocused) => "unfocused".to_owned(),
        iced::Event::Window(Event::Resized(size)) => {
            format!("resized to {}x{}", size.width, size.height)
        },
        iced::Event::Window(Event::Rescaled(scale)) => format!("rescaled to {scale}"),
        iced::Event::Window(Event::CloseRequested) => "close requested".to_owned(),
        _ => return None,
    };

    Some(Message::Window(id, label))
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
        button("minimize").on_press(Message::Minimize),
        button("toggle maximize").on_press(Message::ToggleMaximize),
        // `on_press` fires on button-down, so the move grab starts while the
        // pointer button is still held — what the compositor needs.
        mouse_area(text("— drag here to move —")).on_press(Message::StartDrag),
    ]
    .spacing(16)
    .padding(24)
    .into()
}

fn subscription(_state: &App) -> iced::Subscription<Message> {
    iced::Subscription::batch([
        iced::time::every(Duration::from_secs(1)).map(|_| Message::Tick),
        iced::event::listen_with(window_event),
    ])
}

fn main() {
    tracing_subscriber::fmt::init();

    let window = bluebottle_window::create_overlay(|| {
        iced::application(App::default, update, view)
            .title("Bluebottle Overlay Demo")
            .subscription(subscription)
    })
    .expect("create overlay window");

    // The library owns and paints the main surface; the overlay UI runs on its
    // own render thread. Keep the demo open until it closes itself or the window
    // is closed.
    let start = Instant::now();
    while window.is_open() && start.elapsed() < RUN_FOR {
        std::thread::sleep(Duration::from_millis(100));
    }

    window.request_close();
    window.join().expect("overlay loop exited cleanly");
}
