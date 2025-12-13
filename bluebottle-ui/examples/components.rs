use bluebottle_ui::{color, font};
use iced::widget::{column, text};
use iced::{Element, Settings};

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
    .title("Bluebottle UI Components")
    .theme(color::theme())
    .settings(settings)
    .run()?;

    Ok(())
}

#[derive(Default)]
struct LoadingSpinners;

#[derive(Debug, Clone, Copy)]
enum Message {}

impl LoadingSpinners {
    fn update(&mut self, _message: Message) {}

    fn view(&self) -> Element<'_, Message> {
        column![text("Hello world, in Noto!").font(font::semibold())].into()
    }
}
