/// Creates a text widget that will truncate to a `...` when the text cannot be fit entirely.
pub fn ellipsis_text(
    text: &str,
) -> iced_palace::widget::EllipsizedText<'_, iced::Theme, iced::Renderer> {
    iced_palace::widget::ellipsized_text(text)
}
