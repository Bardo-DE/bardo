//! The notification center's widgets (design/20 §1.14): Calendar, then Up Next, under the
//! history, each on quire's widget interface. quire owns the card
//! (`shell_kit::prelude::WidgetCard`); a widget is a `shell_kit::prelude::Widget`
//! (quire's `MonthWidget` and bardo-shell's own [`up_next::UpNext`]), and bardo-shell only provides each one's
//! entries from its services ([`Widget`] picks the provider for a kind).

mod card;
mod kind;
mod wake;

pub mod calendar;
pub mod up_next;

pub use card::{Card, Feed, Widget, WidgetProvider, card};
pub use kind::CenterWidget;
pub use shell_kit::prelude::WidgetSize;
pub use wake::{provide_widget_wake, use_widget_wake};

/// Where the widgets are drawn: the notification center's column, each card spanning it at
/// Medium (336 fits its 360 less the padding).
pub const SIZE: WidgetSize = WidgetSize::Medium;

/// bardo-shell's own widgets' stylesheets (quire's widgets style themselves), for the notification
/// center to inject beside its own.
#[must_use]
pub fn widgets_css() -> &'static str {
    up_next::CSS
}
