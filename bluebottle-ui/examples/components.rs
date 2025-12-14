use bluebottle_ui::{color, font, icon};
use iced::widget::{column, container, row, text};
use iced::{Element, Settings, padding};

fn main() -> anyhow::Result<()> {
    let settings = Settings {
        fonts: font::required_fonts(),
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
        column![text_fonts(), icons(),]
            .padding(padding::all(32))
            .spacing(16)
            .into()
    }
}

fn text_fonts() -> Element<'static, Message> {
    column![
        text("Text Fonts").font(font::bold()),
        column![
            text("The quick brown fox jumps over the lazy dog").font(font::regular()),
            text("The quick brown fox jumps over the lazy dog").font(font::semibold()),
            text("The quick brown fox jumps over the lazy dog").font(font::bold()),
            text("The quick brown fox jumps over the lazy dog").size(12),
            text("The quick brown fox jumps over the lazy dog").size(14),
            text("The quick brown fox jumps over the lazy dog").size(16),
        ]
        .spacing(4)
        .padding(padding::left(16)),
    ]
    .into()
}

fn icons() -> Element<'static, Message> {
    column![
        text("Icons").font(font::bold()),
        row![
            icon::outline("home").size(48),
            icon::filled("home").size(48),
            icon::outline("favorite_border").size(48),
            icon::filled("favorite").size(48),
        ]
        .spacing(4)
        .padding(padding::left(16)),
    ]
    .into()
}
