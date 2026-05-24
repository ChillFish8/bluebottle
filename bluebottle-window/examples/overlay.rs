//! Interactive smoke test for `bluebottle-window`.
//!
//! Opens an overlay window with a clickable counter rendered by Iced over an
//! opaque "video" stand-in on the main surface, then closes after a while.
//! Click the button (or press it) to confirm input reaches the Iced UI.

use std::time::Duration;

use iced::widget::{button, column, text};

#[derive(Debug, Clone)]
enum Message {
    Increment,
}

fn update(state: &mut u32, message: Message) -> iced::Task<Message> {
    match message {
        Message::Increment => {
            *state += 1;
            tracing::info!("counter incremented to {state}");
        },
    }
    iced::Task::none()
}

fn view(state: &u32) -> iced::Element<'_, Message> {
    column![
        text(format!("count: {state}")).size(32),
        button("increment").on_press(Message::Increment),
    ]
    .spacing(16)
    .padding(24)
    .into()
}

fn main() {
    tracing_subscriber::fmt::init();

    let window =
        bluebottle_window::create_overlay(|| iced::application(|| 0_u32, update, view))
            .expect("create overlay window");

    println!(
        "overlay ready: {}x{} @ scale {}",
        window.size().0,
        window.size().1,
        window.scale_factor(),
    );

    std::thread::sleep(Duration::from_secs(20));

    window.request_close();
    window.join().expect("overlay loop exited cleanly");
}
