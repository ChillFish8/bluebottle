use iced::Element;

/// Creates a new search input widget which triggers an event when ever the user types.
pub fn search<'a, Message>(
    placeholder: &'a str,
    content: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    super::input::text_input(placeholder, content, on_input).into()
}
