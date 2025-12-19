use iced::Element;

pub mod library_select;
pub mod library_view;
pub mod loading;
pub mod settings;
pub mod setup;

/// A [Screen] describes a standard view/state the UI can be in.
pub trait Screen<Message> {
    /// Update the screen state based on the given event.
    fn update(&mut self, message: Message);

    /// Render the view for the screen.
    fn view(&self) -> Element<'_, Message>;
}
