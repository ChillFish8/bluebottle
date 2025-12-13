use std::borrow::Cow;
use iced::{Element, Font, Settings};
use iced::widget::{column, text};

use bluebottle_ui::font;

fn main() -> anyhow::Result<()> {
    let settings = Settings {
        fonts: font::noto_fonts(),
        default_font: font::regular(),
        ..Default::default()
    };

    iced::application(
        LoadingSpinners::default,
        LoadingSpinners::update,
        LoadingSpinners::view,
    )
    .settings(settings)
    .run()?;

    Ok(())
}

#[derive(Default)]
struct LoadingSpinners;

#[derive(Debug, Clone, Copy)]
enum Message {
}

impl LoadingSpinners {
    fn update(&mut self, _message: Message) {

    }

    fn view(&self) -> Element<'_, Message> {
        column![
            text("Hello world, in Noto!")
            .font(font::semibold())
        ].into()
    }
}
