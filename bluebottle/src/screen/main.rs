use bluebottle_ui::splash_background::Backdrop;
use bluebottle_ui::{button, color, image, scrollable, sidebar, splash_background};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{column, container, row, stack, text};
use iced::{Element, Length};

use crate::backdrop;

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
    /// Open the sidebar.
    OpenSidebar,
    /// Close the sidebar (X button or click-away).
    CloseSidebar,
}

/// State for the main library surface.
pub struct MainScreen {
    /// The page background image, or `None` for the glow.
    backdrop: Option<Backdrop>,
    /// Whether the sidebar is open. The widget animates the slide itself.
    sidebar_open: bool,
    /// The poster blurred behind the drawer (the same one it shows).
    sidebar_backdrop: Option<Backdrop>,
    /// Decoded handles for the example poster content.
    posters: Vec<image::Handle>,
}

impl MainScreen {
    /// Builds the screen over `backdrop`, with the default look.
    pub fn new(backdrop: Option<Backdrop>) -> Self {
        Self {
            backdrop,
            sidebar_open: false,
            sidebar_backdrop: backdrop::decode_bytes(POSTER_BYTES[SIDEBAR_POSTER]),
            posters: POSTER_BYTES
                .iter()
                .map(|bytes| image::Handle::from_bytes(*bytes))
                .collect(),
        }
    }

    pub fn update(&mut self, message: MainMsg) {
        match message {
            MainMsg::OpenSidebar => self.sidebar_open = true,
            MainMsg::CloseSidebar => self.sidebar_open = false,
        }
    }

    pub fn view(&self) -> Element<'_, MainMsg> {
        let backdrop = splash_background(self.backdrop.clone());

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

        // The drawer renders nothing and passes events through when closed, so a
        // single layout serves both states.
        let drawer = sidebar(self.drawer_body(), self.sidebar_backdrop.clone())
            .open(self.sidebar_open)
            .on_dismiss(MainMsg::CloseSidebar);

        stack![backdrop, self.content(), trigger, drawer].into()
    }

    /// Example home-screen content, titled rows of posters, so there is real UI
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

    /// The drawer's contents, a header and the detail poster. The frosted
    /// background, border, and shadow live inside the sidebar widget.
    fn drawer_body(&self) -> Element<'_, MainMsg> {
        let header = row![
            text("Details")
                .size(20)
                .color(color::TEXT_DEFAULT)
                .width(Length::Fill),
            button::standard("✕", None, false, MainMsg::CloseSidebar),
        ]
        .align_y(Vertical::Center);

        column![
            header,
            image::poster(
                self.posters[SIDEBAR_POSTER].clone(),
                image::PosterSize::Large
            ),
        ]
        .spacing(20)
        .padding(24)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
