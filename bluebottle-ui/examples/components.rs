use std::sync::LazyLock;

use bluebottle_ui::image::{PersonSize, PosterSize};
use bluebottle_ui::splash_background::{Backdrop, splash_background, splash_panel};
use bluebottle_ui::{color, font, icon};
use iced::widget::{column, container, image, row, text};
use iced::{Center, Element, Length, Settings, padding};
use snafu::ResultExt;

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
static SPLASH_BACKDROP: LazyLock<Option<Backdrop>> = LazyLock::new(|| {
    let path = "bluebottle-ui/assets/examples/poster1.jpg";
    let reader = ::image::ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?;
    let rgba = reader.decode().ok()?.to_rgba8();
    let (width, height) = rgba.dimensions();
    Some(Backdrop::new(rgba.into_raw(), width, height))
});

fn main() -> Result<(), snafu::Whatever> {
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
        .run()
        .whatever_context("run UI")?;

    Ok(())
}

struct Components {
    search_content: String,
    selected_tab: usize,
    selected_icon_tab: usize,
    smart_list_show: (Option<usize>, Option<usize>),
    smart_list_shown: Option<usize>,
    smart_list_hydrated: bool,
}

impl Default for Components {
    fn default() -> Self {
        Self {
            search_content: String::new(),
            selected_tab: 0,
            selected_icon_tab: 1,
            smart_list_show: (None, None),
            smart_list_shown: None,
            smart_list_hydrated: false,
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    Click,
    CardLabel,
    CardSubtext,
    LinkPressed(&'static str),
    SearchInput(String),
    TabSelected(usize),
    IconTabSelected(usize),
    SmartListJump(Option<usize>, Option<usize>),
    SmartListShown(usize),
    SmartListHydrate,
    SmartListTargetFinished,
}

impl Components {
    fn update(&mut self, message: Message) {
        match message {
            Message::SearchInput(content) => {
                self.search_content = content;
            },
            Message::LinkPressed(name) => {
                println!("link pressed: {name}");
            },
            Message::TabSelected(i) => {
                self.selected_tab = i;
            },
            Message::IconTabSelected(i) => {
                self.selected_icon_tab = i;
            },
            Message::SmartListJump(group, child) => {
                self.smart_list_show = (group, child);
            },
            Message::SmartListShown(index) => {
                self.smart_list_shown = Some(index);
            },
            Message::SmartListHydrate => {
                self.smart_list_hydrated = !self.smart_list_hydrated;
            },
            Message::SmartListTargetFinished => {
                // Clear the sticky show_group so the next click on the same
                // jump button re-triggers the animation.
                self.smart_list_show = (None, None);
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
            clickables(),
            navigators(),
            posters(),
            episodes(),
            albums(),
            persons(),
            links(),
            tabs(self.selected_tab, self.selected_icon_tab),
            media_images(),
            clickable_card(),
            splash_backgrounds(),
            bars(),
            pills(),
            pillboxes(),
            rating(),
            titles(),
            breadcrumbs(),
            search_input(&self.search_content),
            inputs(&self.search_content),
            spinners(),
            skeletons(),
            separators(),
            smart_list_demo(
                self.smart_list_show,
                self.smart_list_shown,
                self.smart_list_hydrated,
            ),
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
        text("Text Paragraphs").font(font::bold()),
        column![bluebottle_ui::text::paragraph(
            "The quick brown fox jumps over the lazy dog"
        ),]
        .spacing(4)
        .padding(padding::left(16)),
        text("Text Subheading").font(font::bold()),
        column![bluebottle_ui::text::subheading(
            "The quick brown fox jumps over the lazy dog"
        ),]
        .spacing(4)
        .padding(padding::left(16)),
        text("Text Label").font(font::bold()),
        column![bluebottle_ui::text::label(
            "The quick brown fox jumps over the lazy dog"
        ),]
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
                bluebottle_ui::button::disabled(Some("Disabled"), Some("subscriptions")),
            ]
            .spacing(8)
            .align_x(Center),
            column![
                bluebottle_ui::button::standard("Genres", None, false, Message::Click),
                bluebottle_ui::button::standard("Genres", None, true, Message::Click),
                bluebottle_ui::button::disabled(Some("Disabled"), None,),
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
            bluebottle_ui::button::disabled(None, Some("arrow_back"),),
        ]
        .padding(8)
        .spacing(8)
    ]
    .spacing(4)
    .into()
}

fn clickables() -> Element<'static, Message> {
    use bluebottle_ui::clickable;

    column![
        text("Clickables").font(font::bold()),
        row![
            Element::<Message>::from(clickable(text("Inert")).padding([6, 12])),
            Element::<Message>::from(
                clickable(text("Default"))
                    .padding([6, 12])
                    .on_press(Message::Click),
            ),
            Element::<Message>::from(
                clickable(text("Primary tint"))
                    .padding([6, 12])
                    .tint(color::PRIMARY)
                    .on_press(Message::Click),
            ),
            Element::<Message>::from(
                clickable(text("Square tile"))
                    .padding([12, 16])
                    .radius(8.0)
                    .on_press(Message::Click),
            ),
        ]
        .padding(8)
        .spacing(8)
        .align_y(Center),
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

fn links() -> Element<'static, Message> {
    column![
        text("Links").font(font::bold()),
        row![
            bluebottle_ui::link("Default", Message::LinkPressed("default")),
            bluebottle_ui::link(
                "Large semibold",
                Message::LinkPressed("large-semibold"),
            )
            .size(font::TEXT_LARGE)
            .font(font::semibold()),
            bluebottle_ui::link("Secondary tint", Message::LinkPressed("secondary"))
                .size(font::TEXT_SMALL)
                .color(color::TEXT_SECONDARY),
            bluebottle_ui::link("Inline within a row", Message::LinkPressed("inline"),),
        ]
        .padding(padding::left(16))
        .spacing(16),
    ]
    .spacing(4)
    .into()
}

fn tabs(selected: usize, selected_icon: usize) -> Element<'static, Message> {
    let text_tab = |label: &'static str| -> Element<'static, Message> {
        text(label).size(font::TEXT_MEDIUM).into()
    };

    let icon_tab =
        |icon_name: &'static str, label: &'static str| -> Element<'static, Message> {
            row![
                icon::filled(icon_name).size(20),
                text(label).size(font::TEXT_MEDIUM),
            ]
            .spacing(8)
            .align_y(Center)
            .into()
        };

    column![
        text("Tabs").font(font::bold()),
        bluebottle_ui::tabs(
            [
                text_tab("Overview"),
                text_tab("Episodes"),
                text_tab("Reviews"),
            ],
            selected,
            Message::TabSelected,
        ),
        bluebottle_ui::tabs(
            [
                icon_tab("home", "Home"),
                icon_tab("movie", "Movies"),
                icon_tab("tv", "Shows"),
                icon_tab("music_note", "Music"),
            ],
            selected_icon,
            Message::IconTabSelected,
        ),
    ]
    .spacing(12)
    .into()
}

fn media_images() -> Element<'static, Message> {
    let play_overlay = || {
        container(
            icon::filled("play_arrow")
                .color(color::TEXT_DEFAULT)
                .size(48),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Center)
        .align_y(Center)
    };

    // Inert. No press, no hover affordances, no pointer cursor.
    let inert = bluebottle_ui::media_image(bluebottle_ui::image::poster(
        POSTER.clone(),
        PosterSize::Small,
    ));

    // Clickable with the default primary border on hover.
    let clickable = bluebottle_ui::media_image(bluebottle_ui::image::poster(
        POSTER.clone(),
        PosterSize::Small,
    ))
    .on_press(Message::LinkPressed("media-image-default"));

    // Clickable, border disabled. Shadow, tint, and overlay scale-in still
    // animate.
    let no_border =
        bluebottle_ui::media_image(bluebottle_ui::image::thumbnail(THUMBNAIL.clone()))
            .on_press(Message::LinkPressed("media-image-no-border"))
            .border(false);

    // Clickable with an overlay that scales in from the centre on hover.
    let with_overlay =
        bluebottle_ui::media_image(bluebottle_ui::image::square(SQUARE.clone()))
            .overlay(play_overlay())
            .on_press(Message::LinkPressed("media-image-overlay"));

    column![
        text("Media Images").font(font::bold()),
        row![inert, clickable, no_border, with_overlay,]
            .padding(8)
            .spacing(8),
    ]
    .spacing(4)
    .into()
}

fn clickable_card() -> Element<'static, Message> {
    let label_text =
        |s: &'static str| text(s).size(font::TEXT_MEDIUM).color(color::TEXT_DEFAULT);
    let subtext_text =
        |s: &'static str| text(s).size(font::TEXT_SMALL).color(color::TEXT_SECONDARY);

    // Bare image, no click.
    let non_interactive = bluebottle_ui::media_card(bluebottle_ui::image::poster(
        POSTER.clone(),
        PosterSize::Small,
    ));

    // Image + label only, single press.
    let image_only = bluebottle_ui::media_card(bluebottle_ui::image::poster(
        POSTER.clone(),
        PosterSize::Small,
    ))
    .label(label_text("Poster Only"))
    .on_press(Message::Click);

    let play_overlay = || {
        container(
            icon::filled("play_arrow")
                .color(color::TEXT_DEFAULT)
                .size(48),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Center)
        .align_y(Center)
    };

    // Image + overlay + label + subtext, press fires only on the image.
    let with_overlay =
        bluebottle_ui::media_card(bluebottle_ui::image::thumbnail(THUMBNAIL.clone()))
            .overlay(play_overlay())
            .label(label_text("With Overlay"))
            .subtext(subtext_text("Only the image fires Click"))
            .on_press(Message::Click);

    // Image + label + subtext where the label and subtext are clickable
    // links. Each link has its own message, animates an underline on hover,
    // and tints to primary while pressed.
    let per_region =
        bluebottle_ui::media_card(bluebottle_ui::image::square(SQUARE.clone()))
            .overlay(play_overlay())
            .label(
                bluebottle_ui::link("Per-region Press", Message::CardLabel)
                    .size(font::TEXT_MEDIUM),
            )
            .subtext(
                bluebottle_ui::link(
                    "Each row has its own message",
                    Message::CardSubtext,
                )
                .size(font::TEXT_SMALL)
                .color(color::TEXT_SECONDARY),
            )
            .on_press(Message::Click);

    column![
        text("Clickable Card").font(font::bold()),
        row![
            non_interactive,
            image_only,
            with_overlay,
            per_region,
            bluebottle_ui::media_card::skeleton(bluebottle_ui::image::poster_skeleton(
                PosterSize::Small
            ))
            .label()
            .subtext(),
        ]
        .padding(8)
        .spacing(8),
    ]
    .spacing(4)
    .into()
}

fn splash_backgrounds() -> Element<'static, Message> {
    let backdrop = SPLASH_BACKDROP.clone();

    column![
        text("Splash Backgrounds").font(font::bold()),
        row![
            container(splash_background(backdrop.clone()))
                .width(Length::FillPortion(1))
                .height(320),
            container(splash_panel(backdrop))
                .width(Length::FillPortion(1))
                .height(320),
        ]
        .spacing(8)
        .padding(padding::left(16)),
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
    .spacing(8.0)
    .align_x(Center);

    let bottom_buttons = column![
        bluebottle_ui::button::nav("Library", "storage", false, Message::Click),
        bluebottle_ui::button::nav("Settings", "settings", false, Message::Click),
    ]
    .spacing(8.0)
    .align_x(Center);

    let sidebar = bluebottle_ui::bar::side(top_buttons, bottom_buttons);
    let sidebar_container = container(sidebar).height(600);

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
        bluebottle_ui::text::title(Some("local_fire_department"), "New releases"),
        bluebottle_ui::text::title(None, "Setting option A"),
    ]
    .spacing(8)
    .into()
}

fn breadcrumbs() -> Element<'static, Message> {
    column![
        text("Breadcrumbs").font(font::bold()),
        bluebottle_ui::breadcrumb::breadcrumb(&["Library"]),
        bluebottle_ui::breadcrumb::breadcrumb(&["Library", "Anime"]),
        bluebottle_ui::breadcrumb::breadcrumb(&[
            "Library",
            "Anime",
            "Dusk Beyond the End of the World",
        ]),
        bluebottle_ui::breadcrumb::breadcrumb(&[
            "Library",
            "Anime",
            "Dusk Beyond the End of the World",
        ])
        .size(20),
        bluebottle_ui::breadcrumb::breadcrumb(&[
            "Library",
            "Anime",
            "Dusk Beyond the End of the World",
        ])
        .size(font::TEXT_SMALL),
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

fn separators() -> Element<'static, Message> {
    column![
        text("Separators").font(font::bold()),
        bluebottle_ui::separator::seperator(Length::Fixed(400.0))
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

fn smart_list_demo(
    show: (Option<usize>, Option<usize>),
    shown: Option<usize>,
    hydrated: bool,
) -> Element<'static, Message> {
    use bluebottle_ui::{skeleton, smart_group, smart_list};

    let group_labels = [
        "Recently Added",
        "Trending",
        "Continue Watching",
        "Editor's Picks",
    ];
    let row_count = [6_usize, 8, 5, 7];

    let groups = (0..group_labels.len()).map(|gi| {
        let header = text(group_labels[gi])
            .size(font::TEXT_LARGE)
            .font(font::semibold());

        let children: Vec<Element<'static, Message>> = (0..row_count[gi])
            .map(|ci| {
                if hydrated {
                    container(text(format!("Group {gi} \u{2022} Row {ci}")))
                        .padding(padding::all(8))
                        .height(Length::Fixed(36.0))
                        .width(Length::Fill)
                        .into()
                } else {
                    container(skeleton::skeleton().height(20).width(Length::Fill))
                        .padding(padding::all(8))
                        .height(Length::Fixed(72.0))
                        .width(Length::Fill)
                        .into()
                }
            })
            .collect();

        smart_group(header, children)
    });

    let list = smart_list(groups, Message::SmartListShown)
        .show_group(show.0)
        .show_child(show.1)
        .on_target_finished(|| Message::SmartListTargetFinished)
        .spacing(20.0);

    let controls = row![
        bluebottle_ui::button::standard(
            "Jump to Trending header",
            None,
            false,
            Message::SmartListJump(Some(1), None),
        ),
        bluebottle_ui::button::standard(
            "Centre on Picks / row 4",
            None,
            false,
            Message::SmartListJump(Some(3), Some(4)),
        ),
        bluebottle_ui::button::standard(
            if hydrated {
                "Show skeletons"
            } else {
                "Hydrate"
            },
            None,
            false,
            Message::SmartListHydrate,
        ),
    ]
    .padding(padding::left(16))
    .spacing(8);

    let shown_text = shown
        .map(|i| format!("Shown group: {i} ({})", group_labels[i]))
        .unwrap_or_else(|| "Shown group: none yet".into());

    column![
        text("Smart List").font(font::bold()),
        controls,
        text(shown_text)
            .size(font::TEXT_SMALL)
            .color(color::TEXT_SECONDARY),
        container(list)
            .height(Length::Fixed(320.0))
            .width(Length::Fill)
            .padding(padding::left(16)),
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
