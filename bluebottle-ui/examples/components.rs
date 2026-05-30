use std::sync::LazyLock;

use bluebottle_ui::image::{PersonSize, PosterSize};
use bluebottle_ui::splash_background::{Backdrop, splash_background, splash_panel};
use bluebottle_ui::{clickable, color, font, icon};
use iced::widget::{column, container, image, row, stack, text};
use iced::{
    Background,
    Border,
    Center,
    Color,
    Element,
    Length,
    Right,
    Settings,
    Top,
    padding,
};

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

fn main() {
    tracing_subscriber::fmt::init();

    // Show an animated splash on the main surface while the gallery compiles its
    // shaders and settles its first frame. No real logo asset exists yet, so a
    // square example image stands in; swapping in a real logo is a one line change.
    let logo = ::image::ImageReader::open("bluebottle-ui/assets/examples/music1.jpg")
        .ok()
        .and_then(|reader| reader.with_guessed_format().ok())
        .and_then(|reader| reader.decode().ok())
        .expect("decode splash logo");
    let splash = bluebottle_window::Splash::new(logo, color::BG);

    // Run the gallery as a bluebottle-window overlay rather than a plain
    // iced::application so the render loop FPS cap actually applies. The
    // always-animating demos (splash shader, spinners, skeletons, the FPS
    // counter) would otherwise present as fast as the compositor allows.
    let window = bluebottle_window::create_overlay_with_splash(
        || {
            let settings = Settings {
                fonts: font::required_fonts(),
                default_font: font::regular(),
                ..Default::default()
            };

            iced::application(Components::default, Components::update, Components::view)
                .title("Bluebottle UI Components")
                .theme(|_state: &Components| color::theme())
                .settings(settings)
        },
        splash,
    )
    .expect("create overlay window");

    // Throttle to 60fps. The floating counter should read about this.
    window.set_max_fps(Some(60));

    // The library owns and paints the main surface, so just keep the window open
    // until it is closed. The overlay render thread keeps the animated UI going.
    window.join().expect("overlay loop exited cleanly");
}

struct Components {
    search_content: String,
    selected_tab: usize,
    selected_icon_tab: usize,
    smart_list_show: (Option<usize>, Option<usize>),
    smart_list_shown: Option<usize>,
    smart_list_hydrated: bool,
    selected_accent: color::Accent,
    selected_nav: usize,
    toggle_states: [bool; 3],
    icon_states: [bool; 4],
    icon_flat_states: [bool; 2],
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
            selected_accent: color::Accent::Default,
            selected_nav: 0,
            toggle_states: [true, false, false],
            icon_states: [false, true, false, true],
            icon_flat_states: [false, true],
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
    AccentSelected(color::Accent),
    NavSelected(usize),
    ToggleToolbar(usize),
    ToggleIcon(usize),
    ToggleIconFlat(usize),
}

fn toggle_at(slice: &mut [bool], i: usize) {
    if let Some(state) = slice.get_mut(i) {
        *state = !*state;
    }
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
            Message::AccentSelected(accent) => {
                color::set_accent(accent);
                self.selected_accent = accent;
            },
            Message::NavSelected(i) => {
                self.selected_nav = i;
            },
            Message::ToggleToolbar(i) => toggle_at(&mut self.toggle_states, i),
            Message::ToggleIcon(i) => toggle_at(&mut self.icon_states, i),
            Message::ToggleIconFlat(i) => toggle_at(&mut self.icon_flat_states, i),
            _ => {},
        }
    }

    fn view(&self) -> Element<'_, Message> {
        // The page is a list of categories. Each one is a titled, divided
        // block whose demos are built by a closure so `category` can render
        // them twice, once per compare-card background.
        let blocks = column![
            category("Theme", || vec![accent_picker(self.selected_accent)]),
            category("Typography", || vec![
                typography(),
                font_weights(),
                ellipsis_text(),
                links()
            ]),
            category("Icons", || vec![icons()]),
            category("Buttons", || vec![
                nav_buttons(),
                standard_buttons(),
                icon_buttons(self.icon_states),
                icon_flat_buttons(self.icon_flat_states),
                icon_carousel_buttons(),
                icon_overlay_buttons(),
                dismiss_buttons(),
                ghost_pills(),
                toggle_pills(self.toggle_states),
                hero_buttons(),
                clickables(),
            ]),
            category("Navigation", || vec![
                navigators(),
                tabs(self.selected_tab, self.selected_icon_tab),
                breadcrumbs(),
                bars(self.selected_nav),
            ]),
            category("Images & Media", || vec![
                posters(),
                episodes(),
                albums(),
                persons(),
                media_images(),
                clickable_card(),
            ]),
            category("Inputs", || vec![
                search_input(&self.search_content),
                inputs(&self.search_content),
            ]),
            category("Lists & Feedback", || vec![
                smart_list_demo(
                    self.smart_list_show,
                    self.smart_list_shown,
                    self.smart_list_hydrated,
                ),
                spinners(),
                skeletons(),
            ]),
            category("Surfaces", || vec![splash_backgrounds(), separators()]),
        ]
        .width(Length::Fill)
        .padding(padding::all(32))
        .spacing(40);

        let content = bluebottle_ui::scrollable::scrollable(blocks);
        let counter = container(bluebottle_ui::debug::fps_counter())
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Right)
            .align_y(Top)
            .padding(12);

        stack![content, counter].into()
    }
}

/// A titled, divided category. A prominent heading over a full-width divider,
/// then the demos shown twice side by side, once on the main background and
/// once on the glass gradient, so each can be read against both surfaces.
fn category<'a>(
    title: &'static str,
    demos: impl Fn() -> Vec<Element<'a, Message>>,
) -> Element<'a, Message> {
    let header = column![
        bluebottle_ui::text::heading_large(title),
        bluebottle_ui::separator::seperator(Length::Fill),
    ]
    .spacing(6);

    let cards = row![
        card(column(demos()).spacing(24), Background::Color(color::BG)),
        card(column(demos()).spacing(24), gradient_background()),
    ]
    .spacing(16);

    column![header, cards].spacing(20).into()
}

/// A single widget demo. A bold sub-title above the widget being shown.
fn section<'a>(
    title: &'static str,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    column![bluebottle_ui::text::section_heading(title), content.into()]
        .spacing(8)
        .into()
}

fn accent_picker(selected: color::Accent) -> Element<'static, Message> {
    let entries: [(color::Accent, &'static str, [Color; 4]); 4] = [
        (
            color::Accent::Default,
            "Default",
            [
                iced::color!(0x615FFF),
                iced::color!(0x00BC7D),
                iced::color!(0xFF2056),
                iced::color!(0xFE9A00),
            ],
        ),
        (
            color::Accent::Pastel,
            "Pastel",
            [
                iced::color!(0x7DD3FC),
                iced::color!(0x34D399),
                iced::color!(0xF472B6),
                iced::color!(0xFBBF24),
            ],
        ),
        (
            color::Accent::Electric,
            "Electric",
            [
                iced::color!(0xA78BFA),
                iced::color!(0x22D3EE),
                iced::color!(0xFB7185),
                iced::color!(0xFACC15),
            ],
        ),
        (
            color::Accent::Candy,
            "Candy",
            [
                iced::color!(0xF472B6),
                iced::color!(0x10B981),
                iced::color!(0x60A5FA),
                iced::color!(0xF59E0B),
            ],
        ),
    ];

    let swatch = |c: Color| -> Element<'static, Message> {
        container(text(""))
            .width(Length::Fixed(14.0))
            .height(Length::Fixed(14.0))
            .style(move |_| container::Style {
                background: Some(Background::Color(c)),
                border: Border::default().rounded(3),
                ..container::Style::default()
            })
            .into()
    };

    let mut items: Vec<Element<'static, Message>> = Vec::with_capacity(4);
    for (accent, label, swatches) in entries {
        let strip = row![
            swatch(swatches[0]),
            swatch(swatches[1]),
            swatch(swatches[2]),
            swatch(swatches[3]),
        ]
        .spacing(4);

        let inner = column![text(label).size(12), strip,]
            .spacing(6)
            .align_x(Center);

        let is_selected = accent == selected;
        let tint = if is_selected {
            color::primary_soft()
        } else {
            color::HOVER
        };

        items.push(
            clickable(inner)
                .padding([8, 12])
                .radius(8.0)
                .tint(tint)
                .on_press(Message::AccentSelected(accent))
                .into(),
        );
    }

    section("Accent Theme", row(items).spacing(8))
}

fn typography() -> Element<'static, Message> {
    use bluebottle_ui::text::{self, Variant};

    let scale = column![
        text::display_large("Display Large"),
        text::display_medium("Display Medium"),
        text::heading_large("Heading Large"),
        text::heading_medium("Heading Medium"),
        text::title_small("Title Small"),
        text::subtitle("Subtitle", Variant::Main),
        text::lead("Lead. Supporting copy beneath a title.", Variant::Main),
        text::body("Body. The quick brown fox jumps over the lazy dog."),
        text::section_heading("Section Heading"),
        text::card_title("Card Title"),
        text::label("Label", Variant::Main),
        text::caption("Caption"),
        text::eyebrow("EYEBROW", Variant::Main),
        text::micro_label("MICRO LABEL"),
    ]
    .spacing(6)
    .padding(padding::left(16));

    section("Type Scale", scale)
}

fn font_weights() -> Element<'static, Message> {
    let weights = column![
        text("The quick brown fox jumps over the lazy dog").font(font::regular()),
        text("The quick brown fox jumps over the lazy dog").font(font::semibold()),
        text("The quick brown fox jumps over the lazy dog").font(font::bold()),
    ]
    .spacing(4)
    .padding(padding::left(16));

    section("Font Weights", weights)
}

fn ellipsis_text() -> Element<'static, Message> {
    use bluebottle_ui::text;

    let demo = column![
        bluebottle_ui::ellipsis_text::ellipsis_text(text::body("Short enough to fit"))
            .width(220),
        bluebottle_ui::ellipsis_text::ellipsis_text(text::body(
            "The quick brown fox jumps over the lazy dog"
        ))
        .width(220),
    ]
    .spacing(6)
    .padding(padding::left(16));

    section("Text Ellipsis", demo)
}

fn links() -> Element<'static, Message> {
    let demo = row![
        bluebottle_ui::link(
            bluebottle_ui::text::body("Default"),
            Message::LinkPressed("default"),
        ),
        bluebottle_ui::link(
            bluebottle_ui::text::body("Large semibold")
                .size(16)
                .font(font::semibold()),
            Message::LinkPressed("large-semibold"),
        ),
        bluebottle_ui::link(
            bluebottle_ui::text::label(
                "Secondary tint",
                bluebottle_ui::text::Variant::Alt,
            ),
            Message::LinkPressed("secondary"),
        ),
        bluebottle_ui::link(
            bluebottle_ui::text::body("Inline within a row"),
            Message::LinkPressed("inline"),
        ),
    ]
    .padding(padding::left(16))
    .spacing(16);

    section("Links", demo)
}

fn icons() -> Element<'static, Message> {
    let demo = row![
        icon::outline("home").size(48),
        icon::filled("home").size(48),
        icon::outline("favorite_border").size(48),
        icon::filled("favorite").size(48),
    ]
    .spacing(4)
    .padding(padding::left(16));

    section("Icons", demo)
}

fn nav_buttons() -> Element<'static, Message> {
    let demo = row![
        column![
            bluebottle_ui::button::nav("Home", "home", false, Message::Click),
            bluebottle_ui::button::nav("Search", "search", false, Message::Click),
            bluebottle_ui::button::nav("Liked", "favorite", false, Message::Click),
            bluebottle_ui::button::nav("Anime", "draw", false, Message::Click),
            bluebottle_ui::button::nav("TV Shows", "tv", false, Message::Click),
            bluebottle_ui::button::nav("Movies", "movie", false, Message::Click),
            bluebottle_ui::button::nav("Anime Movies", "movie", false, Message::Click),
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
            bluebottle_ui::button::nav("Anime Movies", "movie", true, Message::Click),
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
            bluebottle_ui::button::nav("Anime Movies", "movie", false, Message::Click),
            bluebottle_ui::button::nav("Music", "library_music", false, Message::Click),
        ]
        .align_x(Center),
    ]
    .spacing(8);

    section("Nav Buttons", demo)
}

fn standard_buttons() -> Element<'static, Message> {
    let demo = row![
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
    .spacing(8);

    section("Standard Buttons", demo)
}

fn icon_buttons(states: [bool; 4]) -> Element<'static, Message> {
    use bluebottle_ui::button::{IconSizeVariant, icon};

    let demo = row![
        icon(
            "bookmark",
            IconSizeVariant::Main,
            states[0],
            Message::ToggleIcon(0),
        ),
        icon(
            "bookmark",
            IconSizeVariant::Main,
            states[1],
            Message::ToggleIcon(1),
        ),
        icon(
            "bookmark",
            IconSizeVariant::Alt,
            states[2],
            Message::ToggleIcon(2),
        ),
        icon(
            "bookmark",
            IconSizeVariant::Alt,
            states[3],
            Message::ToggleIcon(3),
        ),
    ]
    .padding(8)
    .spacing(8)
    .align_y(Center);

    section("Bordered Glass Icons", demo)
}

fn icon_flat_buttons(states: [bool; 2]) -> Element<'static, Message> {
    use bluebottle_ui::button::icon_flat;

    // Off, on, disabled-off, disabled-on. The two disabled slots keep the same
    // 38px footprint as their interactive neighbours so a row stays aligned.
    let demo = row![
        icon_flat("favorite", states[0], Some(Message::ToggleIconFlat(0))),
        icon_flat("favorite", states[1], Some(Message::ToggleIconFlat(1))),
        icon_flat("favorite", false, None),
        icon_flat("favorite", true, None),
    ]
    .padding(8)
    .spacing(8)
    .align_y(Center);

    section("Flat Round Icons", demo)
}

fn icon_carousel_buttons() -> Element<'static, Message> {
    use bluebottle_ui::button::icon_carousel;

    // Active chevrons both white; disabled chevrons dim to text-secondary.
    let demo = row![
        icon_carousel("chevron_left", Some(Message::Click)),
        icon_carousel("chevron_right", Some(Message::Click)),
        icon_carousel("chevron_left", None),
        icon_carousel("chevron_right", None),
    ]
    .padding(8)
    .spacing(8)
    .align_y(Center);

    section("Carousel Nav Icons", demo)
}

fn icon_overlay_buttons() -> Element<'static, Message> {
    use bluebottle_ui::button::icon_overlay;

    let demo = row![
        icon_overlay("cast", Message::Click),
        icon_overlay("more_horiz", Message::Click),
    ]
    .padding(8)
    .spacing(8)
    .align_y(Center);

    section("Overlay Pill Icons", demo)
}

fn dismiss_buttons() -> Element<'static, Message> {
    use bluebottle_ui::button::{dismiss, dismiss_icon};

    let demo = row![dismiss(Message::Click), dismiss_icon(Message::Click)]
        .padding(8)
        .spacing(8)
        .align_y(Center);

    section("Dismiss Buttons", demo)
}

fn hero_buttons() -> Element<'static, Message> {
    let demo = row![
        bluebottle_ui::button::hero("settings", "settings", Message::Click),
        bluebottle_ui::button::hero("play_arrow", "Resume · 1h 48m", Message::Click),
    ]
    .padding(8)
    .spacing(8);

    section("Hero Buttons", demo)
}

fn ghost_pills() -> Element<'static, Message> {
    let demo = column![
        row![
            bluebottle_ui::button::ghost("Add to list", None, Message::Click),
            bluebottle_ui::button::ghost("Share", Some("share"), Message::Click),
        ]
        .spacing(8),
        row![
            bluebottle_ui::button::ghost_small("Add to list", None, Message::Click),
            bluebottle_ui::button::ghost_small("Share", Some("share"), Message::Click),
        ]
        .spacing(8),
    ]
    .padding(8)
    .spacing(8);

    section("Ghost Pills", demo)
}

fn toggle_pills(states: [bool; 3]) -> Element<'static, Message> {
    let toolbar = row![
        bluebottle_ui::button::toggle_pill(
            "Subscribed",
            Some("subscriptions"),
            states[0],
            Message::ToggleToolbar(0)
        ),
        bluebottle_ui::button::toggle_pill(
            "Liked",
            Some("favorite"),
            states[1],
            Message::ToggleToolbar(1)
        ),
        bluebottle_ui::button::toggle_pill(
            "TV",
            Some("tv"),
            states[2],
            Message::ToggleToolbar(2)
        ),
    ]
    .spacing(8);

    let labels_only = row![
        bluebottle_ui::button::toggle_pill("Off", None, false, Message::Click),
        bluebottle_ui::button::toggle_pill("On", None, true, Message::Click),
    ]
    .spacing(8);

    let demo = column![toolbar, labels_only].spacing(8);

    section("Toggle Pills", demo)
}

/// The glass gradient surface. Top stop down to the bottom stop, matching
/// `linear-gradient(180deg, --glass-top, --glass-base)`.
fn gradient_background() -> Background {
    let gradient = iced::gradient::Linear::new(iced::Radians(std::f32::consts::PI))
        .add_stop(0.0, color::GLASS_TOP)
        .add_stop(1.0, color::GLASS_BASE);

    Background::Gradient(gradient.into())
}

/// A rounded surface card behind `content` with the given `background`. Fills
/// its share of the row so the two compare cards split evenly.
fn card<'a>(
    content: impl Into<Element<'a, Message>>,
    background: Background,
) -> Element<'a, Message> {
    container(content.into())
        .width(Length::Fill)
        .padding(20)
        .style(move |_theme| container::Style {
            background: Some(background),
            border: Border {
                radius: 16.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
}

fn clickables() -> Element<'static, Message> {
    use bluebottle_ui::clickable;

    let demo = row![
        Element::<Message>::from(clickable(text("Inert")).padding([6, 12])),
        Element::<Message>::from(
            clickable(text("Default"))
                .padding([6, 12])
                .on_press(Message::Click),
        ),
        Element::<Message>::from(
            clickable(text("Primary tint"))
                .padding([6, 12])
                .tint(color::primary())
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
    .align_y(Center);

    section("Clickables", demo)
}

fn navigators() -> Element<'static, Message> {
    let demo = row![
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
    .spacing(8);

    section("Carousel Navigators", demo)
}

fn tabs(selected: usize, selected_icon: usize) -> Element<'static, Message> {
    let text_tab = |label: &'static str| -> Element<'static, Message> {
        text(label).size(14).into()
    };

    let mut idx = 0;
    let mut icon_tab =
        |icon_name: &'static str, label: &'static str| -> Element<'static, Message> {
            let this_idx = idx;
            idx += 1;

            let color = (this_idx == selected_icon).then_some(color::primary());

            row![
                icon::filled(icon_name).size(20).color_maybe(color),
                text(label).size(14),
            ]
            .spacing(8)
            .align_y(Center)
            .into()
        };

    let demo = column![
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
    .spacing(12);

    section("Tabs", demo)
}

fn breadcrumbs() -> Element<'static, Message> {
    let demo = column![
        bluebottle_ui::breadcrumb::breadcrumb(&["Library"]),
        bluebottle_ui::breadcrumb::breadcrumb(&["Library", "Anime"]),
        bluebottle_ui::breadcrumb::breadcrumb(&[
            "Library",
            "Anime",
            "Dusk Beyond the End of the World",
        ]),
    ]
    .spacing(8);

    section("Breadcrumbs", demo)
}

fn bars(selected: usize) -> Element<'static, Message> {
    // Each entry owns an index. Clicking one selects it and deselects the rest,
    // so the example shows the on/off swap and leaves the others free to hover.
    let entry = |label, icon, index| {
        bluebottle_ui::button::nav(
            label,
            icon,
            selected == index,
            Message::NavSelected(index),
        )
    };

    let top_buttons = column![
        entry("Home", "home", 0),
        entry("Search", "search", 1),
        entry("Liked", "favorite", 2),
        entry("Anime", "draw", 3),
    ]
    .spacing(8.0)
    .align_x(Center);

    let bottom_buttons = column![
        entry("Library", "storage", 4),
        entry("Settings", "settings", 5),
    ]
    .spacing(8.0)
    .align_x(Center);

    let sidebar = bluebottle_ui::bar::side(top_buttons, bottom_buttons);

    section("Sidebar", container(sidebar).height(600))
}

fn posters() -> Element<'static, Message> {
    let content = POSTER.clone();

    let demo = column![
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
    .spacing(4);

    section("Image Posters", demo)
}

fn episodes() -> Element<'static, Message> {
    let content = THUMBNAIL.clone();

    let demo = row![
        bluebottle_ui::image::thumbnail(content),
        bluebottle_ui::image::thumbnail_skeleton(),
    ]
    .padding(8)
    .spacing(8);

    section("Image Episodes", demo)
}

fn albums() -> Element<'static, Message> {
    let content = SQUARE.clone();

    let demo = row![
        bluebottle_ui::image::square(content),
        bluebottle_ui::image::square_skeleton(),
    ]
    .padding(8)
    .spacing(8);

    section("Image Albums", demo)
}

fn persons() -> Element<'static, Message> {
    let content = PERSON_POSTER.clone();

    let demo = row![
        bluebottle_ui::image::person(content.clone(), PersonSize::Poster),
        bluebottle_ui::image::person_skeleton(PersonSize::Poster),
        bluebottle_ui::image::person(content, PersonSize::Square),
        bluebottle_ui::image::person_skeleton(PersonSize::Square),
    ]
    .padding(8)
    .spacing(8);

    section("Image Persons", demo)
}

fn media_images() -> Element<'static, Message> {
    let play_overlay = || {
        container(
            icon::filled("play_arrow")
                .color(color::TEXT_PRIMARY)
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

    let demo = row![inert, clickable, no_border, with_overlay,]
        .padding(8)
        .spacing(8);

    section("Media Images", demo)
}

fn clickable_card() -> Element<'static, Message> {
    let label_text = |s: &'static str| text(s).size(14).color(color::TEXT_PRIMARY);
    let subtext_text = |s: &'static str| text(s).size(12).color(color::TEXT_SECONDARY);

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
                .color(color::TEXT_PRIMARY)
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
            .label(bluebottle_ui::link(
                bluebottle_ui::text::body("Per-region Press"),
                Message::CardLabel,
            ))
            .subtext(bluebottle_ui::link(
                bluebottle_ui::text::label(
                    "Each row has its own message",
                    bluebottle_ui::text::Variant::Alt,
                ),
                Message::CardSubtext,
            ))
            .on_press(Message::Click);

    let demo = row![
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
    .spacing(8);

    section("Clickable Card", demo)
}

fn search_input(content: &str) -> Element<'_, Message> {
    section(
        "Search Input",
        bluebottle_ui::search::search("Sample input...", content, Message::SearchInput),
    )
}

fn inputs(content: &str) -> Element<'_, Message> {
    let demo = column![
        bluebottle_ui::input::text_input(
            "Sample input...",
            content,
            Message::SearchInput
        ),
        bluebottle_ui::input::text_input("Password...", content, Message::SearchInput)
            .secure(true),
    ]
    .spacing(8);

    section("Text Input", demo)
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
        let header = text(group_labels[gi]).size(16).font(font::semibold());

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

    let demo = column![
        controls,
        text(shown_text).size(12).color(color::TEXT_SECONDARY),
        container(list)
            .height(Length::Fixed(320.0))
            .width(Length::Fill)
            .padding(padding::left(16)),
    ]
    .spacing(8);

    section("Smart List", demo)
}

fn spinners() -> Element<'static, Message> {
    let demo = column![
        bluebottle_ui::spinner::linear(),
        bluebottle_ui::spinner::circle().size(40),
    ]
    .spacing(8);

    section("Spinners", demo)
}

fn skeletons() -> Element<'static, Message> {
    section(
        "Skeletons",
        bluebottle_ui::skeleton::skeleton().height(224).width(224),
    )
}

fn splash_backgrounds() -> Element<'static, Message> {
    let backdrop = SPLASH_BACKDROP.clone();

    let demo = row![
        container(splash_background(backdrop.clone()))
            .width(Length::FillPortion(1))
            .height(320),
        container(splash_panel(backdrop))
            .width(Length::FillPortion(1))
            .height(320),
    ]
    .spacing(8)
    .padding(padding::left(16));

    section("Splash Backgrounds", demo)
}

fn separators() -> Element<'static, Message> {
    section(
        "Separators",
        bluebottle_ui::separator::seperator(Length::Fixed(400.0)),
    )
}
