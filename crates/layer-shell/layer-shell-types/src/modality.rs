//! How the person last drove the seat, pointer or keyboard, published to every surface's
//! document (`use_input_modality`): what a design system's
//! `:focus-visible`-like focus ring follows. Blitz never matches `:focus-visible` (a quire
//! finding), so quire draws its ring under `.ds[data-modality=keyboard]` and asks the host, which
//! sees the raw input, which modality is current (`ui_components::HostModality`).
//!
//! One value per seat, shared by every surface: a key press in the launcher and a Tab into a
//! bar menu are the same person at the same keyboard, and a surface mapped mid-session (a popup
//! opened with a key) starts from the current value rather than from `Pointer`. The rule is
//! quire's own host's (`ds-native`'s `modality_after`) and the one browsers use for
//! `:focus-visible`: a key press that is not a lone modifier turns it `Keyboard`, a pointer
//! button press turns it `Pointer`, and nothing else changes it — not pointer motion, not a
//! scroll, not a release or an auto-repeat. Motion is deliberately left out: a hand brushing
//! the mouse (or a palm on the touchpad) in the middle of a Tab walk would otherwise hide the
//! ring it is following.

/// The kind of input the seat last saw that decides the modality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum InputModality {
    /// The last deciding input was a pointer press (and before any input at all): no focus
    /// rings.
    #[default]
    Pointer,
    /// The last deciding input was a key: focus rings show.
    Keyboard,
}

impl InputModality {
    /// The conventional attribute value: quire's `data-modality`.
    #[must_use]
    pub fn slug(self) -> &'static str {
        match self {
            InputModality::Pointer => "pointer",
            InputModality::Keyboard => "keyboard",
        }
    }
}
