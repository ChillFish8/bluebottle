use bluebottle_ui::image::PersonSize;
use bluebottle_ui::{color, font, icon};
use iced::widget::{column, container, image, row, text};
use iced::{Center, Element, Length, Settings, padding};

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
        let elements = column![
            text_fonts(),
            icons(),
            nav_buttons(),
            standard_buttons(),
            icon_buttons(),
            icon_toggle_buttons(),
            posters(),
            episodes(),
            albums(),
            persons(),
            clickable_card(),
            bars(),
            pills(),
            pillboxes(),
            rating(),
        ]
        .width(Length::Fill)
        .padding(padding::all(32))
        .spacing(16);
        bluebottle_ui::scrollable::scrollable(elements).into()
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
                bluebottle_ui::button::nav(
                    "Anime Movies",
                    "movie",
                    false,
                    Message::Click
                ),
                bluebottle_ui::button::nav(
                    "Music",
                    "library_music",
                    false,
                    Message::Click
                ),
            ]
            .align_x(Center),
            column![
                bluebottle_ui::button::nav("Home", "home", true, Message::Click),
                bluebottle_ui::button::nav("Search", "search", true, Message::Click),
                bluebottle_ui::button::nav("Liked", "favorite", true, Message::Click),
                bluebottle_ui::button::nav("Anime", "draw", true, Message::Click),
                bluebottle_ui::button::nav("TV Shows", "tv", true, Message::Click),
                bluebottle_ui::button::nav("Movies", "movie", true, Message::Click),
                bluebottle_ui::button::nav(
                    "Anime Movies",
                    "movie",
                    true,
                    Message::Click
                ),
                bluebottle_ui::button::nav(
                    "Music",
                    "library_music",
                    true,
                    Message::Click
                ),
            ]
            .align_x(Center),
            column![
                bluebottle_ui::button::nav("Home", "home", true, Message::Click),
                bluebottle_ui::button::nav("Search", "search", false, Message::Click),
                bluebottle_ui::button::nav("Liked", "favorite", false, Message::Click),
                bluebottle_ui::button::nav("Anime", "draw", false, Message::Click),
                bluebottle_ui::button::nav("TV Shows", "tv", false, Message::Click),
                bluebottle_ui::button::nav("Movies", "movie", false, Message::Click),
                bluebottle_ui::button::nav(
                    "Anime Movies",
                    "movie",
                    false,
                    Message::Click
                ),
                bluebottle_ui::button::nav(
                    "Music",
                    "library_music",
                    false,
                    Message::Click
                ),
            ]
            .align_x(Center),
        ]
        .spacing(8)
    ]
    .spacing(4)
    .into()
}

fn standard_buttons() -> Element<'static, Message> {
    column![
        text("Standard Buttons").font(font::bold()),
        row![
            column![
                bluebottle_ui::button::standard(
                    "Episodes",
                    Some("subscriptions"),
                    false,
                    Message::Click
                ),
                bluebottle_ui::button::standard(
                    "Episodes",
                    Some("subscriptions"),
                    true,
                    Message::Click
                ),
            ]
            .spacing(8)
            .align_x(Center),
            column![
                bluebottle_ui::button::standard("Genres", None, false, Message::Click),
                bluebottle_ui::button::standard("Genres", None, true, Message::Click),
            ]
            .spacing(8)
            .align_x(Center),
        ]
        .spacing(8)
    ]
    .spacing(4)
    .into()
}

fn icon_buttons() -> Element<'static, Message> {
    column![
        text("Icon Buttons").font(font::bold()),
        row![
            bluebottle_ui::button::icon("settings", false, Message::Click),
            bluebottle_ui::button::icon("settings", true, Message::Click),
            bluebottle_ui::button::icon("arrow_back", false, Message::Click)
                .style(bluebottle_ui::button::secondary_button_style),
            bluebottle_ui::button::icon("arrow_back", true, Message::Click)
                .style(bluebottle_ui::button::secondary_button_style),
        ]
        .padding(8)
        .spacing(8)
    ]
    .spacing(4)
    .into()
}

fn icon_toggle_buttons() -> Element<'static, Message> {
    column![
        text("Icon Toggle Buttons").font(font::bold()),
        row![
            bluebottle_ui::button::toggle_icon(
                "favorite_border",
                "favorite",
                false,
                Message::Click
            ),
            bluebottle_ui::button::toggle_icon(
                "favorite_border",
                "favorite",
                true,
                Message::Click
            ),
            bluebottle_ui::button::toggle_icon(
                "cancel",
                "cancel",
                false,
                Message::Click
            ),
            bluebottle_ui::button::toggle_icon("cancel", "cancel", true, Message::Click),
        ]
        .padding(8)
        .spacing(8)
    ]
    .spacing(4)
    .into()
}

fn posters() -> Element<'static, Message> {
    let content = image::Handle::from_path("bluebottle-ui/assets/examples/poster1.jpg");

    column![
        text("Image Posters").font(font::bold()),
        row![
            bluebottle_ui::image::poster(
                content.clone(),
                bluebottle_ui::image::PosterSize::Large
            ),
            bluebottle_ui::image::poster(
                content.clone(),
                bluebottle_ui::image::PosterSize::Medium
            ),
            bluebottle_ui::image::poster(
                content,
                bluebottle_ui::image::PosterSize::Small
            ),
        ]
        .padding(8)
        .spacing(8)
    ]
    .spacing(4)
    .into()
}

fn episodes() -> Element<'static, Message> {
    let content =
        image::Handle::from_path("bluebottle-ui/assets/examples/thumbnail1.jpg");

    column![
        text("Image Episodes").font(font::bold()),
        row![bluebottle_ui::image::thumbnail(content)]
            .padding(8)
            .spacing(8)
    ]
    .spacing(4)
    .into()
}

fn albums() -> Element<'static, Message> {
    let content = image::Handle::from_path("bluebottle-ui/assets/examples/music1.jpg");

    column![
        text("Image Albums").font(font::bold()),
        row![bluebottle_ui::image::square(content)]
            .padding(8)
            .spacing(8)
    ]
    .spacing(4)
    .into()
}

fn persons() -> Element<'static, Message> {
    let content = image::Handle::from_path("bluebottle-ui/assets/examples/person1.jpg");

    column![
        text("Image Persons").font(font::bold()),
        row![
            bluebottle_ui::image::person(content.clone(), PersonSize::Poster),
            bluebottle_ui::image::person(content, PersonSize::Square),
        ]
        .padding(8)
        .spacing(8)
    ]
    .spacing(4)
    .into()
}

fn clickable_card() -> Element<'static, Message> {
    let poster = image::Handle::from_path("bluebottle-ui/assets/examples/poster1.jpg");
    let thumbnail =
        image::Handle::from_path("bluebottle-ui/assets/examples/thumbnail1.jpg");
    let square = image::Handle::from_path("bluebottle-ui/assets/examples/person2.jpg");

    column![
        text("Clickable Card").font(font::bold()),
        row![
            bluebottle_ui::card::card(
                "Example Poster",
                "Sample text",
                bluebottle_ui::image::poster(
                    poster,
                    bluebottle_ui::image::PosterSize::Small
                ),
                icon::filled("replay").color(color::TEXT_PRIMARY),
                Message::Click,
            ),
            bluebottle_ui::card::card(
                "Example Thumbnail",
                "Sample text",
                bluebottle_ui::image::thumbnail(thumbnail),
                icon::filled("replay").color(color::TEXT_PRIMARY),
                Message::Click,
            ),
            bluebottle_ui::card::card(
                "Example Square",
                "Sample text",
                bluebottle_ui::image::square(square),
                icon::filled("replay").color(color::TEXT_PRIMARY),
                Message::Click,
            ),
        ]
        .padding(8)
        .spacing(8)
    ]
    .spacing(4)
    .into()
}

fn bars() -> Element<'static, Message> {
    let topbar = bluebottle_ui::bar::top(text("center text"), "Example Library");

    let top_buttons = column![
        bluebottle_ui::button::nav("Home", "home", true, Message::Click),
        bluebottle_ui::button::nav("Search", "search", false, Message::Click),
        bluebottle_ui::button::nav("Liked", "favorite", false, Message::Click),
        bluebottle_ui::button::nav("Anime", "draw", false, Message::Click),
    ]
    .align_x(Center);

    let bottom_buttons = column![
        bluebottle_ui::button::nav("Library", "storage", false, Message::Click),
        bluebottle_ui::button::nav("Settings", "settings", false, Message::Click),
    ]
    .align_x(Center);

    let sidebar = bluebottle_ui::bar::side(top_buttons, bottom_buttons);
    let sidebar_container = container(sidebar).height(120);

    column![
        text("Topbar").font(font::bold()),
        topbar,
        text("Sidebar").font(font::bold()),
        sidebar_container,
    ]
    .spacing(8)
    .width(Length::Fill)
    .into()
}

fn pills() -> Element<'static, Message> {
    let no_icon_small = row![
        bluebottle_ui::pill::small("Small Enabled", None).on_press(Message::Click),
        bluebottle_ui::pill::small("Small Disabled", None),
    ]
    .spacing(4);

    let icon_small = row![
        bluebottle_ui::pill::small("Small Icon Enabled", Some("access_time_filled"))
            .on_press(Message::Click),
        bluebottle_ui::pill::small("Small Icon Disabled", Some("access_time_filled")),
    ]
    .spacing(4);

    let no_icon_regular = row![
        bluebottle_ui::pill::regular("24m", None).on_press(Message::Click),
        bluebottle_ui::pill::regular("24m", None),
    ]
    .spacing(4);

    let icon_regular = row![
        bluebottle_ui::pill::regular("24m remaining", Some("access_time_filled"))
            .on_press(Message::Click),
        bluebottle_ui::pill::regular("24m remaining", Some("access_time_filled")),
    ]
    .spacing(4);

    column![
        text("Pills").font(font::bold()),
        column![no_icon_small, icon_small].spacing(8),
        column![no_icon_regular, icon_regular].spacing(8),
    ]
    .spacing(8)
    .into()
}

fn pillboxes() -> Element<'static, Message> {
    let tags_labels = [
        "Elves",
        "Magic",
        "Immortality",
        "Friendships",
        "Slice of lift",
        "Female protagonist",
        "Magic",
        "Elf",
        "Dragons",
    ];

    let genres_labels = ["Fantasy", "Drama", "Animation", "Adventure", "Anime"];

    let tags_labels = tags_labels
        .into_iter()
        .map(|label| bluebottle_ui::pill::small(label, None).into());

    let genres_labels = genres_labels
        .into_iter()
        .map(|label| bluebottle_ui::pill::small(label, None).into());

    column![
        text("Pill Boxes").font(font::bold()),
        container(bluebottle_ui::pill_box::pill_box("Tags", tags_labels)).width(200),
        bluebottle_ui::pill_box::pill_box("Genres", genres_labels),
    ]
    .spacing(8)
    .into()
}

fn rating() -> Element<'static, Message> {
    column![
        text("Rating").font(font::bold()),
        bluebottle_ui::rating::rating(Some("7"), Some("88")),
        bluebottle_ui::rating::rating(None, Some("88")),
        bluebottle_ui::rating::rating(Some("7"), None),
    ]
    .spacing(8)
    .into()
}
