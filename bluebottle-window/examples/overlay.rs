//! Minimal smoke test for `bluebottle-window`.
//!
//! Opens an overlay window for a few seconds, then closes it. During Phase 2
//! the overlay is a translucent shared-memory fill over an opaque "video"
//! stand-in; later phases render a real Iced UI here instead.

use std::time::Duration;

use iced::widget::text;

#[derive(Debug, Clone)]
enum Message {}

fn view(_state: &u32) -> iced::Element<'_, Message> {
    text("bluebottle overlay").into()
}

fn main() {
    tracing_subscriber::fmt::init();

    let window = bluebottle_window::create_overlay(|| {
        iced::application(
            || 0_u32,
            |_state: &mut u32, _message: Message| iced::Task::none(),
            view,
        )
    })
    .expect("create overlay window");

    println!(
        "overlay ready: {}x{} @ scale {}",
        window.size().0,
        window.size().1,
        window.scale_factor(),
    );

    std::thread::sleep(Duration::from_secs(4));

    window.request_close();
    window.join().expect("overlay loop exited cleanly");
}
