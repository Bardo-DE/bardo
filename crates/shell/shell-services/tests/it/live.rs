//! The tests that touch the person's running login: its systemd user manager, the nested
//! compositor and a dbus-daemon, and PAM with a throwaway account (`cargo xtask live session`).

mod auth_pam;
mod clipboard_live;
mod idle_bus;
mod launch_scope;
