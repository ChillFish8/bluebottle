use std::sync::Arc;
use std::time::{Duration, Instant};

use bluebottle_ui::{button, color, easing, image};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{
    center,
    column,
    container,
    mouse_area,
    row,
    scrollable,
    slider,
    stack,
    text,
};
use iced::{Border, Color, Element, Length, Subscription, Task, border};

use crate::background::{BackgroundLook, BackgroundSource, background};
use crate::inspect::{SnapshotImage, scrim};

/// How long the inspect modal's fade in / out runs.
const FADE: Duration = Duration::from_millis(220);

/// Corner radius of the modal panel; shared by the scrim's frosted pane and the
/// container's border so they line up.
const PANEL_RADIUS: f32 = 16.0;

/// Placeholder posters embedded for the example home-screen content, so the
/// screen has real widgets to render (and for the inspect blur to capture).
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
    /// Open the inspect modal (captures a scene snapshot first).
    OpenInspect,
    /// The scene snapshot finished capturing; reveal the modal.
    SnapshotReady(iced::window::Screenshot),
    /// Dismiss the inspect modal (X button or click-away).
    CloseInspect,
    /// Animation frame while the modal fades.
    Tick,
    /// Swallows presses on the panel so they don't dismiss the modal.
    Ignore,
    /// Tuning sliders for the modal look.
    SetBlur(f32),
    SetPanelBlur(f32),
    SetSaturate(f32),
    SetTint(f32),
    SetPanelOpacity(f32),
    SetPanelShift(f32),
}

/// State for the main library surface.
pub struct MainScreen {
    source: Arc<BackgroundSource>,
    background_look: BackgroundLook,
    inspect: Option<Inspect>,
    /// A scene snapshot is being captured; suppresses duplicate open requests.
    capturing: bool,
    inspect_look: InspectLook,
    /// Decoded handles for the example poster content.
    posters: Vec<image::Handle>,
}

/// Tunable look of the inspect modal, dialled in by the panel sliders.
#[derive(Debug, Clone, Copy)]
struct InspectLook {
    /// Scrim blur radius, in snapshot pixels.
    blur: f32,
    /// Extra blur radius for the panel, compounded over the scrim blur.
    panel_blur: f32,
    /// Saturation multiplier for the blurred scene (1 = unchanged).
    saturate: f32,
    /// Scrim tint coverage in `[0, 1]`.
    tint: f32,
    /// Panel background opacity over the blur, in `[0, 1]`.
    panel_opacity: f32,
    /// Panel background tone shift from the app background toward primary, `[0, 1]`.
    panel_shift: f32,
}

impl Default for InspectLook {
    fn default() -> Self {
        Self {
            blur: 60.0,
            panel_blur: 75.0,
            saturate: 1.4,
            tint: 0.75,
            panel_opacity: 0.95,
            panel_shift: 0.05,
        }
    }
}

/// Live state of an open (or animating) inspect modal.
struct Inspect {
    phase: Phase,
    /// Start of the current phase's animation.
    started: Instant,
    /// Factor the close eases down from (the live factor when closing began).
    from: f32,
    /// Panel size in logical pixels, sized to the window when the modal opened.
    size: (f32, f32),
    /// The blurred scene behind the scrim.
    snapshot: Arc<SnapshotImage>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Opening,
    Open,
    Closing,
}

impl Inspect {
    /// Animation progress of the current phase, clamped to `[0, 1]`.
    fn raw(&self) -> f32 {
        (self.started.elapsed().as_secs_f32() / FADE.as_secs_f32()).clamp(0.0, 1.0)
    }

    /// The scrim/panel fade factor in `[0, 1]`.
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
        Self {
            source,
            background_look: BackgroundLook::default(),
            inspect: None,
            capturing: false,
            inspect_look: InspectLook::default(),
            posters: POSTER_BYTES
                .iter()
                .map(|bytes| image::Handle::from_bytes(*bytes))
                .collect(),
        }
    }

    pub fn update(&mut self, message: MainMsg) -> Task<MainMsg> {
        match message {
            MainMsg::OpenInspect => {
                // Guard on `capturing` too: the screenshot is in flight while
                // `inspect` is still None, so without it a second click would
                // dispatch a duplicate capture and restart the fade.
                if self.inspect.is_none() && !self.capturing {
                    self.capturing = true;
                    // Capture the current (modal-free) frame, then reveal.
                    return iced::window::latest()
                        .and_then(iced::window::screenshot)
                        .map(MainMsg::SnapshotReady);
                }
            },
            MainMsg::SnapshotReady(screenshot) => {
                self.capturing = false;
                // Size the panel to the window: 0.60 width (min 700px) and 3/4
                // height (clamped 500–1400px). The screenshot is physical; divide
                // by its scale for logical (layout) pixels.
                let scale = screenshot.scale_factor.max(f32::EPSILON);
                let window_width = screenshot.size.width as f32 / scale;
                let window_height = screenshot.size.height as f32 / scale;
                self.inspect = Some(Inspect {
                    phase: Phase::Opening,
                    started: Instant::now(),
                    from: 0.0,
                    size: (
                        (window_width * 0.60).max(700.0),
                        (window_height * 0.75).clamp(500.0, 1400.0),
                    ),
                    snapshot: Arc::new(SnapshotImage::from_screenshot(&screenshot)),
                });
            },
            MainMsg::CloseInspect => {
                if let Some(inspect) = &mut self.inspect
                    && inspect.phase != Phase::Closing
                {
                    // Ease down from the live factor so a mid-open close doesn't snap.
                    inspect.from = inspect.factor();
                    inspect.phase = Phase::Closing;
                    inspect.started = Instant::now();
                }
            },
            MainMsg::Tick => {
                if let Some(inspect) = &mut self.inspect
                    && inspect.raw() >= 1.0
                {
                    match inspect.phase {
                        Phase::Opening => inspect.phase = Phase::Open,
                        Phase::Closing => self.inspect = None,
                        Phase::Open => {},
                    }
                }
            },
            MainMsg::Ignore => {},
            MainMsg::SetBlur(value) => self.inspect_look.blur = value,
            MainMsg::SetPanelBlur(value) => self.inspect_look.panel_blur = value,
            MainMsg::SetSaturate(value) => self.inspect_look.saturate = value,
            MainMsg::SetTint(value) => self.inspect_look.tint = value,
            MainMsg::SetPanelOpacity(value) => self.inspect_look.panel_opacity = value,
            MainMsg::SetPanelShift(value) => self.inspect_look.panel_shift = value,
        }
        Task::none()
    }

    pub fn subscription(&self) -> Subscription<MainMsg> {
        match &self.inspect {
            Some(inspect) if inspect.animating() => {
                iced::time::every(Duration::from_millis(16)).map(|_| MainMsg::Tick)
            },
            _ => Subscription::none(),
        }
    }

    pub fn view(&self) -> Element<'_, MainMsg> {
        let backdrop = background(Arc::clone(&self.source), self.background_look);

        // The Inspect trigger, parked in the bottom-right corner.
        let trigger = container(button::standard(
            "Inspect",
            None,
            false,
            MainMsg::OpenInspect,
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Horizontal::Right)
        .align_y(Vertical::Bottom)
        .padding(24);

        match &self.inspect {
            None => stack![backdrop, self.content(), trigger].into(),
            Some(inspect) => {
                let factor = inspect.factor();
                let veil = mouse_area(scrim(
                    Arc::clone(&inspect.snapshot),
                    self.inspect_look.blur,
                    self.inspect_look.panel_blur,
                    self.inspect_look.saturate,
                    self.inspect_look.tint,
                    self.inspect_look.panel_shift,
                    self.inspect_look.panel_opacity,
                    inspect.size,
                    PANEL_RADIUS,
                    factor,
                ))
                .on_press(MainMsg::CloseInspect);
                stack![
                    backdrop,
                    self.content(),
                    trigger,
                    veil,
                    self.panel(inspect.size, factor)
                ]
                .into()
            },
        }
    }

    /// Example home-screen content: titled rows of posters, so there is real UI
    /// over the background to see (and for the inspect blur to capture).
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

    /// The centered modal panel: tuning sliders over the scrim's frosted pane.
    /// The container itself is transparent — the scrim draws the blurred fill —
    /// with just a fading border for definition.
    fn panel(&self, size: (f32, f32), factor: f32) -> Element<'_, MainMsg> {
        let look = self.inspect_look;
        let label = |value: String| text(value).size(13).color(color::TEXT_DEFAULT);

        let header = row![
            text("Inspect")
                .size(18)
                .color(color::TEXT_DEFAULT)
                .width(Length::Fill),
            button::standard("✕", None, false, MainMsg::CloseInspect),
        ]
        .align_y(Vertical::Center);

        let controls = column![
            header,
            label(format!("blur: {:.0}", look.blur)),
            slider(0.0..=150.0, look.blur, MainMsg::SetBlur).step(1.0),
            label(format!("panel blur: {:.0}", look.panel_blur)),
            slider(0.0..=150.0, look.panel_blur, MainMsg::SetPanelBlur).step(1.0),
            label(format!("saturate: {:.2}", look.saturate)),
            slider(0.0..=3.0, look.saturate, MainMsg::SetSaturate).step(0.05),
            label(format!("tint: {:.2}", look.tint)),
            slider(0.0..=1.0, look.tint, MainMsg::SetTint).step(0.01),
            label(format!("panel opacity: {:.2}", look.panel_opacity)),
            slider(0.0..=1.0, look.panel_opacity, MainMsg::SetPanelOpacity).step(0.01),
            label(format!("panel tone: {:.2}", look.panel_shift)),
            slider(0.0..=1.0, look.panel_shift, MainMsg::SetPanelShift).step(0.01),
        ]
        .spacing(10)
        .padding(20)
        .width(Length::Fill);

        let outline = Border {
            color: Color {
                a: 0.4 * factor,
                ..color::PRIMARY
            },
            width: 1.0,
            ..border::rounded(PANEL_RADIUS)
        };
        let panel = container(controls)
            .width(Length::Fixed(size.0))
            .height(Length::Fixed(size.1))
            .clip(true)
            .style(move |_theme| container::Style {
                border: outline,
                ..container::Style::default()
            });

        // Swallow presses on the panel so only clicks on the veil dismiss it.
        center(mouse_area(panel).on_press(MainMsg::Ignore)).into()
    }
}
