//! `use_styles`: the styles every `Ds` root draws, live, and whether the person's are drawn.

use dioxus::prelude::{ReadSignal, try_consume_context, use_hook};
use layer_shell_host::use_watch;
use tokio::sync::watch;
use ui_blitz::StylesGuard;
use ui_components::style::kit::Styles;

use crate::hooks::use_appearance;
use crate::settings::SettingsFeed;

/// What a `Ds` root takes for its styles: the `styles` prop and the `custom_styles` prop.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StyleSource {
    /// The three sheets, live; empty when the document was built without a [`SettingsFeed`].
    pub styles: ReadSignal<Styles>,
    /// `appearance.custom_styles`, forced off while the crash guard has the styles off.
    pub custom_styles: bool,
}

/// The [`StyleSource`] of this document. Call it before the component's first early return.
pub fn use_styles() -> StyleSource {
    let feed = use_hook(try_consume_context::<SettingsFeed>);
    let (styles, guard) = use_hook(|| match feed {
        Some(feed) => (feed.styles, feed.guard),
        None => (watch::channel(Styles::default()).1, watch::channel(None).1),
    });
    let styles = use_watch(styles);
    let guard = use_watch(guard);
    let appearance = use_appearance();
    let withheld = guard().as_ref().is_some_and(|x| !StylesGuard::load(x));
    let custom_styles = !withheld && appearance().settings.appearance.custom_styles;
    StyleSource {
        styles,
        custom_styles,
    }
}
