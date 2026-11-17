//! Bridges layer-shell-host's raw seat modality and surface scale into the UI kit's `ui_components::prelude::HostSignals`, the
//! root context `Ds` reads to stamp `data-modality` (and so decide whether the focus ring shows)
//! and to size the pixel tokens. layer-shell-host cannot name the UI kit's `ds` crate
//! (`scripts/check-boundary.sh`), so bardo-shell republishes: [`use_host_signals`] reads layer-shell-host's
//! `use_input_modality()` (one value per seat, shared by every surface) and `use_surface_scale()`
//! and provides them as `ui_components::prelude::HostSignals` with a `use_effect` (layer-shell-host README, "Consuming:
//! input modality (focus rings)"). It runs once per document root: in `mount` and `mount_lock`
//! for a surface and in `PopupFrame` for a popup, so every `Ds` root has the same keyboard-only
//! focus ring and one scale, and no surface calls it by hand. The seat's window activity is not
//! reported to a surface, so the window is always active.
//!
//! A headless render with no `SurfaceHandle` (a `ui_harness::Harness` test) has no seat to read,
//! so the hook provides nothing: `Ds` then reads whatever `HostSignals` the harness itself
//! provides at its root (its own seat, driven by its `key`/`click`), or falls back to `Pointer`
//! and 1x with none at all. Whether the handle exists never changes over a component's life, so
//! the hooks run in the same order on every render.

use dioxus::prelude::*;
use layer_shell_hooks::SurfaceHandle;
use layer_shell_types::{InputModality as SeatModality, Scale120};
use ui_components::prelude::*;

/// layer-shell-host's raw modality, as the UI kit's own: whether the person last used the keyboard.
fn to_ds(modality: SeatModality) -> bool {
    matches!(modality, SeatModality::Keyboard)
}

/// Provides `HostSignals` to this component and everything under it.
///
/// The signals are `ui_components::prelude::HostSignals`, kept live from layer-shell-host's
/// per-seat [`layer_shell_hooks::use_input_modality`] and the surface's
/// [`layer_shell_hooks::use_surface_scale`]. Nothing is provided on a component with no
/// `SurfaceHandle`.
pub fn use_host_signals() {
    let hosted = use_hook(try_consume_context::<SurfaceHandle>);
    if hosted.is_some() {
        use_republished(
            layer_shell_hooks::use_input_modality(),
            layer_shell_hooks::use_surface_scale(),
        );
    }
}

/// The seat's `modality` and the surface's `scale`, provided as `ui_components::prelude::HostSignals`.
pub(super) fn use_republished(modality: ReadSignal<SeatModality>, scale: ReadSignal<Scale120>) {
    let signals = use_context_provider(|| HostSignals {
        modality: Signal::new(to_ds(*modality.peek())),
        scale: Signal::new(Scale(scale.peek().0)),
        activity: Signal::new(true),
    });
    use_effect(move || {
        let (mut provided, next) = (signals.modality, to_ds(modality()));
        if *provided.peek() != next {
            provided.set(next);
        }
    });
    use_effect(move || {
        let (mut provided, next) = (signals.scale, Scale(scale().0));
        if *provided.peek() != next {
            provided.set(next);
        }
    });
}

/// The scale `Ds` sizes this surface's pixel tokens by: the host's, or 1x with none. For what a
/// surface draws outside `Ds` at device pixels (its icon rasters).
pub fn use_host_scale() -> Scale {
    use_hook(try_consume_context::<HostSignals>).map_or(Scale::ONE, |signals| (signals.scale)())
}

#[cfg(test)]
#[path = "signals_tests.rs"]
mod tests;
