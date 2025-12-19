use iced::{Element, task};

/// A [View] describes a standard view/state of a UI component.
pub trait View<Message> {
    /// Update the screen state based on the given event.
    fn update(&mut self, message: Message) -> task::Task<Message>;

    /// Render the view for the screen.
    fn view(&self) -> Element<'_, Message>;
}
