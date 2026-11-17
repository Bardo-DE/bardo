//! The caller table as files, and porter's caller mapped onto almanac's.
//!
//! The identification is porter's (`account_callers::ProcCallers`: the bus names the pid, the pid's
//! cgroup names the unit or the app scope), and so is the reading of the table files
//! (`account_callers::load_callers`). What is memoryd's own is where they are —
//! `/etc/bardo/memoryd-callers.toml` with the user's `<config>/bardo/memoryd/callers.toml` laid
//! over it — and [`caller_for`] that turns a porter role into one of almanac's callers. The router, the shell, cuad and the Settings app are the four fixed
//! callers; every other identified process is an app.

use caller_role_types::CallerRole;
use memory_service::Caller;

/// The app that is intentd, the action router (its row has the `agent` role).
pub const ROUTER_APP: &str = "org.bardo.Intents";
/// The app that is the shell, sill (its row has the `sheet_host` role).
pub const SHELL_APP: &str = "org.bardo.Shell";
/// The app that is the System Settings app, the Settings app (its row has the `settings` role).
pub const SETTINGS_APP: &str = "org.bardo.Settings";

/// almanac's caller for the process porter identified.
///
/// Cuad is the `cua` role, the shell the `sheet_host` role of [`SHELL_APP`], the router the `agent`
/// role of [`ROUTER_APP`], the Settings app the `settings` role of [`SETTINGS_APP`]; any other row,
/// or an app scope or Flatpak scope the table does not name, is that app. The fixed callers need
/// the role as well as the app, so a row cannot make an app the router by naming it.
#[must_use]
pub fn caller_for(found: account_callers::Caller) -> Caller {
    let name = found.app.name.as_str();
    let role = match found.role {
        CallerRole::Cua => CallerRole::Cua,
        CallerRole::SheetHost if name == SHELL_APP => CallerRole::SheetHost,
        CallerRole::Agent if name == ROUTER_APP => CallerRole::Agent,
        CallerRole::Settings if name == SETTINGS_APP => CallerRole::Settings,
        // Every other process is an app with its own access, whatever its role: a terminal
        // (temor) is no app and no fixed caller, and gets no shell or router standing; the
        // compositor, the porter daemons, the AI broker and the agent launcher likewise.
        _ => CallerRole::App,
    };
    Caller::new(role, found.app)
}
