pub mod animate;
pub mod border;
pub mod color;
pub mod easing;
pub mod font;
pub mod icon;
pub mod spacing;
pub mod style;
pub mod util;
mod widget;

pub use self::widget::blurred_image::{BlurRegion, blurred_image};
pub use self::widget::breadcrumb::breadcrumb;
pub use self::widget::card::{
    album_card,
    album_card_skeleton,
    episode_still,
    episode_still_skeleton,
    poster_card,
    poster_card_skeleton,
};
pub use self::widget::clickable::clickable;
pub use self::widget::link::link;
pub use self::widget::media_image::{PillCorner, media_image, media_image_skeleton};
pub use self::widget::picks_switcher::picks_switcher;
pub use self::widget::poster_fan::{PosterFan, poster_fan};
pub use self::widget::scrollable::scrollable;
pub use self::widget::sidebar::sidebar;
pub use self::widget::skeleton::skeleton;
pub use self::widget::smart_list::{smart_group, smart_list};
pub use self::widget::splash_background::{splash_background, splash_panel};
pub use self::widget::sticky::{Sticky, sticky};
pub use self::widget::tabs::{Tab, tab, tabs};
pub use self::widget::{dropdown, *};
