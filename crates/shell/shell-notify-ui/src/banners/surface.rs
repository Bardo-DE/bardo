//! The banner column as a [`Surface`]: one per output, spawned hidden, mapped while a card is up.

use dioxus::prelude::Element;
use layer_shell_host::{KeyboardFocus, LayerConfig, OutputId, OutputInfo};
use shell_model::desktop::ShellRole;
use shell_settings::ShellFile;

use super::layer::layer;
use super::view::banners;
use shell_ui_kit::links::{BannerLink, Links, Transient};
use shell_ui_kit::notification_card::NotificationTokens;
use shell_ui_kit::surface::{FocusLoss, Place, Presence, Surface};

/// The compositor taking the keyboard from the banner column is told on the link, which says which
/// transient lost it.
fn keyboard_lost(link: &BannerLink, _output: OutputId, focus: KeyboardFocus) {
    if focus == KeyboardFocus::Lost {
        link.keyboard_lost(Transient::Banner);
    }
}

/// The notification banners, stacked down the main display's right edge.
#[derive(Debug)]
pub enum Banners {}

impl Surface for Banners {
    const NAME: &'static str = "banners";
    type Link = BannerLink;
    type Slot = ();
    const KEYBOARD: FocusLoss<BannerLink> = FocusLoss::CloseLink(keyboard_lost);
    const ROLE: Option<ShellRole> = Some(ShellRole::Notifications);
    const LAYER_ORDER: u8 = 180;
    const PRESENCE: Presence = Presence::SelfMapped;

    fn link(links: &Links) -> BannerLink {
        links.banners.clone()
    }

    fn slots(_place: &Place) -> Vec<()> {
        vec![()]
    }

    fn layer(output: &OutputInfo, _settings: &ShellFile, (): ()) -> LayerConfig {
        // The shipped tokens: the surface has no document yet; `ColumnView` follows its size
        // and margin with the resolved ones once it is mounted.
        layer(output, &NotificationTokens::default())
    }

    fn root(output: OutputInfo, (): ()) -> Element {
        banners(&output)
    }
}
