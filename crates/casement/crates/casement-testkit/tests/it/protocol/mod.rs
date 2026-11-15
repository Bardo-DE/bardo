//! One file per protocol module (`scripts/check-protocol-coverage.sh`):
//! bind every version, happy paths, every protocol error, destruction order, a transcript
//! golden (11 §3.2). Every test drives the real `State` through the `Fixture`. `sandbox` is
//! the one table that is not a protocol: the globals a sandboxed client never sees (I7).
mod alpha_modifier;
mod background_effect;
mod capture_cursor;
mod capture_dma;
mod capture_source;
mod color_management;
mod commit_timing;
mod compositor;
mod consent;
mod consent_store;
mod content_type;
mod cursor_shape;
mod data_control;
mod data_device;
mod dead_focus;
mod dmabuf;
mod fifo;
mod foreign_toplevel_list;
mod fractional_scale;
mod gamma;
mod geometry;
mod idle_inhibit;
mod idle_notify;
mod image_capture;
mod input_method;
mod layer_keyboard;
mod layer_shell;
mod output;
mod output_management;
mod output_power;
mod pointer_constraints;
mod pointer_gestures;
mod popup_grab;
mod popup_input;
mod popup_reuse;
mod presentation;
mod primary_selection;
mod quire_desktop;
mod quire_shell;
mod quire_shell_divider;
mod quire_shell_drop;
mod quire_shell_overview;
mod quire_shell_reveal;
mod quire_shell_spaces;
mod quire_shell_trial;
mod quire_window;
mod relative_pointer;
mod sandbox;
mod screencopy;
mod seat;
mod security_context;
mod session_lock;
mod shm;
mod shoot;
mod shortcuts_inhibit;
mod single_pixel;
mod subcompositor;
mod system_bell;
mod tablet;
mod text_input;
mod viewporter;
mod virtual_keyboard;
mod wlr_toplevel;
mod workspace;
mod xdg_activation;
mod xdg_decoration;
mod xdg_dialog;
mod xdg_foreign;
mod xdg_grab;
mod xdg_shell;
pub(crate) use layer_shell::map as map_layer;
pub(crate) use xdg_shell::{map_toplevel, rect_of, settle, windows};
mod xdg_toplevel_icon;
mod xdg_toplevel_tag;

/// The one shape of every "is cut" cap test: `ask` runs `cap` times and the client is still
/// connected after a pump, then once more and the compositor has cut it (the system backend
/// sends no error first, so the connection is simply closed).
pub(crate) fn cut_past_cap(
    fixture: &mut casement_testkit::Fixture,
    client: casement_testkit::ClientId,
    cap: usize,
    mut ask: impl FnMut(&mut casement_testkit::Fixture),
) {
    for _ in 0..cap {
        ask(fixture);
    }
    fixture.pump();
    assert!(
        fixture.client(client).is_connected(),
        "up to the cap is allowed"
    );
    ask(fixture);
    fixture.pump();
    assert!(
        !fixture.client(client).is_connected(),
        "the client past the cap is cut"
    );
}
