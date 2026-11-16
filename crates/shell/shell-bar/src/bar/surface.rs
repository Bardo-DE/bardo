//! The bar as a [`Surface`]: one per output, shown at once.

use dioxus::prelude::Element;
use layer_shell_host::{HostRoot, LayerConfig, OutputInfo};
use shell_model::desktop::ShellRole;
use shell_settings::ShellFile;

use super::layer::layer;
use super::tokens::BarTokens;
use super::view::bar;
use shell_ui_kit::links::{ControlCenterLink, Links};
use shell_ui_kit::surface::{OutputPolicy, Place, Presence, Surface};

/// The menu bar along the top of an output, with its menus and status items.
#[derive(Debug)]
pub enum Bar {}

impl Surface for Bar {
    const NAME: &'static str = "bar";
    type Link = ControlCenterLink;
    type Slot = ();
    const OUTPUTS: OutputPolicy = OutputPolicy::FocusedWindow;
    const ROLE: Option<ShellRole> = Some(ShellRole::Bar);
    const LAYER_ORDER: u8 = 20;
    const PRESENCE: Presence = Presence::Always;

    fn link(links: &Links) -> ControlCenterLink {
        links.control_center.clone()
    }

    fn needs(root: HostRoot, links: &Links) -> HostRoot {
        root.context(links.focus.clone())
            .context(links.actions.clone())
            .context(links.notification_center.clone())
            .context(links.edges.clone())
    }

    fn slots(_place: &Place) -> Vec<()> {
        vec![()]
    }

    fn layer(output: &OutputInfo, settings: &ShellFile, (): ()) -> LayerConfig {
        // The shipped tokens: the surface has no document yet; `bar` follows its layer with the
        // resolved ones once it is mounted.
        layer(output, settings.bar.autohide, BarTokens::default())
    }

    fn root(output: OutputInfo, (): ()) -> Element {
        bar(output)
    }
}
