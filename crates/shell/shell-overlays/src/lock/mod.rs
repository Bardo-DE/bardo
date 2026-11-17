//! bardo-shell's own lock screen (design/20 §1.9).
//!
//! One `ext-session-lock-v1` surface per output, which layer-shell-host creates when the daemon
//! locks (`crates/shell/shell-app/src/lock/`), never a layer surface. Each draws Bardo's
//! `LockScreen` stage in `Ds { material: Window, extent: Viewport }`: the output's pre-blurred
//! wallpaper crop ([`backdrop::backdrop_for`]), Bardo's `LockClock` ([`model::clock_lines`]) and,
//! on the output [`shell_ui_kit::links::LockLink`] names, Bardo's `LockPrompt`
//! ([`model::lock_user`], [`model::prompt_state`], [`model::caps_lock`]). Bardo's field holds the
//! text and hands it over only in `onsubmit`, which the surface wraps in a `Secret` at once and
//! sends as `AuthCommand::Submit`; it is never kept in a signal. Escape (Bardo's) empties the
//! field.
//!
//! When `Auth.phase` becomes `Accepted` every lock surface fades out (design/20 §1.9: `fade`
//! `--t-move --e-exit`, Bardo's motion timer) and the prompt's calls layer-shell-host's
//! `use_lock().unlock()` when its fade settles; the daemon unlocks as a backstop if that never
//! comes. A surface never decides a password itself.
//!
//! Keyboard: a lock surface always has it (the compositor gives it to one of them); the daemon
//! moves [`shell_ui_kit::links::LockLink`] to whichever lock surface gains focus, so the prompt is where typing goes.

mod backdrop;
mod body;
mod layer;
mod model;
mod surface;
mod view;

pub use surface::LOCK;
pub use view::LOCK_CSS;
