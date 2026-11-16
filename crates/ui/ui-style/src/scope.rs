//! What every component under a `Ds` can read: the resolved appearance, the material and the
//! blur state of the scope it is in.

use crate::appearance::material::Material;
use crate::appearance::{resolve::Resolved, theme::Scheme};
use dioxus::prelude::*;

/// The enclosing scope, as `Ds` and `Surface` provide it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Scope {
    /// The root's resolved scheme, accent and motion level.
    pub resolved: Resolved,
    /// The scheme of the nearest `Surface` that overrides it, else the root's.
    pub scheme: Scheme,
    /// The nearest material.
    pub material: Material,
    /// Whether the compositor blurs behind this surface.
    pub blur: bool,
    /// Whether the person last drove the surface with the keyboard (focus rings show); `false`
    /// after a pointer.
    pub modality: bool,
    /// Whether the window is the one the person is working in.
    pub activity: bool,
}

/// The enclosing scope. Panics outside a `Ds`: every quire component is drawn inside one.
#[must_use]
pub fn use_scope() -> Scope {
    use_scope_signal()()
}

/// The enclosing scope as the signal `Ds` and `Surface` provide, for hooks that read the
/// motion level when a timer starts rather than when they were created.
#[must_use]
pub fn use_scope_signal() -> Signal<Scope> {
    use_context::<Signal<Scope>>()
}

/// Provide `env` to the subtree, updating the provided value when it changes. Nothing here
/// reads the signal, so the write does not re-render the caller.
#[must_use]
pub fn use_scope_provider(env: Scope) -> Signal<Scope> {
    let mut provided = use_context_provider(|| Signal::new(env));
    if *provided.peek() != env {
        provided.set(env);
    }
    provided
}
