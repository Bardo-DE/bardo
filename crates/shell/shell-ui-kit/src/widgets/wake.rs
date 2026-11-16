//! The stamp a widget host provides to wake every widget's fill on open or show
//! (design/23-WIDGETS.md section 4.1; quire's `BatteryRing`, `Epoch`): the
//! notification center provides one from its own opening
//! (`crate::panel::Opening`).
//! A widget reads it with [`use_widget_wake`] instead of a `wake` prop threaded through
//! [`crate::widgets::Widget`] and its host, so every host wakes its widgets in one place.

use dioxus::prelude::*;
use ui_components::motion::epoch::Epoch;

/// The center's wake, provided once for the widgets under it to read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct WidgetWake(pub Epoch);

/// `wake`, from the nearest host's [`WidgetWake`], or its default when none is running (a widget
/// mounted alone, as a test does).
#[must_use]
pub fn use_widget_wake() -> Epoch {
    try_use_context::<WidgetWake>().unwrap_or_default().0
}

/// Provide `stamp` as this subtree's [`WidgetWake`], for [`use_widget_wake`] to read.
pub fn provide_widget_wake(stamp: Epoch) {
    use_context_provider(|| WidgetWake(stamp));
}
