//! A request to close a window, which the app may refuse (GTK's `close-request`, `AppKit`'s
//! `windowShouldClose:`).
//!
//! The person closes a window with its frame's close button or the compositor's own control. The
//! event loop does not drop the window then: it asks the window's close handler, registered with
//! [`use_close_request`], and drops it when the handler answers `true`. With no handler it closes, as
//! it always did. An app with unsaved work answers `false`, asks the person in its
//! own UI, and closes the window through [`WindowHandle::close`](crate::WindowHandle::close)
//! once they have answered; that call, [`AppHandle::quit`](crate::AppHandle::quit) and the
//! `LastWindowClosed` policy are the app's own doing and are never put to the handler.

use dioxus::core::{Runtime, current_scope_id};
use dioxus::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// What the event loop calls with the window's runtime entered.
type Handler = Rc<dyn Fn() -> bool>;

/// The component's newest handler, which the registered one calls.
type Latest = Box<dyn FnMut() -> bool>;

/// One window's close handler: the document's components set it, the event loop asks it.
#[derive(Clone, Default)]
pub(crate) struct CloseGuard(Rc<RefCell<Option<Handler>>>);

impl CloseGuard {
    /// Take `handler` as the window's, in place of any before it.
    fn set(&self, handler: Handler) {
        self.0.replace(Some(handler));
    }

    /// Forget `handler`, unless another has taken its place since.
    fn clear(&self, handler: &Handler) {
        let mut held = self.0.borrow_mut();
        if held.as_ref().is_some_and(|held| Rc::ptr_eq(held, handler)) {
            *held = None;
        }
    }

    /// Whether the window closes on a close request: its handler's answer, or `true` when it has
    /// none. Called from the event loop, outside any component.
    pub(crate) fn ask(&self) -> bool {
        let handler = self.0.borrow().clone();
        handler.is_none_or(|handler| handler())
    }
}

/// Decide what the calling component's window does when the person asks to close it (its frame's
/// close button or the compositor's close). Answer `true` to close the window now, or `false` to
/// leave it open while the handler deals with the request itself (asking the person to save,
/// say); close the window yourself with [`WindowHandle::close`](crate::WindowHandle::close) when
/// the person agrees. One handler per window: the component that registered last holds it, and it goes when
/// that component unmounts. `handler` is called from the event loop, so it can write signals but
/// must not wait. Does nothing outside a window `launch` runs (the harness, a snapshot).
pub fn use_close_request(handler: impl FnMut() -> bool + 'static) {
    let Some(guard) = use_hook(try_consume_context::<CloseGuard>) else {
        return;
    };
    let latest: Rc<RefCell<Latest>> = use_hook(|| {
        let first: Latest = Box::new(|| true);
        Rc::new(RefCell::new(first))
    });
    // Each render hands over the handler that sees the component's newest props.
    *latest.borrow_mut() = Box::new(handler);
    let registered: Handler = use_hook(|| {
        let runtime = Runtime::current();
        let scope = current_scope_id();
        let handler: Handler =
            Rc::new(move || runtime.in_scope(scope, || (*latest.borrow_mut())()));
        guard.set(Rc::clone(&handler));
        handler
    });
    use_drop(move || guard.clear(&registered));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    /// What a probe component registered with.
    #[derive(Clone)]
    struct Asked(Rc<Cell<u32>>, Rc<Cell<bool>>);

    #[allow(non_snake_case)]
    fn Guarded() -> Element {
        let Asked(asked, answer) = use_context::<Asked>();
        use_close_request(move || {
            asked.set(asked.get() + 1);
            answer.get()
        });
        rsx! {}
    }

    #[test]
    fn a_window_closes_unless_its_handler_keeps_it() {
        let guard = CloseGuard::default();
        assert!(guard.ask(), "no handler, no veto");

        let asked = Rc::new(Cell::new(0));
        let answer = Rc::new(Cell::new(false));
        let mut vdom = VirtualDom::new(Guarded);
        vdom.provide_root_context(guard.clone());
        vdom.provide_root_context(Asked(Rc::clone(&asked), Rc::clone(&answer)));
        vdom.rebuild_in_place();

        assert!(!guard.ask());
        answer.set(true);
        assert!(guard.ask());
        assert_eq!(asked.get(), 2, "each request reached the handler");

        drop(vdom);
        assert!(guard.ask());
        assert_eq!(asked.get(), 2, "a handler is forgotten with its component");
    }
}
