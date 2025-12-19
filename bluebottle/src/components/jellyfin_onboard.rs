//! Onboard a new Jellyfin media library.

use iced::Element;

use crate::view;

pub struct JellyfinOnboard {
    jellyfin_server_url: String,
    jellyfin_username: String,
    jellyfin_password: String,
}

pub enum JellyfinOnboardMsg {

}

impl view::View<JellyfinOnboardMsg> for JellyfinOnboard {
    fn update(&mut self, message: JellyfinOnboardMsg) {
        todo!()
    }

    fn view(&self) -> Element<'_, JellyfinOnboardMsg> {
        todo!()
    }
}
