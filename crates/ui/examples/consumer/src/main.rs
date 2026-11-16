//! Runs the example on Blitz, through `ui_blitz::launch` (`../CONSUMING.md` section 1).
//! `cargo run` opens a window; `cargo test` (see `tests/coherence.rs`) never opens one.
//! `consumer::App` reads its own live settings at first render through
//! `ui_settings::use_environment` (`src/lib.rs::App`) — nothing to load here; `launch` enters
//! the Tokio runtime its watches run on.

fn main() -> Result<(), ui_blitz::LaunchError> {
    ui_blitz::launch(
        consumer::App,
        ui_blitz::AppConfig::new("quire consumer example", ui_blitz::WindowSize::new(480, 360)),
    )
}
