use std::sync::LazyLock;

use bluebottle_ui::image::{PersonSize, PosterSize};
use bluebottle_ui::splash_background::{Backdrop, splash_background, splash_panel};
use bluebottle_ui::{clickable, color, font, icon, tab};
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
    checkbox_states: [bool; 4],
    switch_states: [bool; 4],
    dropdown_open: bool,
    dropdown_choice: &'static str,
    source_choice: usize,
    reorder_order: Vec<usize>,
    season_choice: usize,
    labelled_choice: usize,
    filter_choices: Vec<bool>,
    slider_value: f32,
    stepped_slider_value: f32,
    volume_value: f32,
    search_dense: String,
    display_name: String,
    nickname: String,
    valid_handle: String,
    error_handle: String,
    password_value: String,
    password_revealed: bool,
    stepper_value: i32,
    stepper_compact_value: i32,
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
            checkbox_states: [true, false, true, false],
            switch_states: [true, false, true, false],
            dropdown_open: true,
            dropdown_choice: DROPDOWN_CHOICES[0],
            source_choice: 0,
            reorder_order: (0..REORDER_ITEMS.len()).collect(),
            season_choice: 0,
            labelled_choice: 0,
            filter_choices: {
                let mut v = vec![false; FILTER_TAGS.len()];
                if let Some(slot) = v.get_mut(0) {
                    *slot = true;
                }
                if let Some(slot) = v.get_mut(3) {
                    *slot = true;
                }
                v
            },
            slider_value: 0.62,
            stepped_slider_value: 0.5,
            volume_value: 0.42,
            search_dense: String::new(),
            display_name: "Avery".into(),
            nickname: "Birdie".into(),
            valid_handle: "@avery".into(),
            error_handle: "@taken".into(),
            password_value: String::new(),
            password_revealed: false,
            stepper_value: 50,
            stepper_compact_value: 5,
        }
    }
}

const DROPDOWN_CHOICES: &[&str] = &["Recent", "A to Z", "Year", "Rating"];
const REORDER_ITEMS: &[(&str, &str)] = &[
    ("Continue Watching", "play_arrow"),
    ("Recently Added", "fiber_new"),
    ("Top Picks", "auto_awesome"),
    ("Trending", "trending_up"),
    ("My Watchlist", "bookmark"),
];

const FILTER_TAGS: &[&str] = &[
    "Action",
    "Comedy",
    "Drama",
    "Sci-Fi",
    "Fantasy",
    "Romance",
    "Documentary",
    "Animation",
];

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
    ToggleCheckbox(usize),
    ToggleSwitch(usize),
    DropdownToggle,
    DropdownDismiss,
    DropdownPick(&'static str),
    SourcePick(usize),
    SourceManage,
    Reorder(usize, usize),
    SeasonPick(usize),
    LabelledPick(usize),
    FilterChoice(usize),
    FilterBulk(bool),
    SliderChanged(f32),
    SteppedSliderChanged(f32),
    VolumeChanged(f32),
    SliderReleased,
    SearchClear,
    SearchSubmit,
    SearchDenseInput(String),
    SearchDenseClear,
    DisplayNameInput(String),
    NicknameInput(String),
    ValidHandleInput(String),
    ErrorHandleInput(String),
    PasswordInput(String),
    PasswordToggle,
    StepperChanged(i32),
    StepperCompactChanged(i32),
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
            Message::ToggleCheckbox(i) => toggle_at(&mut self.checkbox_states, i),
            Message::ToggleSwitch(i) => toggle_at(&mut self.switch_states, i),
            Message::DropdownToggle => {
                self.dropdown_open = !self.dropdown_open;
                println!("dropdown toggle -> {}", self.dropdown_open);
            },
            Message::DropdownDismiss => {
                self.dropdown_open = false;
                println!("dropdown dismiss");
            },
            Message::DropdownPick(choice) => {
                self.dropdown_choice = choice;
                self.dropdown_open = false;
                println!("dropdown pick {choice}");
            },
            Message::SourcePick(choice) => {
                self.source_choice = choice;
            },
            Message::SourceManage => {
                println!("manage sources");
            },
            Message::Reorder(from, to) => {
                let entry = self.reorder_order.remove(from);
                self.reorder_order.insert(to, entry);
            },
            Message::SeasonPick(choice) => {
                self.season_choice = choice;
            },
            Message::LabelledPick(choice) => {
                self.labelled_choice = choice;
            },
            Message::FilterChoice(i) => toggle_at(&mut self.filter_choices, i),
            Message::FilterBulk(all) => {
                for choice in self.filter_choices.iter_mut() {
                    *choice = all;
                }
            },
            Message::SliderChanged(value) => {
                self.slider_value = value;
            },
            Message::SteppedSliderChanged(value) => {
                self.stepped_slider_value = value;
            },
            Message::VolumeChanged(value) => {
                self.volume_value = value;
            },
            Message::SliderReleased => {},
            Message::SearchClear => {
                self.search_content.clear();
            },
            Message::SearchSubmit => {},
            Message::SearchDenseInput(content) => {
                self.search_dense = content;
            },
            Message::SearchDenseClear => {
                self.search_dense.clear();
            },
            Message::DisplayNameInput(content) => {
                self.display_name = content;
            },
            Message::NicknameInput(content) => {
                self.nickname = content;
            },
            Message::ValidHandleInput(content) => {
                self.valid_handle = content;
            },
            Message::ErrorHandleInput(content) => {
                self.error_handle = content;
            },
            Message::PasswordInput(content) => {
                self.password_value = content;
            },
            Message::PasswordToggle => {
                self.password_revealed = !self.password_revealed;
            },
            Message::StepperChanged(value) => {
                self.stepper_value = value;
            },
            Message::StepperCompactChanged(value) => {
                self.stepper_compact_value = value;
            },
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
                icon_buttons(self.icon_states),
                icon_flat_buttons(self.icon_flat_states),
                icon_carousel_buttons(),
                icon_overlay_buttons(),
                dismiss_buttons(),
                media_primary_buttons(),
                media_transport_buttons(),
                media_accent_buttons(),
                ghost_pills(),
                toggle_pills(self.toggle_states),
                checkboxes(self.checkbox_states),
                switches(self.switch_states),
                switch_rows(self.switch_states),
                hero_buttons(),
            ]),
            category("Navigation", || vec![
                navigators(),
                tabs(self.selected_tab, self.selected_icon_tab),
                breadcrumbs(),
                bars(self.selected_nav),
            ]),
            category("Inputs", || {
                vec![
                    search_fields(&self.search_content, &self.search_dense),
                    text_fields(
                        &self.display_name,
                        &self.nickname,
                        &self.valid_handle,
                        &self.error_handle,
                    ),
                    password_fields(&self.password_value, self.password_revealed),
                    steppers(self.stepper_value, self.stepper_compact_value),
                    dropdown_demo(self.dropdown_open, self.dropdown_choice),
                    source_demo(self.source_choice),
                    season_demo(self.season_choice),
                    labelled_demo(self.labelled_choice),
                    filter_demo(&self.filter_choices),
                    sliders(
                        self.slider_value,
                        self.stepped_slider_value,
                        self.volume_value,
                    ),
                ]
            }),
            category("Images & Media", || vec![
                posters(),
                episodes(),
                albums(),
                persons(),
                media_images(),
                clickable_card(),
            ]),
            category("Lists & Feedback", || vec![
                smart_list_demo(
                    self.smart_list_show,
                    self.smart_list_shown,
                    self.smart_list_hydrated,
                ),
                reorderable_demo(&self.reorder_order),
                spinners(),
                skeletons(),
            ]),
            category("Surfaces", || vec![
                cards(),
                library_counts(),
                library_sources(),
                film_facts_demo(),
                splash_backgrounds(),
                separators(),
            ]),
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
        .spacing(8)
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
        .spacing(8)
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
        .spacing(8)
        .align_x(Center),
    ]
    .spacing(8);

    section("Nav Buttons", demo)
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

fn media_primary_buttons() -> Element<'static, Message> {
    use bluebottle_ui::button::{PrimarySizeVariant, primary};

    let demo = row![
        primary("play_arrow", PrimarySizeVariant::Small, Message::Click),
        primary("play_arrow", PrimarySizeVariant::Medium, Message::Click),
        primary("play_arrow", PrimarySizeVariant::Large, Message::Click),
    ]
    .padding(8)
    .spacing(16)
    .align_y(Center);

    section("Primary Play / Pause", demo)
}

fn media_transport_buttons() -> Element<'static, Message> {
    use bluebottle_ui::button::{mode, skip, transport_mini};

    let demo = row![
        skip("skip_previous", Message::Click),
        mode("shuffle", false, Message::Click),
        mode("repeat", true, Message::Click),
        skip("skip_next", Message::Click),
        transport_mini("skip_previous", Message::Click),
        transport_mini("skip_next", Message::Click),
    ]
    .padding(8)
    .spacing(8)
    .align_y(Center);

    section("Transport Skip / Mode", demo)
}

fn media_accent_buttons() -> Element<'static, Message> {
    use bluebottle_ui::button::{AccentSizeVariant, accent};

    let demo = row![
        accent("play_arrow", AccentSizeVariant::Main, Message::Click),
        accent("play_arrow", AccentSizeVariant::Alt, Message::Click),
    ]
    .padding(8)
    .spacing(8)
    .align_y(Center);

    section("Accent Hover-Reveal Play", demo)
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

fn checkboxes(states: [bool; 4]) -> Element<'static, Message> {
    use bluebottle_ui::button::CheckboxSizeVariant;

    let main = row![
        bluebottle_ui::button::checkbox(
            states[0],
            CheckboxSizeVariant::Main,
            Some(Message::ToggleCheckbox(0)),
        ),
        bluebottle_ui::button::checkbox(
            states[1],
            CheckboxSizeVariant::Main,
            Some(Message::ToggleCheckbox(1)),
        ),
    ]
    .spacing(8)
    .align_y(Center);

    let alt = row![
        bluebottle_ui::button::checkbox(
            states[2],
            CheckboxSizeVariant::Alt,
            Some(Message::ToggleCheckbox(2)),
        ),
        bluebottle_ui::button::checkbox(
            states[3],
            CheckboxSizeVariant::Alt,
            Some(Message::ToggleCheckbox(3)),
        ),
    ]
    .spacing(8)
    .align_y(Center);

    let demo = column![main, alt].spacing(8);

    section("Checkboxes", demo)
}

fn switches(states: [bool; 4]) -> Element<'static, Message> {
    use bluebottle_ui::button::SwitchSizeVariant;

    let main = row![
        bluebottle_ui::button::switch(
            states[0],
            SwitchSizeVariant::Main,
            Some(Message::ToggleSwitch(0)),
        ),
        bluebottle_ui::button::switch(
            states[1],
            SwitchSizeVariant::Main,
            Some(Message::ToggleSwitch(1)),
        ),
    ]
    .spacing(8)
    .align_y(Center);

    let alt = row![
        bluebottle_ui::button::switch(
            states[2],
            SwitchSizeVariant::Alt,
            Some(Message::ToggleSwitch(2)),
        ),
        bluebottle_ui::button::switch(
            states[3],
            SwitchSizeVariant::Alt,
            Some(Message::ToggleSwitch(3)),
        ),
    ]
    .spacing(8)
    .align_y(Center);

    let disabled = row![
        bluebottle_ui::button::switch(false, SwitchSizeVariant::Main, None),
        bluebottle_ui::button::switch(true, SwitchSizeVariant::Main, None),
        bluebottle_ui::button::switch(false, SwitchSizeVariant::Alt, None),
        bluebottle_ui::button::switch(true, SwitchSizeVariant::Alt, None),
    ]
    .spacing(8)
    .align_y(Center);

    let demo = column![main, alt, disabled].spacing(8);

    section("Switches", demo)
}

fn switch_rows(states: [bool; 4]) -> Element<'static, Message> {
    let live = column![
        bluebottle_ui::button::switch_row(
            "Autoplay next episode",
            Some("Start the next episode automatically when one ends."),
            states[0],
            Some(Message::ToggleSwitch(0)),
        ),
        bluebottle_ui::button::switch_row(
            "Use system theme",
            None,
            states[1],
            Some(Message::ToggleSwitch(1)),
        ),
        bluebottle_ui::button::switch_row(
            "Subtitles by default",
            Some("Turn on captions whenever a track is available."),
            states[2],
            Some(Message::ToggleSwitch(2)),
        ),
    ]
    .spacing(2);

    // Disabled rows render fixed on/off states so the demo's "Sign in to
    // unlock" copy is not contradicted by the live Switches section toggling
    // the same state slot.
    let disabled = column![
        bluebottle_ui::button::switch_row(
            "Background downloads",
            Some("Sign in to unlock this setting."),
            false,
            None,
        ),
        bluebottle_ui::button::switch_row(
            "Crash reports",
            Some("Your org has locked this setting on."),
            true,
            None,
        ),
    ]
    .spacing(2);

    let demo = column![live, disabled].spacing(8);

    section("Toggle Rows", demo)
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
    let bar_fill = color::with_alpha(color::GLASS_BASE, color::srgb_alpha(0.92));
    let on_glass = move |bar: Element<'static, Message>| {
        container(bar).style(move |_theme| container::Style {
            background: Some(Background::Color(bar_fill)),
            ..container::Style::default()
        })
    };

    let demo = column![
        on_glass(
            bluebottle_ui::tabs(
                [
                    tab("info", "Overview"),
                    tab("playlist_play", "Episodes"),
                    tab("rate_review", "Reviews"),
                ],
                selected,
                Message::TabSelected,
            )
            .into(),
        ),
        on_glass(
            bluebottle_ui::tabs(
                [
                    tab("home", "Home"),
                    tab("movie", "Movies"),
                    tab("tv", "Shows"),
                    tab("music_note", "Music"),
                ],
                selected_icon,
                Message::IconTabSelected,
            )
            .into(),
        ),
    ]
    .spacing(12);

    section("Tabs", demo)
}

fn dropdown_demo(open: bool, chosen: &'static str) -> Element<'static, Message> {
    let label = text(chosen).size(13);

    // The clickables shrink to their text width on purpose. A `Length::Fill`
    // width inside the `Length::Shrink` column collapses to zero, which is what
    // produced the thin-line menu before. The column itself sizes to the widest
    // item, so the rows still align.
    let menu = DROPDOWN_CHOICES
        .iter()
        .fold(column![].spacing(2), |col, choice| {
            col.push(
                clickable(text(*choice).size(13))
                    .on_press(Message::DropdownPick(choice))
                    .padding(padding::all(6).left(10).right(10))
                    .radius(6.0),
            )
        });

    let widget = bluebottle_ui::dropdown::dropdown(label, menu, open)
        .on_toggle(|opening| {
            if opening {
                Message::DropdownToggle
            } else {
                Message::DropdownDismiss
            }
        })
        .background(color::SECONDARY)
        .border(color::border())
        .selected_background(color::HOVER)
        .selected_border(color::border_strong())
        .radius(10.0)
        .padding(padding::all(8).left(12).right(12));

    section(
        "Dropdown (chassis)",
        container(widget).height(240).width(Length::Fill),
    )
}

fn source_demo(selected: usize) -> Element<'static, Message> {
    use bluebottle_ui::dropdown::source::{
        Resolution,
        SourceStatus,
        SourceTag,
        source,
        source_entry,
    };

    let entries = vec![
        source_entry(
            "Local Library",
            SourceStatus::Online,
            "192.168.1.10",
            142,
            [SourceTag::Local, SourceTag::Recommended],
            Resolution::UHD4KHDR,
        ),
        source_entry(
            "Bedroom Plex",
            SourceStatus::Online,
            "plex.lan",
            812,
            [SourceTag::Recommended],
            Resolution::FullHD,
        ),
        source_entry(
            "Office Cast",
            SourceStatus::Online,
            "office.local",
            38,
            [SourceTag::Cast],
            Resolution::HD,
        ),
        source_entry(
            "Living Room Jellyfin",
            SourceStatus::Downloaded,
            "10.0.0.42",
            204,
            [SourceTag::Recommended, SourceTag::Cast],
            Resolution::UHD4K,
        ),
        source_entry(
            "Studio NAS",
            SourceStatus::Downloaded,
            "nas.studio",
            1024,
            [SourceTag::Local],
            Resolution::Other("ProRes".into()),
        ),
        source_entry(
            "Cabin Backup",
            SourceStatus::Downloaded,
            "cabin.tail-9b7.ts.net",
            60,
            [],
            Resolution::SD,
        ),
    ];

    let widget = source(entries, selected, Message::SourcePick)
        .footer_action("Manage Library", Message::SourceManage);

    section("Source", container(widget).height(360).width(Length::Fill))
}

fn seasons() -> Vec<bluebottle_ui::dropdown::season::SeasonInfo> {
    use bluebottle_ui::dropdown::season::season_info;

    vec![
        season_info("Season 1", "Pilot run", 2021, 10),
        season_info("Season 2", "Expanded ensemble", 2022, 12),
        season_info("Season 3", "World tour", 2023, 12),
        season_info("Season 4", "Origin arc", 2024, 11),
        season_info("Season 5", "Festival run", 2025, 13),
        season_info("Specials", "Holiday and side stories", 2026, 4),
    ]
}

fn season_demo(selected: usize) -> Element<'static, Message> {
    let widget = bluebottle_ui::dropdown::season::season(
        seasons(),
        selected,
        Message::SeasonPick,
    );

    section("Season", container(widget).height(240).width(Length::Fill))
}

fn labelled_demo(selected: usize) -> Element<'static, Message> {
    use bluebottle_ui::dropdown::labelled::item_row;

    let items = [
        item_row("Recent").count("128"),
        item_row("A to Z").count("96"),
        item_row("Year").count("42"),
        item_row("Rating").count("18"),
    ];

    let widget = bluebottle_ui::dropdown::labelled::labelled(
        "Sort:",
        Some("filter_list"),
        items,
        selected,
        Message::LabelledPick,
    );

    section(
        "Labelled",
        container(widget).height(240).width(Length::Fill),
    )
}

fn filter_demo(choices: &[bool]) -> Element<'static, Message> {
    let widget = bluebottle_ui::dropdown::filter::filter(
        "Genres",
        FILTER_TAGS.iter().copied(),
        choices.iter().copied(),
        Message::FilterChoice,
        Message::FilterBulk,
    );

    section("Filter", container(widget).height(360).width(Length::Fill))
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

fn reorderable_demo(order: &[usize]) -> Element<'static, Message> {
    use bluebottle_ui::reorderable::{grab_handle, reorderable};

    let row =
        |label: &'static str, icon_name: &'static str| -> Element<'static, Message> {
            let glyph = icon::filled(icon_name)
                .size(18)
                .color(color::TEXT_SECONDARY);
            let title =
                bluebottle_ui::text::label(label, bluebottle_ui::text::Variant::Main)
                    .font(font::semibold());

            let handle = grab_handle(
                icon::filled("drag_indicator")
                    .size(20)
                    .color(color::TEXT_SECONDARY),
            );

            let body = row![glyph, title].spacing(10).align_y(Center);

            let inner = row![body, container(handle).width(Length::Fill).align_x(Right)]
                .spacing(12)
                .align_y(Center);

            bluebottle_ui::card::card(inner)
                .padding(padding::all(12))
                .width(Length::Fill)
                .into()
        };

    let children: Vec<Element<'static, Message>> = order
        .iter()
        .copied()
        .map(|i| {
            let (label, icon_name) = REORDER_ITEMS[i];
            row(label, icon_name)
        })
        .collect();

    let demo = reorderable(children, Message::Reorder).spacing(8.0);

    section("Reorderable List", demo)
}

fn spinners() -> Element<'static, Message> {
    use bluebottle_ui::spinner::{
        DotPulseSize,
        DotRingSize,
        Tone,
        dot_pulse,
        dot_ring,
        progress_bar,
    };

    let rings = row![
        dot_ring().diameter(DotRingSize::Small),
        dot_ring().diameter(DotRingSize::Medium),
        dot_ring().diameter(DotRingSize::Large),
        dot_ring().diameter(DotRingSize::Large).tone(Tone::White),
    ]
    .spacing(16)
    .align_y(Center);

    let pulses = row![
        dot_pulse(),
        dot_pulse().diameter(DotPulseSize::Small),
        dot_pulse().tone(Tone::White),
    ]
    .spacing(20)
    .align_y(Center);

    let bars = column![
        progress_bar().width(320),
        progress_bar().value(0.62).width(320),
        progress_bar().value(0.18).width(320),
        progress_bar().value(0.45).tone(Tone::White).width(320),
    ]
    .spacing(12);

    let demo = column![rings, pulses, bars].spacing(20);

    section("Loaders", demo)
}

fn search_fields<'a>(value: &'a str, dense: &'a str) -> Element<'a, Message> {
    use bluebottle_ui::input::{SearchFieldSize, search_field};

    let standard = search_field(value)
        .placeholder("Search films, episodes, people")
        .on_input(Message::SearchInput)
        .on_clear(Message::SearchClear)
        .on_submit(Message::SearchSubmit)
        .width(360);

    let dense_field = search_field(dense)
        .size(SearchFieldSize::Dense)
        .placeholder("Find in library")
        .on_input(Message::SearchDenseInput)
        .on_clear(Message::SearchDenseClear)
        .width(280);

    section("Search", column![standard, dense_field].spacing(12))
}

fn text_fields<'a>(
    display_name: &'a str,
    nickname: &'a str,
    valid_handle: &'a str,
    error_handle: &'a str,
) -> Element<'a, Message> {
    use bluebottle_ui::input::text_field;

    let neutral = text_field("Display name", display_name)
        .placeholder("Type a name")
        .help("Shown next to your activity.")
        .on_input(Message::DisplayNameInput)
        .width(360);

    let optional = text_field("Nickname", nickname)
        .optional(true)
        .placeholder("A friendly short name")
        .help("Will appear under your handle.")
        .on_input(Message::NicknameInput)
        .width(360);

    let valid = text_field("Handle", valid_handle)
        .valid(true)
        .placeholder("@yourname")
        .help("Looks good.")
        .on_input(Message::ValidHandleInput)
        .width(360);

    let error = text_field("Handle", error_handle)
        .error("That name is taken.")
        .placeholder("@yourname")
        .on_input(Message::ErrorHandleInput)
        .width(360);

    let disabled = text_field("Handle", "fixed")
        .disabled(true)
        .help("Locked while syncing.")
        .width(360);

    section(
        "Text Fields",
        column![neutral, optional, valid, error, disabled].spacing(16),
    )
}

fn password_fields(value: &str, revealed: bool) -> Element<'_, Message> {
    use bluebottle_ui::input::password_field;

    let field = password_field("Password", value, revealed)
        .placeholder("Choose one")
        .help("At least eight characters.")
        .on_input(Message::PasswordInput)
        .on_toggle_reveal(Message::PasswordToggle)
        .width(360);

    section("Password", field)
}

fn steppers(value: i32, compact: i32) -> Element<'static, Message> {
    use bluebottle_ui::input::{StepperSize, stepper};

    let standard = stepper(value, Message::StepperChanged)
        .min(0)
        .max(100)
        .step(5)
        .suffix("%");

    let compact_stepper = stepper(compact, Message::StepperCompactChanged)
        .size(StepperSize::Compact)
        .min(0)
        .max(10)
        .step(1);

    section(
        "Steppers",
        row![standard, compact_stepper].spacing(16).align_y(Center),
    )
}

fn sliders(continuous: f32, stepped: f32, volume: f32) -> Element<'static, Message> {
    use bluebottle_ui::input::slider;

    let bare = slider(continuous, Message::SliderChanged)
        .on_release(Message::SliderReleased)
        .width(320);

    let stepped_slider = slider(stepped, Message::SteppedSliderChanged)
        .step(0.1)
        .width(320);

    let labelled = slider(volume, Message::VolumeChanged)
        .lead_icon("volume_up", |v| format!("{:.0}%", v * 100.0))
        .width(360);

    let demo = column![bare, stepped_slider, labelled].spacing(20);

    section("Sliders", demo)
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

fn library_counts() -> Element<'static, Message> {
    let demo = row![
        bluebottle_ui::card::library_count("Movies", color::primary(), "movie", 1_284,),
        bluebottle_ui::card::library_count("TV Shows", color::success(), "tv", 327,),
        bluebottle_ui::card::library_count(
            "Music",
            color::warning(),
            "library_music",
            12_503,
        ),
    ]
    .spacing(12);

    section("Library Counts", demo)
}

fn library_sources() -> Element<'static, Message> {
    use bluebottle_ui::card::{
        LibrarySourceKind,
        LibrarySourceStatus,
        library_source,
        library_source_count,
    };

    let online_remote = library_source(
        "Living Room NAS",
        "192.168.1.42:/media",
        LibrarySourceKind::Remote,
        LibrarySourceStatus::Online,
        [
            library_source_count("Movies", 1_284),
            library_source_count("TV Shows", 327),
        ],
        Message::Click,
    );

    let offline_remote = library_source(
        "Studio Archive",
        "nas.studio.lan:/vault",
        LibrarySourceKind::Remote,
        LibrarySourceStatus::Offline,
        [library_source_count("Movies", 482)],
        Message::Click,
    );

    let local = library_source(
        "Backup Drive",
        "/mnt/backup",
        LibrarySourceKind::Local,
        LibrarySourceStatus::Online,
        [library_source_count("Music", 12_503)],
        Message::Click,
    );

    let demo = column![online_remote, offline_remote, local].spacing(12);

    section("Library Sources", demo)
}

fn film_facts_demo() -> Element<'static, Message> {
    let entries = [
        bluebottle_ui::card::fact("Director", "Christopher Nolan"),
        bluebottle_ui::card::fact("Studio", "Warner Bros."),
        bluebottle_ui::card::fact("Released", "Jul 16, 2010"),
        bluebottle_ui::card::fact("Runtime", "2h 28m"),
        bluebottle_ui::card::fact("Language", "English"),
        bluebottle_ui::card::fact("Rating", "PG-13"),
    ];

    section("Fact Grid", bluebottle_ui::card::fact_grid(3, entries))
}

fn cards() -> Element<'static, Message> {
    let body = |label: &'static str| {
        container(bluebottle_ui::text::label(
            label,
            bluebottle_ui::text::Variant::Main,
        ))
        .padding(20)
    };

    let neutral = bluebottle_ui::card::card(body("Bordered glass"));

    let accent = bluebottle_ui::card::card(body("Accent highlight"))
        .background(color::primary_glass())
        .border(color::primary());

    let custom_padding =
        bluebottle_ui::card::card(body("Roomy padding")).padding(padding::all(24));

    let demo = row![neutral, accent, custom_padding]
        .spacing(12)
        .align_y(Center);

    section("Cards", demo)
}
