//! Interactive smoke test for `bluebottle-window`.
//!
//! Opens an overlay window rendered by Iced over an opaque "video" stand-in on
//! the main surface, then closes after a while. Exercises:
//! - input: click "increment" to bump the counter;
//! - async `Task`: each increment kicks off a 500ms async "ping";
//! - `Subscription`: a 1s timer ticks regardless of input.

use std::time::Duration;

use iced::widget::{button, column, text};

#[derive(Debug, Default)]
struct App {
    count: u32,
    ticks: u32,
}

#[derive(Debug, Clone)]
enum Message {
    Increment,
    Pinged,
    Tick,
}

fn update(state: &mut App, message: Message) -> iced::Task<Message> {
    match message {
        Message::Increment => {
            state.count += 1;
            tracing::info!("counter incremented to {}", state.count);
            return iced::Task::perform(
                async { tokio::time::sleep(Duration::from_millis(500)).await },
                |()| Message::Pinged,
            );
        },
        Message::Pinged => tracing::info!("async ping completed"),
        Message::Tick => {
            state.ticks += 1;
            tracing::info!("subscription tick {}", state.ticks);
        },
    }
    iced::Task::none()
}

fn view(state: &App) -> iced::Element<'_, Message> {
    column![
        text(format!("count: {}", state.count)).size(32),
        text(format!("ticks: {}", state.ticks)).size(20),
        button("increment").on_press(Message::Increment),
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

    println!(
        "overlay ready: {}x{} @ scale {}",
        window.size().0,
        window.size().1,
        window.scale_factor(),
    );

    std::thread::sleep(Duration::from_secs(15));

    window.request_close();
    window.join().expect("overlay loop exited cleanly");
}
