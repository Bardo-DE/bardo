//! Effectful services behind watch channels: power, network, Wi-Fi, Bluetooth, power profiles,
//! Now Playing, the session, the lock screen's password check and the polkit agent, notifications,
//! calendar events, screenshots, volume, tray, apps, launching, dock pins, wallpaper, displays,
//! the windows and workspaces and the key remap daemon keyboard daemon.
//!
//! Every service is one module with the same shape: a marker type (in `shell-model`, beside its
//! state) that this crate implements [`service::Run`] for, started by [`service::spawn`] against
//! [`env::Env`]'s seams into a `shell_model::service::Handle` (a watch receiver of its state and a
//! `Control` for its commands), and a `backend` that carries its effects. Its state, commands and the pure `step` that is the only place the state changes
//! live in `shell-model`, so every transition is table-tested without a bus or a socket. Shared
//! data travels on `tokio::sync::watch`, never as a Dioxus signal.
//!
//! This crate never reaches `dioxus` (`cargo xtask check layers`): the hooks that read these
//! channels live in `shell-ui-kit::hooks`.
#![expect(
    missing_docs,
    reason = "lints baseline: this crate documents its public items when its domain is refactored (plan phases 3-7)"
)]

mod account_label;
pub mod accounts;
pub mod app_menu;
pub mod apps;
pub mod auth;
pub mod badges;
pub mod battery;
pub mod bluetooth;
mod call_error;
pub mod callers;
pub mod capture_pick;
pub mod clipboard;
pub mod clock;
pub mod companion;
pub mod deadline;
mod debounced;
pub mod desktop_wl;
pub mod displays;
pub mod durable;
pub mod env;
pub mod error;
pub mod events;
pub mod first_run;
pub mod guest_asks;
pub mod idle;
pub mod input_source;
pub mod intents;
pub mod kbd_backlight;
pub mod key_remap;
pub mod launch;
pub mod locker;
pub mod login_session;
mod machine;
pub mod network;
pub mod night_shift;
mod notices;
pub mod notifications;
pub mod now_playing;
mod owner;
pub mod pins;
pub mod polkit_agent;
pub mod power_profile;
mod restart;
pub mod screenshots;
pub mod seat;
mod serve;
pub mod service;
pub mod session;
mod stream;
pub mod supervise;
pub mod sync_hold;
pub mod tray;
pub mod voice;
pub mod volume;
pub mod wallpaper;
mod wayland_thread;
pub mod wifi;
pub mod work_areas;
pub mod workspace_looks;

pub use accounts::AccountsLinks;
pub use app_menu::{AppMenuDeps, MenuLinks, MenuNameOf};
pub use companion::CompanionLinks;
pub use desktop_wl::WindowsLinks;
pub use error::ServiceError;
pub use idle::IdleLinks;
pub use intents::{IntentsLinks, ShellCall};
pub use key_remap::KeyRemapLinks;
pub use notifications::Answers;
pub use session::SessionLinks;
pub use voice::VoiceLinks;
