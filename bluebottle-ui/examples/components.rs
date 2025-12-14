use bluebottle_ui::{color, font, icon};
use iced::widget::{column, container, row, scrollable, text};
use iced::{Center, Element, Settings, padding, Length};

fn main() -> anyhow::Result<()> {
    let settings = Settings {
        fonts: font::required_fonts(),
        default_font: font::regular(),
        ..Default::default()
    };

    iced::application(Components::default, Components::update, Components::view)
        .title("Bluebottle UI Components")
        .theme(color::theme())
        .settings(settings)
        .run()?;

    Ok(())
}

#[derive(Default)]
struct Components;

#[derive(Debug, Clone, Copy)]
enum Message {
    Click,
}

impl Components {
    fn update(&mut self, _message: Message) {}

    fn view(&self) -> Element<'_, Message> {
        let elements = column![text_fonts(), icons(), nav_buttons(), nav_buttons(),]
            .width(Length::Fill)
            .padding(padding::all(32))
            .spacing(16);
        scrollable(elements).into()
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

fn nav_buttons() -> Element<'static, Message> {
    column![
        text("Nav Buttons").font(font::bold()),
        row![
            column![
                bluebottle_ui::button::nav("Home", "home", false, Message::Click),
                bluebottle_ui::button::nav("Search", "search", false, Message::Click),
                bluebottle_ui::button::nav("Liked", "favorite", false, Message::Click),
                bluebottle_ui::button::nav("Anime", "draw", false, Message::Click),
                bluebottle_ui::button::nav("TV Shows", "tv", false, Message::Click),
                bluebottle_ui::button::nav("Movies", "movie", false, Message::Click),
                bluebottle_ui::button::nav("Music", "library_music", false, Message::Click),
            ]
            .align_x(Center),
            column![
                bluebottle_ui::button::nav("Home", "home", true, Message::Click),
                bluebottle_ui::button::nav("Search", "search", true, Message::Click),
                bluebottle_ui::button::nav("Liked", "favorite", true, Message::Click),
                bluebottle_ui::button::nav("Anime", "draw", true, Message::Click),
                bluebottle_ui::button::nav("TV Shows", "tv", true, Message::Click),
                bluebottle_ui::button::nav("Movies", "movie", true, Message::Click),
                bluebottle_ui::button::nav("Music", "library_music", true, Message::Click),
            ]
            .align_x(Center),
            column![
                bluebottle_ui::button::nav("Home", "home", true, Message::Click),
                bluebottle_ui::button::nav("Search", "search", false, Message::Click),
                bluebottle_ui::button::nav("Liked", "favorite", false, Message::Click),
                bluebottle_ui::button::nav("Anime", "draw", false, Message::Click),
                bluebottle_ui::button::nav("TV Shows", "tv", false, Message::Click),
                bluebottle_ui::button::nav("Movies", "movie", false, Message::Click),
                bluebottle_ui::button::nav("Music", "library_music", false, Message::Click),
            ]
            .align_x(Center),
        ]
        .spacing(8)
    ]
    .into()
}

fn standard_buttons() -> Element<'static, Message> {
    column![].into()
}
