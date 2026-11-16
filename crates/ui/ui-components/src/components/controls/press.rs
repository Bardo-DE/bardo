//! Press: what a Button hands its `onclick`: which pointer button activated it, the modifiers held
//! and where it happened.
//!
//! A tray icon's right-click has to reach the app as a secondary press, its middle click as a
//! middle one, and SNI's `ContextMenu(x, y)` and `Activate(x, y)` want the point.

use crate::focus::click::kept_click;
use crate::focus::press_focus::on_click;
use crate::host::narrow;
use dioxus::html::input_data::MouseButton;
use dioxus::prelude::*;
use ui_core::geometry::units::{Point, Px};
use ui_core::press::{PointerButton, Press};
use ui_core::vocab::{Availability, PressPhase};

/// The press a mouse event describes, as `button`.
pub(crate) fn press_of(event: &MouseEvent, button: PointerButton) -> Press {
    let at = event.client_coordinates();
    Press {
        button,
        modifiers: event.modifiers(),
        at: Point {
            x: Px(narrow::f32_of(at.x)),
            y: Px(narrow::f32_of(at.y)),
        },
    }
}

/// The pointer button a mouse event's trigger names. No trigger is a keyboard activation,
/// which reports primary; the back and forward buttons (and unknown ones) are not presses.
#[must_use]
pub fn button_of(trigger: Option<MouseButton>) -> Option<PointerButton> {
    match trigger {
        None | Some(MouseButton::Primary) => Some(PointerButton::Primary),
        Some(MouseButton::Secondary) => Some(PointerButton::Secondary),
        Some(MouseButton::Auxiliary) => Some(PointerButton::Middle),
        Some(MouseButton::Fourth | MouseButton::Fifth | MouseButton::Unknown) => None,
    }
}

/// Keep `event` at the control when `stop` is set. A button inside a `<summary>` (a collapsible
/// section's header action) must keep its press to itself, or the `<details>` around it toggles
/// as well: the caller only receives a [`Press`], never the event, so it cannot stop the event
/// itself. A stopped press ends at the control: its propagation is stopped before `onclick` runs,
/// and its default action is prevented, since on Blitz a click's default action walks up the
/// ancestors (it is what toggles a `<details>` from its `<summary>`). A `type=button` has no
/// default action of its own, so nothing the control does is lost. The keyboard still moves to
/// the control, as for any press: the control hands the click to the host's click focus itself
/// (FINDINGS "Native focus").
fn keep_at_control(stop: bool, event: &MouseEvent) {
    if stop {
        event.stop_propagation();
        event.prevent_default();
    }
}

/// What a control's availability writes on the element besides `aria-disabled`: `disabled`, so the
/// platform neither focuses nor activates it.
///
/// Present only when disabled, as an attribute string: a `bool` attribute reaches dioxus-native as
/// `disabled="false"` on every enabled control, which Blitz reads as disabled (a click then no
/// longer toggles an enclosing `<details>`). A busy control takes no input either, but stays where
/// it is in the tab order.
#[must_use]
pub fn disabled(availability: Availability) -> Option<&'static str> {
    match availability {
        Availability::Enabled | Availability::Busy => None,
        Availability::Disabled => Some("true"),
    }
}

/// The keys that activate a focused control.
///
/// A push button answers Return and Space; a button that answers Escape leaves Return to the dialog
/// around it (the default button's key) and answers Space only, as does a switch, a checkbox, a
/// radio button or a segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ActivationKeys {
    /// Return and Space.
    #[default]
    ReturnAndSpace,
    /// Space alone.
    SpaceOnly,
}

/// Whether `key` is Space.
pub(crate) fn is_space(key: &Key) -> bool {
    matches!(key, Key::Character(text) if text == " ")
}

/// Whether `key` activates a control that takes `keys`.
pub(crate) fn is_activation_key(key: &Key, keys: ActivationKeys) -> bool {
    match keys {
        ActivationKeys::ReturnAndSpace => matches!(key, Key::Enter) || is_space(key),
        ActivationKeys::SpaceOnly => is_space(key),
    }
}

/// Whether `event` is a fresh activation for a control that takes `keys`: a held key's repeats
/// are not.
pub(crate) fn activates(event: &KeyboardEvent, keys: ActivationKeys) -> bool {
    is_activation_key(&event.key(), keys) && !event.is_auto_repeating()
}

/// The three listeners a pressable control puts on its element, all reporting through `press`.
///
/// `click` (primary, and keyboard activation), `contextmenu` (secondary: Blitz and browsers send a
/// right-click as that and never as a click; its default is prevented), and `mouseup` for the
/// middle button, which neither fires as a click on Blitz. Under a stopped press each of them keeps
/// its event at the control before reporting.
#[derive(Debug, Clone, Copy)]
pub struct PressListeners {
    press: EventHandler<Press>,
    stop_propagation: bool,
}

impl PressListeners {
    /// Listeners that report to `press` and let the event bubble on.
    #[must_use]
    pub fn new(press: EventHandler<Press>) -> Self {
        PressListeners {
            press,
            stop_propagation: false,
        }
    }

    /// The same listeners, keeping the event at the control (`true`) or passing it on.
    #[must_use]
    pub fn with_stop_propagation(self, stop_propagation: bool) -> Self {
        PressListeners {
            stop_propagation,
            ..self
        }
    }

    /// A `click`: primary, or whatever button the event names. A click kept at the control
    /// (a stopped press) is handed to the host's click focus last, as the root would have
    /// (`focus::click::kept_click`): the control has the keyboard afterwards.
    pub fn click(&self, event: &MouseEvent) {
        keep_at_control(self.stop_propagation, event);
        on_click(event);
        if let Some(button) = button_of(event.trigger_button()) {
            self.press.call(press_of(event, button));
        }
        if self.stop_propagation {
            kept_click(event);
        }
    }

    /// A `contextmenu`: a secondary press. The page's own menu is the app's to open.
    pub fn context_menu(&self, event: &MouseEvent) {
        event.prevent_default();
        keep_at_control(self.stop_propagation, event);
        self.press.call(press_of(event, PointerButton::Secondary));
    }

    /// A `keydown`: the `keys` that activate the control report a primary press at the origin
    /// (a keyboard activation has no point). Blitz raises no `click` for a key on a focused
    /// `button`, so the control reports the activation itself, and the key stays at the control:
    /// what a key activates nothing around it hears as well. A held key does not repeat it.
    pub fn key_down(&self, event: &KeyboardEvent, keys: ActivationKeys) {
        if !activates(event, keys) {
            return;
        }
        event.prevent_default();
        event.stop_propagation();
        self.press.call(Press {
            modifiers: event.modifiers(),
            ..Press::primary()
        });
    }

    /// A `mouseup`: only the middle button counts (the primary one arrives as `click`).
    pub fn mouse_up(&self, event: &MouseEvent) {
        if event.trigger_button() == Some(MouseButton::Auxiliary) {
            keep_at_control(self.stop_propagation, event);
            self.press.call(press_of(event, PointerButton::Middle));
        }
    }
}

/// Where the press on a control stands, for `data-pressed` (design/30 section 1.4).
///
/// Down by the primary button or by Space or Return, until it is released, leaves the control or
/// the control loses the keyboard. A control puts [`Pressing::listeners`] on its element and writes
/// [`Pressing::attr`].
#[derive(Debug, Clone, Copy)]
pub struct Pressing {
    phase: Signal<PressPhase>,
}

/// A [`Pressing`] for one control.
pub fn use_pressing() -> Pressing {
    Pressing {
        phase: use_signal(PressPhase::default),
    }
}

impl Pressing {
    /// `data-pressed`: `"true"` while a press is under way.
    #[must_use]
    pub fn attr(&self) -> Option<&'static str> {
        self.phase.read().attr()
    }

    fn set(&self, next: PressPhase) {
        let mut phase = self.phase;
        if *phase.peek() != next {
            phase.set(next);
        }
    }

    /// The pointer went down.
    pub fn pointer_down(&self, event: &MouseEvent) {
        if button_of(event.trigger_button()) == Some(PointerButton::Primary) {
            self.set(PressPhase::Pressed);
        }
    }

    /// A key went down: the `keys` that activate the control press it.
    pub fn key_down(&self, event: &KeyboardEvent, keys: ActivationKeys) {
        if is_activation_key(&event.key(), keys) {
            self.set(PressPhase::Pressed);
        }
    }

    /// The press ended, or left the control.
    pub fn released(&self) {
        self.set(PressPhase::Idle);
    }
}

#[cfg(test)]
mod tests {
    use super::{ActivationKeys, button_of, disabled, is_activation_key};
    use dioxus::html::input_data::MouseButton;
    use dioxus::prelude::Key;
    use ui_core::press::PointerButton;
    use ui_core::vocab::Availability;

    #[test]
    fn only_a_disabled_control_is_disabled() {
        assert_eq!(disabled(Availability::Disabled), Some("true"));
        assert_eq!(disabled(Availability::Enabled), None);
        assert_eq!(disabled(Availability::Busy), None);
    }

    #[test]
    fn return_and_space_activate_and_nothing_else_does() {
        use ActivationKeys::{ReturnAndSpace, SpaceOnly};
        let cases = [
            (Key::Enter, ReturnAndSpace, true),
            (Key::Enter, SpaceOnly, false),
            (Key::Character(" ".to_string()), ReturnAndSpace, true),
            (Key::Character(" ".to_string()), SpaceOnly, true),
            (Key::Character("a".to_string()), ReturnAndSpace, false),
            (Key::Tab, ReturnAndSpace, false),
            (Key::Escape, ReturnAndSpace, false),
        ];
        for (key, keys, want) in cases {
            assert_eq!(is_activation_key(&key, keys), want, "{key:?} {keys:?}");
        }
    }

    #[test]
    fn a_trigger_names_its_button() {
        const CASES: &[(Option<MouseButton>, Option<PointerButton>)] = &[
            (None, Some(PointerButton::Primary)),
            (Some(MouseButton::Primary), Some(PointerButton::Primary)),
            (Some(MouseButton::Secondary), Some(PointerButton::Secondary)),
            (Some(MouseButton::Auxiliary), Some(PointerButton::Middle)),
            (Some(MouseButton::Fourth), None),
            (Some(MouseButton::Fifth), None),
            (Some(MouseButton::Unknown), None),
        ];
        for (trigger, want) in CASES {
            assert_eq!(button_of(*trigger), *want, "{trigger:?}");
        }
    }
}
