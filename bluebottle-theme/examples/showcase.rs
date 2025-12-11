//! This showcase aims to show all the components and their styling for the themes
//! hopefully to show how they all look and feel.
use iced::{Background, Color, Element};
use iced::widget::{column, center, button};
use iced::theme;
use iced::widget::button::Status;
use bluebottle_theme::*;

fn main() {
    iced::application(ThemeDemoApp::default, ThemeDemoApp::update, ThemeDemoApp::view)
        .title("Bluebottle Theme Demo")
        .theme(bluebottle_dark)
        .run()
        .unwrap();
}


#[derive(Default)]
struct ThemeDemoApp {

}

#[derive(Debug, Copy, Clone)]
enum Message {

}

impl ThemeDemoApp {
    fn view(&self) -> Element<'_, Message> {

        let column = column![
            button("Filled")
            .style(|theme: &theme::Theme, status| {
                    let palette = theme.palette();

                    button::Style {
                        background: Some(Background::Color(palette.primary)),
                        text_color: palette.text,
                        border: iced::Border {
                            radius: 4.0.into(),
                            ..iced::Border::default()
                        },
                        ..button::Style::default()
                    }
                }),

            button("Outline"),
        ].spacing(8);

        center(column).into()
    }

    fn update(&mut self, _message: Message) {

    }
}