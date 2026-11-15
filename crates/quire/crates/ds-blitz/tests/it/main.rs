//! The crate's integration tests: one executable, one module per former test file.
//! A separate test target needs a stated reason.
//! Separate targets, each with its reason: `pdf_thumb` and `pdf_thumb_queue` share the process-wide
//! PDF worker queue and its cache with each other and would count each other's rasters;
//! `spell_worker` and `spell_edit` start the process-wide spell service on their own dictionaries.
//! All four also have `required-features` in Cargo.toml.

mod cancelled_animation;
mod caret_same_frame;
mod cascade_layer;
mod cjk_line_break;
mod clipboard;
mod colour_emoji;
mod css_filter;
mod edit_format_bar_focus;
mod edit_stacking;
mod edit_surface;
mod edit_surface_pointer;
mod file_drop;
mod focus;
mod frames;
mod hit_absolute_in_padded_parent;
mod hit_outside_empty_parent;
mod ime_texture_layer;
mod net_policy;
mod open_window_harness;
mod pdf_app;
mod pdf_output;
mod pdf_raster;
mod pixel_snap;
#[cfg(feature = "debug-probe")]
mod probe_snapshot;
mod provide_host;
mod registered_property_animation;
mod resolved_token;
mod root_contexts;
mod selection_same_frame;
mod strip_hit_area;
mod styles_guard;
mod support;
mod text_stack_fonts;
mod user_style_reload;
mod window_scroll;
