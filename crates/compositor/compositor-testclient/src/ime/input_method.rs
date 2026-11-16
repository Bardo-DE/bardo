//! `zwp_input_method_v2`: the client side of an input method (what fcitx5 is). Filled by
//! P0-H7. Self-contained like [`super::text_input`]: the client state's `Dispatch` impls turn each
//! wire event into an [`InputMethodEvent`] with [`method_event`], [`grab_event`] and
//! [`popup_event`].

use crate::ClientError;
use crate::GlobalList;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{Dispatch, QueueHandle};
use wayland_protocols_misc::zwp_input_method_v2::client::{
    zwp_input_method_keyboard_grab_v2,
    zwp_input_method_manager_v2::ZwpInputMethodManagerV2,
    zwp_input_method_v2::{self, ZwpInputMethodV2},
    zwp_input_popup_surface_v2,
};

/// The user data of the method object.
#[derive(Debug, Clone, Copy, Default)]
pub struct MethodData;

/// The user data of its keyboard grab.
#[derive(Debug, Clone, Copy, Default)]
pub struct GrabData;

/// The user data of its popup surface.
#[derive(Debug, Clone, Copy, Default)]
pub struct PopupData;

/// One event the compositor sent an input method.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMethodEvent {
    /// A field was enabled: the method may compose.
    Activate,
    /// The field is gone.
    Deactivate,
    /// The field's text around the cursor.
    SurroundingText {
        /// The text.
        text: String,
        /// Cursor byte offset.
        cursor: u32,
        /// Anchor byte offset.
        anchor: u32,
    },
    /// The kind of field.
    ContentType {
        /// The hints, as the wire's bit set.
        hint: u32,
        /// The purpose, as the wire's value.
        purpose: u32,
    },
    /// The end of one batch of state.
    Done,
    /// Another method holds the seat.
    Unavailable,
    /// The grabbed keyboard sent a key.
    GrabKey {
        /// The key (evdev code).
        key: u32,
        /// Pressed (`1`) or released (`0`).
        state: u32,
    },
    /// The grabbed keyboard's modifier state.
    GrabModifiers {
        /// Depressed mask.
        depressed: u32,
        /// Latched mask.
        latched: u32,
        /// Locked mask.
        locked: u32,
    },
    /// The grab began with a keymap of this many bytes.
    GrabKeymap {
        /// The keymap's size.
        size: u32,
    },
    /// The popup surface's text-input rectangle moved.
    PopupRectangle {
        /// X.
        x: i32,
        /// Y.
        y: i32,
        /// Width.
        width: i32,
        /// Height.
        height: i32,
    },
}

impl InputMethodEvent {
    /// The event's wire name.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            InputMethodEvent::Activate => "activate",
            InputMethodEvent::Deactivate => "deactivate",
            InputMethodEvent::SurroundingText { .. } => "surrounding_text",
            InputMethodEvent::ContentType { .. } => "content_type",
            InputMethodEvent::Done => "done",
            InputMethodEvent::Unavailable => "unavailable",
            InputMethodEvent::GrabKey { .. } => "key",
            InputMethodEvent::GrabModifiers { .. } => "modifiers",
            InputMethodEvent::GrabKeymap { .. } => "keymap",
            InputMethodEvent::PopupRectangle { .. } => "text_input_rectangle",
        }
    }
}

/// The event the compositor sent the method object, if it is one this client records.
#[must_use]
pub fn method_event(event: zwp_input_method_v2::Event) -> Option<InputMethodEvent> {
    Some(match event {
        zwp_input_method_v2::Event::Activate => InputMethodEvent::Activate,
        zwp_input_method_v2::Event::Deactivate => InputMethodEvent::Deactivate,
        zwp_input_method_v2::Event::SurroundingText {
            text,
            cursor,
            anchor,
        } => InputMethodEvent::SurroundingText {
            text,
            cursor,
            anchor,
        },
        zwp_input_method_v2::Event::ContentType { hint, purpose } => {
            InputMethodEvent::ContentType {
                hint: u32::from(hint),
                purpose: u32::from(purpose),
            }
        }
        zwp_input_method_v2::Event::Done => InputMethodEvent::Done,
        zwp_input_method_v2::Event::Unavailable => InputMethodEvent::Unavailable,
        _ => return None,
    })
}

/// The event the grabbed keyboard sent, if it is one this client records.
#[must_use]
pub fn grab_event(event: zwp_input_method_keyboard_grab_v2::Event) -> Option<InputMethodEvent> {
    Some(match event {
        zwp_input_method_keyboard_grab_v2::Event::Keymap { size, .. } => {
            InputMethodEvent::GrabKeymap { size }
        }
        zwp_input_method_keyboard_grab_v2::Event::Key { key, state, .. } => {
            InputMethodEvent::GrabKey {
                key,
                state: u32::from(state),
            }
        }
        zwp_input_method_keyboard_grab_v2::Event::Modifiers {
            mods_depressed,
            mods_latched,
            mods_locked,
            ..
        } => InputMethodEvent::GrabModifiers {
            depressed: mods_depressed,
            latched: mods_latched,
            locked: mods_locked,
        },
        _ => return None,
    })
}

/// The event the popup surface got, if it is one this client records.
#[must_use]
pub fn popup_event(event: zwp_input_popup_surface_v2::Event) -> Option<InputMethodEvent> {
    let zwp_input_popup_surface_v2::Event::TextInputRectangle {
        x,
        y,
        width,
        height,
    } = event
    else {
        return None;
    };
    Some(InputMethodEvent::PopupRectangle {
        x,
        y,
        width,
        height,
    })
}

/// Binds the input-method manager and asks for the method of `seat`.
pub fn bind_input_method<S>(
    globals: &GlobalList,
    qh: &QueueHandle<S>,
    seat: &WlSeat,
) -> Result<ZwpInputMethodV2, ClientError>
where
    S: Dispatch<ZwpInputMethodManagerV2, ()> + Dispatch<ZwpInputMethodV2, MethodData> + 'static,
{
    let manager = globals
        .bind::<ZwpInputMethodManagerV2, S, ()>(qh, 1..=1, ())
        .map_err(|_| ClientError::MissingGlobal("zwp_input_method_manager_v2".to_owned()))?;
    Ok(manager.get_input_method(seat, qh, MethodData))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_name_themselves_like_the_wire() {
        assert_eq!(InputMethodEvent::Activate.name(), "activate");
        assert_eq!(
            InputMethodEvent::GrabKey { key: 30, state: 1 }.name(),
            "key"
        );
    }
}
