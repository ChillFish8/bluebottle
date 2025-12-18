use std::sync::LazyLock;

use bluebottle_ui::image::{PersonSize, PosterSize};
use bluebottle_ui::{color, font, icon};
use iced::widget::{column, container, image, row, text};
use iced::{Center, Element, Length, Settings, padding};

static POSTER: LazyLock<image::Handle> = LazyLock::new(|| {
    image::Handle::from_path("bluebottle-ui/assets/examples/poster1.jpg")
});
static THUMBNAIL: LazyLock<image::Handle> = LazyLock::new(|| {
    image::Handle::from_path("bluebottle-ui/assets/examples/thumbnail1.jpg")
});
static PERSON_POSTER: LazyLock<image::Handle> = LazyLock::new(|| {
    image::Handle::from_path("bluebottle-ui/assets/examples/person1.jpg")
});
static SQUARE: LazyLock<image::Handle> = LazyLock::new(|| {
    image::Handle::from_path("bluebottle-ui/assets/examples/music1.jpg")
});

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

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
struct Components {
    search_content: String,
}

#[derive(Debug, Clone)]
enum Message {
    Click,
    SearchInput(String),
}

impl Components {
    fn update(&mut self, message: Message) {
        match message {
            Message::SearchInput(content) => {
                self.search_content = content;
            },
            _ => {},
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let elements = column![
            text_fonts(),
            ellipsis_text(),
            icons(),
            nav_buttons(),
            standard_buttons(),
            icon_buttons(),
            icon_toggle_buttons(),
            navigators(),
            posters(),
            episodes(),
            albums(),
            persons(),
            clickable_card(),
            bars(),
            pills(),
            pillboxes(),
            rating(),
            titles(),
            search_input(&self.search_content),
            inputs(&self.search_content),
            spinners(),
            skeletons(),
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

fn ellipsis_text() -> Element<'static, Message> {
    column![
        text("Text Ellipsis").font(font::bold()),
        column![
            bluebottle_ui::ellipsis_text::ellipsis_text(
                "The quick brown fox jumps over the lazy dog"
            )
            .width(160)
            .height(50),
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

fn navigators() -> Element<'static, Message> {
    column![
        text("Carousel Navigators").font(font::bold()),
        row![
            bluebottle_ui::carousel_navigator::navigator(
                1,
                7,
                Message::Click,
                Message::Click,
            ),
            bluebottle_ui::carousel_navigator::navigator(
                4,
                7,
                Message::Click,
                Message::Click,
            ),
            bluebottle_ui::carousel_navigator::navigator(
                7,
                7,
                Message::Click,
                Message::Click,
            ),
        ]
        .padding(8)
        .spacing(8)
    ]
    .spacing(4)
    .into()
}

fn posters() -> Element<'static, Message> {
    let content = POSTER.clone();

    column![
        text("Image Posters").font(font::bold()),
        row![
            bluebottle_ui::image::poster(content.clone(), PosterSize::Large),
            bluebottle_ui::image::poster_skeleton(PosterSize::Large),
        ]
        .padding(8)
        .spacing(8),
        row![
            bluebottle_ui::image::poster(content.clone(), PosterSize::Medium),
            bluebottle_ui::image::poster_skeleton(PosterSize::Medium),
        ]
        .padding(8)
        .spacing(8),
        row![
            bluebottle_ui::image::poster(content, PosterSize::Small),
            bluebottle_ui::image::poster_skeleton(PosterSize::Small),
        ]
        .padding(8)
        .spacing(8),
    ]
    .spacing(4)
    .into()
}

fn episodes() -> Element<'static, Message> {
    let content = THUMBNAIL.clone();

    column![
        text("Image Episodes").font(font::bold()),
        row![
            bluebottle_ui::image::thumbnail(content),
            bluebottle_ui::image::thumbnail_skeleton(),
        ]
        .padding(8)
        .spacing(8)
    ]
    .spacing(4)
    .into()
}

fn albums() -> Element<'static, Message> {
    let content = SQUARE.clone();

    column![
        text("Image Albums").font(font::bold()),
        row![
            bluebottle_ui::image::square(content),
            bluebottle_ui::image::square_skeleton(),
        ]
        .padding(8)
        .spacing(8)
    ]
    .spacing(4)
    .into()
}

fn persons() -> Element<'static, Message> {
    let content = PERSON_POSTER.clone();

    column![
        text("Image Persons").font(font::bold()),
        row![
            bluebottle_ui::image::person(content.clone(), PersonSize::Poster),
            bluebottle_ui::image::person_skeleton(PersonSize::Poster),
            bluebottle_ui::image::person(content, PersonSize::Square),
            bluebottle_ui::image::person_skeleton(PersonSize::Square),
        ]
        .padding(8)
        .spacing(8)
    ]
    .spacing(4)
    .into()
}

fn clickable_card() -> Element<'static, Message> {
    let poster = POSTER.clone();
    let thumbnail = THUMBNAIL.clone();
    let square = SQUARE.clone();

    column![
        text("Clickable Card").font(font::bold()),
        row![
            bluebottle_ui::card::card(
                "Example Poster",
                "Sample text",
                bluebottle_ui::image::poster(poster, PosterSize::Small),
                icon::filled("replay").color(color::TEXT_PRIMARY),
                Message::Click,
            ),
            bluebottle_ui::card::skeleton(bluebottle_ui::image::poster_skeleton(
                PosterSize::Small
            )),
        ]
        .padding(8)
        .spacing(8),
        row![
            bluebottle_ui::card::card(
                "Example Thumbnail",
                "Sample text",
                bluebottle_ui::image::thumbnail(thumbnail),
                icon::filled("replay").color(color::TEXT_PRIMARY),
                Message::Click,
            ),
            bluebottle_ui::card::skeleton(bluebottle_ui::image::thumbnail_skeleton()),
        ]
        .padding(8)
        .spacing(8),
        row![
            bluebottle_ui::card::card(
                "Example Square",
                "Sample text",
                bluebottle_ui::image::square(square),
                icon::filled("replay").color(color::TEXT_PRIMARY),
                Message::Click,
            ),
            bluebottle_ui::card::skeleton(bluebottle_ui::image::square_skeleton()),
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

fn titles() -> Element<'static, Message> {
    column![
        text("Titles").font(font::bold()),
        bluebottle_ui::title::title(Some("local_fire_department"), "New releases"),
        bluebottle_ui::title::title(None, "Setting option A"),
    ]
    .spacing(8)
    .into()
}

fn search_input(content: &str) -> Element<'_, Message> {
    column![
        text("Search input").font(font::bold()),
        bluebottle_ui::search::search("Sample input...", content, Message::SearchInput),
    ]
    .spacing(8)
    .into()
}

fn inputs(content: &str) -> Element<'_, Message> {
    column![
        text("Text input").font(font::bold()),
        bluebottle_ui::input::text_input(
            "Sample input...",
            content,
            Message::SearchInput
        ),
        bluebottle_ui::input::text_input("Password...", content, Message::SearchInput)
            .secure(true),
    ]
    .spacing(8)
    .into()
}

fn spinners() -> Element<'static, Message> {
    column![
        text("Spinners").font(font::bold()),
        bluebottle_ui::spinner::linear(),
        bluebottle_ui::spinner::circle().size(40),
    ]
    .spacing(8)
    .into()
}

fn skeletons() -> Element<'static, Message> {
    column![
        text("Skeletons").font(font::bold()),
        bluebottle_ui::skeleton::skeleton().height(224).width(224),
    ]
    .spacing(8)
    .into()
}
