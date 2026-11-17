//! One widget of any kind: the [`WidgetProvider`] every kind implements, the [`Card`] that puts a
//! provider's feed on the UI kit's card, and the one match on the kind ([`with_provider`]).

use dioxus::prelude::*;
use shell_kit::prelude::*;
use shell_kit::widget::timeline::Timeline;
use ui_components::prelude::*;

use super::{
    CenterWidget, SIZE, calendar::CalendarProvider, up_next::UpNextProvider, use_widget_wake,
};

/// What a provider hands the card: the entry now, and where the widget's controls go (none for
/// a widget without any).
pub struct Feed<W: shell_kit::prelude::Widget> {
    pub entry: W::Entry,
    pub onintent: Option<EventHandler<W::Intent>>,
}

impl<W: shell_kit::prelude::Widget> std::fmt::Debug for Feed<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Feed")
            .field("intents", &self.onintent.is_some())
            .finish_non_exhaustive()
    }
}

/// One kind's provider: the `shell_kit::prelude::Widget` it feeds, its kind, and the hook
/// that reads its services into a [`Feed`].
pub trait WidgetProvider: Clone + PartialEq + Default + 'static {
    type W: shell_kit::prelude::Widget;
    const KIND: CenterWidget;
    fn use_feed() -> Feed<Self::W>;
}

/// `provider`'s widget on the UI kit's card in the notification center, its element id
/// `widget-<kind>`; the provider value only names the type.
#[component]
pub fn Card<P: WidgetProvider>(provider: P) -> Element {
    let _ = provider;
    let Feed { entry, onintent } = P::use_feed();
    rsx! {
        WidgetCard {
            widget: <P::W as Default>::default(),
            timeline: Timeline::now(entry),
            size: SIZE,
            wake: use_widget_wake(),
            common: Common { id: Some(format!("widget-{}", P::KIND.slug())), ..Common::default() },
            onintent,
        }
    }
}

/// `kind`'s card: the only match on the kind.
#[expect(
    clippy::missing_errors_doc,
    reason = "`Element` is dioxus's `Result` alias; a component renders or returns `Ok`, and the caller is the dioxus runtime"
)]
pub fn card(kind: CenterWidget) -> Element {
    match kind {
        CenterWidget::Calendar => rsx! { Card { provider: CalendarProvider } },
        CenterWidget::UpNext => rsx! { Card { provider: UpNextProvider } },
    }
}

/// One widget of `kind` on the UI kit's card in the notification center: the kind's provider feeds
/// the card.
#[component]
pub fn Widget(kind: CenterWidget) -> Element {
    card(kind)
}
