//! What the manager's events mean to the app: `quit` (version 2, the person asked the whole app
//! to quit) is `HostEvent::QuitAsked`. Pure.

use desktop_protocols::client::bardo_window_v1::bardo_window_manager_v1;

use crate::event::{HostEvent, ShellEvent};

/// The host event for a manager event, or none for an event this host does not know.
pub(super) fn host_event(event: &bardo_window_manager_v1::Event) -> Option<ShellEvent> {
    match event {
        bardo_window_manager_v1::Event::Quit => Some(HostEvent::QuitAsked),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_quit_event_asks_the_app_to_quit() {
        assert_eq!(
            host_event(&bardo_window_manager_v1::Event::Quit),
            Some(HostEvent::QuitAsked)
        );
    }
}
