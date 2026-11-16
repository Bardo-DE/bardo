//! A menu taller than the room it has scrolls inside, and says so (design/13, `NSMenu`): an arrow
//! strip, a full-width band one row high holding a chevron, at each end that has more rows past
//! it. Resting the pointer on a strip scrolls the list a step each `DelayToken::ScrollArrowStep`;
//! a strip goes once its end is reached. The wheel and a trackpad scroll the list as they do any
//! other, and a highlight moved by a key is brought into view.

use crate::components::controls::scroller::handle::{ScrollerRef, use_scroller};
use crate::components::menus::menu::drift::{Drift, DriftIn, Edge};
use crate::components::menus::menu::placement::MENU_INSET;
use crate::components::menus::menu::tracker::Tracker;
use crate::host::measure::MountedRef;
use dioxus::prelude::*;
use ui_core::geometry::scroll::Scroll;
use ui_core::geometry::units::Px;
use ui_motion::machine::{MachineRef, use_machine};
use ui_style::icon::Icon;
use ui_style::icon::render::{Glyph, IconSize};
use ui_style::tokens::delay::DelayToken;

/// The two ends' strips: whether an arrow strip is drawn at each end (rows lie past it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Cues {
    pub start: bool,
    pub end: bool,
}

/// How much hidden length counts as hidden at all: below this the end is reached.
const NEGLIGIBLE: f32 = 0.5;

/// Which ends have rows past them. The list's padding `pad` is not a row: that much past an end
/// still shows nothing more.
pub(crate) fn cues(scroll: Scroll, pad: Px) -> Cues {
    let past = |hidden: f32| hidden - pad.0 > NEGLIGIBLE;
    Cues {
        start: past(scroll.offset.0),
        end: past(scroll.max_offset().0 - scroll.offset.0),
    }
}

/// A menu panel's list and the strips over its ends.
#[derive(Clone, Copy)]
pub(crate) struct Scrolling {
    scroller: ScrollerRef,
    drift: MachineRef<Drift>,
    cues: Memo<Cues>,
}

/// The scroll state of a panel's list and the drift of a held strip.
pub(crate) fn use_scrolling() -> Scrolling {
    let scroller = use_scroller();
    let shown = use_memo(move || cues(*scroller.scroll().read(), MENU_INSET));
    let drift = use_machine(
        |_| Drift::Idle,
        DelayToken::ScrollArrowStep.delay(),
        || (),
        move |edge: Edge, machine| {
            let before = *scroller.scroll().peek();
            scroller.by(edge.step());
            // At the end there is nowhere to go: stop, whether or not the pointer is still on it.
            if *scroller.scroll().peek() == before {
                machine.send(DriftIn::Off);
            }
        },
    );
    Scrolling {
        scroller,
        drift,
        cues: shown,
    }
}

impl Scrolling {
    /// The panel's `rows`, in the list that scrolls, between the strips that show.
    pub(crate) fn frame(self, tracker: Tracker, rows: Element) -> Element {
        let Cues { start, end } = *self.cues.read();
        let scroller = self.scroller;
        rsx! {
            if start {
                {self.strip(Edge::Start)}
            }
            div {
                class: "ds-menu-list",
                onmounted: move |event: MountedEvent| {
                    let element = MountedRef(event.data());
                    scroller.attach(element.clone());
                    tracker.list_mounted(element);
                },
                onscroll: move |event: ScrollEvent| scroller.heard(Px(event.scroll_top() as f32)),
                {rows}
            }
            if end {
                {self.strip(Edge::End)}
            }
        }
    }

    /// The strip at `edge`: a chevron toward the rows past it, scrolling them in while the
    /// pointer rests on it.
    fn strip(self, edge: Edge) -> Element {
        let drift = self.drift;
        let icon = match edge {
            Edge::Start => Icon::ChevronUp,
            Edge::End => Icon::ChevronDown,
        };
        rsx! {
            div {
                class: "ds-menu-scroll",
                "data-end": edge.slug(),
                "aria-hidden": "true",
                onpointerenter: move |_| drift.send(DriftIn::Over(edge)),
                onpointerleave: move |_| drift.send(DriftIn::Off),
                Glyph { icon, size: IconSize::Tiny }
            }
        }
    }
}

/// Bring the highlighted row into view when it moves, unless the pointer is on it already (the
/// row under the pointer is where it was put, and scrolling would slide the next one under it).
pub(crate) fn use_in_view(
    tracker: Tracker,
    highlighted: Option<usize>,
    pointed: CopyValue<Option<usize>>,
) {
    use_effect(use_reactive((&highlighted,), move |(highlighted,)| {
        if let Some(index) = highlighted
            && *pointed.peek() != Some(index)
        {
            tracker.show_row(index);
        }
    }));
}

#[cfg(test)]
mod tests {
    use super::{Cues, cues};
    use ui_core::geometry::scroll::Scroll;
    use ui_core::geometry::units::Px;

    fn at(offset: f32, viewport: f32, content: f32) -> Scroll {
        Scroll {
            offset: Px(offset),
            viewport: Px(viewport),
            content: Px(content),
        }
    }

    #[test]
    fn a_strip_shows_only_where_rows_lie_past_the_end() {
        const HIDDEN: bool = false;
        const SHOWN: bool = true;
        // (name, scroll, cues)
        #[rustfmt::skip]
        let cases = [
            ("not measured", at(0.0, 0.0, 0.0), Cues { start: HIDDEN, end: HIDDEN }),
            ("fits", at(0.0, 300.0, 300.0), Cues { start: HIDDEN, end: HIDDEN }),
            ("only padding past the end", at(0.0, 300.0, 305.0), Cues { start: HIDDEN, end: HIDDEN }),
            ("more rows below", at(0.0, 300.0, 700.0), Cues { start: HIDDEN, end: SHOWN }),
            ("in the middle", at(100.0, 300.0, 700.0), Cues { start: SHOWN, end: SHOWN }),
            ("at the end", at(400.0, 300.0, 700.0), Cues { start: SHOWN, end: HIDDEN }),
            ("a row's padding from the top", at(5.0, 300.0, 700.0), Cues { start: HIDDEN, end: SHOWN }),
        ];
        for (name, scroll, want) in cases {
            assert_eq!(cues(scroll, Px(5.0)), want, "{name}");
        }
    }
}
