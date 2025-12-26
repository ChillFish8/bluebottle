use iced::Element;
use iced::widget::space;

use crate::view;

pub mod library_select;
pub mod library_view;
pub mod loading;
pub mod settings;
pub mod setup;

/// A screen is a top level component that holds some significance to navigation.
pub trait Screen<Message>: view::View<Message> {
    /// Signals to the app to hide the side navbar when the screen is rendered.
    const HIDE_SIDEBAR: bool = false;

    /// The descriptor for the given screen which is displayed in the top right of the
    /// top navbar.
    fn nav_descriptor(&self) -> &str;

    /// The center element to display.
    fn nav_center<'a>(&self) -> Element<'a, Message>
    where
        Message: 'a,
    {
        space().into()
    }
}
