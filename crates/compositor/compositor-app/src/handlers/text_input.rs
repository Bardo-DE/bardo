//! `text_input` (P0-H7): text-input focus follows keyboard focus inside smithay's own
//! `KeyboardTarget` for surfaces; the reconciliation and the activation gate's functions are
//! `compositor_wayland::text_input`'s, called by `exec`. smithay's activation hook is not used:
//! the gate decides from the field's state when it reconciles.

use crate::state::State;
use smithay::wayland::text_input::TextInputActivation;

impl TextInputActivation for State {}
