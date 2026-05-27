use std::sync::Arc;
use std::time::{Duration, Instant};

use bluebottle_ui::{button, color, easing, image, style};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{
    Space,
    column,
    container,
    float,
    mouse_area,
    row,
    scrollable,
    stack,
    text,
};
use iced::{Element, Length, Subscription, Vector};

use crate::backdrop;
use crate::background::{BackgroundLook, BackgroundSource, background};
use crate::sidebar::sidebar;

/// How long the sidebar's slide in / out runs.
const FADE: Duration = Duration::from_millis(220);

/// The drawer's width, in logical pixels.
const SIDEBAR_WIDTH: f32 = 850.0;

/// Which embedded poster the sidebar shows and blurs for its background.
const SIDEBAR_POSTER: usize = 0;

/// Placeholder posters embedded for the example home-screen content, so the
/// screen has real widgets to render.
const POSTER_BYTES: [&[u8]; 10] = [
    include_bytes!("../../../data/data/images/poster1.jpg"),
    include_bytes!("../../../data/data/images/poster2.jpg"),
    include_bytes!("../../../data/data/images/poster3.png"),
    include_bytes!("../../../data/data/images/poster4.jpg"),
    include_bytes!("../../../data/data/images/poster5.jpg"),
    include_bytes!("../../../data/data/images/poster6.jpg"),
    include_bytes!("../../../data/data/images/poster7.jpg"),
    include_bytes!("../../../data/data/images/poster8.jpg"),
    include_bytes!("../../../data/data/images/poster9.jpg"),
    include_bytes!("../../../data/data/images/poster10.jpg"),
];

/// Messages emitted by the main screen.
#[derive(Debug, Clone)]
pub enum MainMsg {
    /// Slide the sidebar in.
    OpenSidebar,
    /// Slide the sidebar out (X button or click-away).
    CloseSidebar,
    /// Animation frame while the sidebar slides.
    Tick,
    /// Swallows presses on the drawer so they don't dismiss it.
    Ignore,
}

/// State for the main library surface.
pub struct MainScreen {
    source: Arc<BackgroundSource>,
    background_look: BackgroundLook,
    sidebar: Option<SidebarState>,
    /// The poster blurred behind the drawer (the same one it shows).
    sidebar_source: Arc<BackgroundSource>,
    /// Decoded handles for the example poster content.
    posters: Vec<image::Handle>,
}

/// Live state of an open (or animating) sidebar.
struct SidebarState {
    phase: Phase,
    /// Start of the current phase's animation.
    started: Instant,
    /// Factor the close eases down from (the live factor when closing began).
    from: f32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Opening,
    Open,
    Closing,
}

impl SidebarState {
    /// Animation progress of the current phase, clamped to `[0, 1]`.
    fn raw(&self) -> f32 {
        (self.started.elapsed().as_secs_f32() / FADE.as_secs_f32()).clamp(0.0, 1.0)
    }

    /// The slide factor in `[0, 1]` (0 = off-screen, 1 = docked).
    fn factor(&self) -> f32 {
        match self.phase {
            Phase::Opening => easing::EMPHASIZED_DECELERATE.y_at_x(self.raw()),
            Phase::Open => 1.0,
            Phase::Closing => {
                self.from * (1.0 - easing::EMPHASIZED_ACCELERATE.y_at_x(self.raw()))
            },
        }
    }

    fn animating(&self) -> bool {
        matches!(self.phase, Phase::Opening | Phase::Closing)
    }
}

impl MainScreen {
    /// Builds the screen over `source`, with the default look.
    pub fn new(source: Arc<BackgroundSource>) -> Self {
        // The drawer blurs the poster it shows; decode it up front so the
        // pipeline keys on a stable `Arc` rather than re-uploading each frame.
        // If it fails to decode, fall back to a solid tinted fill.
        let sidebar_source =
            Arc::new(match backdrop::decode_bytes(POSTER_BYTES[SIDEBAR_POSTER]) {
                Some(image) => BackgroundSource::Image(Arc::new(image)),
                None => BackgroundSource::Solid,
            });

        Self {
            source,
            background_look: BackgroundLook::default(),
            sidebar: None,
            sidebar_source,
            posters: POSTER_BYTES
                .iter()
                .map(|bytes| image::Handle::from_bytes(*bytes))
                .collect(),
        }
    }

    pub fn update(&mut self, message: MainMsg) {
        match message {
            MainMsg::OpenSidebar => {
                if self.sidebar.is_none() {
                    self.sidebar = Some(SidebarState {
                        phase: Phase::Opening,
                        started: Instant::now(),
                        from: 0.0,
                    });
                }
            },
            MainMsg::CloseSidebar => {
                if let Some(state) = &mut self.sidebar
                    && state.phase != Phase::Closing
                {
                    // Ease down from the live factor so a mid-open close doesn't snap.
                    state.from = state.factor();
                    state.phase = Phase::Closing;
                    state.started = Instant::now();
                }
            },
            MainMsg::Tick => {
                if let Some(state) = &mut self.sidebar
                    && state.raw() >= 1.0
                {
                    match state.phase {
                        Phase::Opening => state.phase = Phase::Open,
                        Phase::Closing => self.sidebar = None,
                        Phase::Open => {},
                    }
                }
            },
            MainMsg::Ignore => {},
        }
    }

    pub fn subscription(&self) -> Subscription<MainMsg> {
        match &self.sidebar {
            Some(state) if state.animating() => {
                iced::time::every(Duration::from_millis(16)).map(|_| MainMsg::Tick)
            },
            _ => Subscription::none(),
        }
    }

    pub fn view(&self) -> Element<'_, MainMsg> {
        let backdrop = background(Arc::clone(&self.source), self.background_look);

        // The sidebar trigger, parked in the bottom-right corner.
        let trigger = container(button::standard(
            "Details",
            None,
            false,
            MainMsg::OpenSidebar,
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Horizontal::Right)
        .align_y(Vertical::Bottom)
        .padding(24);

        match &self.sidebar {
            None => stack![backdrop, self.content(), trigger].into(),
            Some(state) => {
                let factor = state.factor();
                // The scrim wash over the uncovered screen, faded in by the slide;
                // clicking it dismisses the sidebar.
                let veil = mouse_area(
                    container(Space::new().width(Length::Fill).height(Length::Fill))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .style(move |_theme| container::Style {
                            background: Some(
                                color::with_alpha(color::SCRIM, color::SCRIM.a * factor)
                                    .into(),
                            ),
                            ..container::Style::default()
                        }),
                )
                .on_press(MainMsg::CloseSidebar);

                stack![backdrop, self.content(), trigger, veil, self.drawer(factor)]
                    .into()
            },
        }
    }

    /// Example home-screen content: titled rows of posters, so there is real UI
    /// over the background to see.
    fn content(&self) -> Element<'_, MainMsg> {
        let section = |heading: &str, handles: &[image::Handle]| {
            let mut posters = row![].spacing(16);
            for handle in handles {
                posters = posters
                    .push(image::poster(handle.clone(), image::PosterSize::Medium));
            }
            column![
                text(heading.to_string())
                    .size(24)
                    .color(color::TEXT_DEFAULT),
                posters,
            ]
            .spacing(12)
        };

        let body = column![
            text("Bluebottle").size(40).color(color::TEXT_DEFAULT),
            section("Trending Now", &self.posters[0..5]),
            section("Recently Added", &self.posters[5..10]),
        ]
        .spacing(32)
        .padding(48);

        scrollable(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// The sliding drawer: the frosted sidebar background under its content,
    /// right-docked and translated off-screen by `(1 - factor)` of its width.
    fn drawer(&self, factor: f32) -> Element<'_, MainMsg> {
        let header = row![
            text("Details")
                .size(20)
                .color(color::TEXT_DEFAULT)
                .width(Length::Fill),
            button::standard("✕", None, false, MainMsg::CloseSidebar),
        ]
        .align_y(Vertical::Center);

        let body = column![
            header,
            image::poster(
                self.posters[SIDEBAR_POSTER].clone(),
                image::PosterSize::Large
            ),
        ]
        .spacing(20)
        .padding(24)
        .width(Length::Fill)
        .height(Length::Fill);

        // A faint accent line down the leading edge, riding along inside the
        // translated drawer.
        let border = container(
            Space::new()
                .width(Length::Fixed(style::BORDER_WIDTH))
                .height(Length::Fill),
        )
        .style(|_theme| container::Style {
            background: Some(color::BORDER.into()),
            ..container::Style::default()
        });

        // The shader paints the opaque background; the container adds the
        // leading-edge elevation shadow (a quad is still drawn for a shadow with
        // no fill).
        let drawer = container(stack![
            sidebar(Arc::clone(&self.sidebar_source), sidebar_look()),
            body,
            border
        ])
        .width(Length::Fixed(SIDEBAR_WIDTH))
        .height(Length::Fill)
        .clip(true)
        .style(|_theme| container::Style {
            shadow: style::PANEL_SHADOW,
            ..container::Style::default()
        });

        // Swallow presses on the drawer so only the veil dismisses it.
        let drawer = mouse_area(drawer).on_press(MainMsg::Ignore);

        // Right-dock the drawer, then slide it off the right edge by the inverse
        // of the factor; `float` translates without disturbing layout.
        container(float(drawer).translate(move |bounds, _viewport| {
            Vector::new((1.0 - factor) * bounds.width, 0.0)
        }))
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Horizontal::Right)
        .into()
    }
}

/// The drawer's background look: the main background's style over the elevated
/// surface tint, settling into it by mid-height.
fn sidebar_look() -> BackgroundLook {
    BackgroundLook {
        base: color::SURFACE,
        image_fade: 0.45,
        bg_end: 0.5,
        bg_solid: 0.5,
        focus: 0.5,
        ..BackgroundLook::default()
    }
}
