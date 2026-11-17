//! compositor-core's integration tests: ids, kinds, the model and its machines, focus, input, output and the wire round trips.
//!
//! One integration binary: every former `tests/*.rs` is a module here, and `support` is shared.

mod bardo;
mod display_trial;
mod focus;
mod focus_beyond;
mod focus_props;
mod focus_spaces;
mod frame;
mod gestures;
mod globals;
mod gpu;
mod ids;
mod input;
mod model;
mod model_apps;
mod model_drop;
mod model_input;
mod model_layers;
mod model_lock;
mod model_overview;
mod model_props;
mod model_requests;
mod model_shell;
mod model_spaces;
mod model_split;
mod model_window_menu;
mod model_windows;
mod model_wiring;
mod output;
mod output_request;
mod privilege_decide;
mod privilege_step;
mod round_trip;
mod space_persist;
mod support;
mod tile_drag;
mod window;
