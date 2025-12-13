use bluebottle_ui::{color, font};
use iced::widget::{column, text, row, container};
use iced::{padding, Element, Settings};

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
        column![
            text_fonts(),
            colors(),
        ].padding(padding::all(32)).into()
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
        ].spacing(4).padding(padding::left(16)),
    ].into()
}

fn colors() -> Element<'static, Message> {
    column![
        text("Colors").font(font::bold()),
        column![
            row![
                container("")
                .width(64)
                .height(64)
                .style(container::rounded_box)
            ]
        ].spacing(4).padding(padding::left(16)),
    ].into()
}