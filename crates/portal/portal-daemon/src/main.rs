//! The `bardo-portal` binary: parse nothing, build the real seams, serve until killed. A build with the
//! `debug` feature also reads `BARDO_PORTAL_DEV_CALLERS`, the person's own rows file.

use std::path::PathBuf;
use std::process::ExitCode;

use base_time::{Clock, SystemClock};
use base_xdg::Xdg;
use caller_identity_dbus::caller_identity_core::Table;
use caller_identity_dbus::caller_identity_linux::{RealProc, TableFiles};
use portal_daemon::access::ShellAsker;
use portal_daemon::capture::DisplayName;
use portal_daemon::link::BusLink;
use portal_daemon::names::AppNames;
use portal_daemon::screencast::restore::FileStore;
use portal_daemon::screenshot::{
    ExtOpen, LiveLayout, OutputGrab, ScreenshotSetup, ShellColorPicker,
};
use portal_daemon::settings::DesktopAppearance;
use portal_daemon::{Parts, serve};

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let callers = match with_person_rows(caller_table()) {
        Ok(callers) => callers,
        Err(reason) => {
            tracing::error!(%reason, "the portal could not start");
            return ExitCode::FAILURE;
        }
    };
    let parts = Parts::real(
        FileStore::new(state_file()),
        DesktopAppearance::at(appearance_file()),
    )
    .with_names(app_names())
    .with_callers(callers);
    let parts = {
        let setup = screenshots(parts.link(), parts.display());
        parts.with_screenshots(setup)
    };
    match serve(parts).await {
        Ok(_connection) => {
            std::future::pending::<()>().await;
            ExitCode::SUCCESS
        }
        Err(error) => {
            tracing::error!(%error, "the portal could not start");
            ExitCode::FAILURE
        }
    }
}

/// The root-owned table that says which program is the portal frontend and which is the shell. When
/// it cannot be loaded (missing, or writable by someone other than root) the table is empty: nobody
/// is recognised, every call is refused and the shell is never asked.
fn caller_table() -> Table {
    TableFiles::desktop()
        .load(&RealProc::system())
        .unwrap_or_else(|error| {
            tracing::error!(%error, "the table of the desktop's own programs could not be loaded, so every call is refused");
            Table::empty()
        })
}

/// The variable that names the person's own rows file, in a build with the `debug` feature.
#[cfg(feature = "debug")]
const DEV_CALLERS_VAR: &str = "BARDO_PORTAL_DEV_CALLERS";

/// `shipped`, with the rows of the file `BARDO_PORTAL_DEV_CALLERS` names added after it: callers they
/// name are let in as the portal frontend or the shell, below every real proof. Unset or empty adds
/// nothing. A file that cannot be taken stops the start (the reason says why), so a jail never runs
/// with stand-ins that are quietly refused.
#[cfg(feature = "debug")]
fn with_person_rows(shipped: Table) -> Result<Table, String> {
    let Some(path) = std::env::var_os(DEV_CALLERS_VAR)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
    else {
        return Ok(shipped);
    };
    let rows = portal_daemon::dev_rows::with_dev_rows(shipped, &path)
        .map_err(|error| format!("{DEV_CALLERS_VAR} {}: {error}", path.display()))?;
    tracing::warn!(file = %path.display(), "callers named in the person's own rows file are let in below every real proof");
    Ok(rows)
}

/// A release build has no rows file of the person's: `shipped` as it is.
#[cfg(not(feature = "debug"))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "the debug build's version of this function can fail, and the caller matches on both"
)]
fn with_person_rows(shipped: Table) -> Result<Table, String> {
    Ok(shipped)
}

/// Where the portal keeps its state: `$XDG_STATE_HOME/bardo/portal`, or under `$HOME/.local/state`.
/// The only place the environment is read for it; the libraries take the path.
fn state_dir() -> PathBuf {
    let base = Xdg::process()
        .state_home()
        .unwrap_or_else(|_| PathBuf::from("."));
    base.join("bardo").join("portal")
}

/// Where restore tokens are kept: `restore` in the state folder.
fn state_file() -> PathBuf {
    state_dir().join("restore")
}

/// Where an app's own name is found: the desktop entries in the data directories (the person's, the
/// flatpak exports, then the system's), preferring the person's language.
fn app_names() -> AppNames {
    let xdg = Xdg::process();
    let data_home = xdg.data_home().ok();
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(home) = data_home {
        dirs.push(home.join("flatpak/exports/share"));
        dirs.insert(0, home);
    }
    dirs.extend(xdg.data_dirs());
    dirs.push(PathBuf::from("/var/lib/flatpak/exports/share"));
    AppNames::new(dirs, locales())
}

/// The person's locales, most specific first: `zh_TW`, then `zh`. None for the plain C locale.
fn locales() -> Vec<String> {
    let chosen = ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|key| std::env::var(key).ok())
        .find(|value| !value.is_empty());
    let Some(value) = chosen else {
        return Vec::new();
    };
    let full = value.split(['.', '@']).next().unwrap_or("").to_owned();
    let language = full.split('_').next().unwrap_or("").to_owned();
    let mut found = vec![full];
    found.push(language);
    found.dedup();
    found.retain(|l| !l.is_empty() && l != "C" && l != "POSIX");
    found
}

/// Where the desktop keeps its appearance: `$XDG_CONFIG_HOME/bardo/appearance.toml`, or under
/// `$HOME/.config`.
fn appearance_file() -> PathBuf {
    let base = Xdg::process()
        .config_home()
        .unwrap_or_else(|_| PathBuf::from("."));
    base.join("bardo").join("appearance.toml")
}

/// Screenshots: the compositor the session's own display names (found the way Wayland clients
/// find theirs), files in `$HOME/Pictures`, named with the time the portal started. The outputs
/// are asked of the compositor at each screenshot, so a screen plugged in or out is accounted
/// for. Captures connect on `display`: a connection the compositor hands the portal first, the
/// session's own display when it hands none. The colour picker is the shell's.
fn screenshots(link: BusLink, display: DisplayName) -> ScreenshotSetup {
    let pictures = Xdg::process()
        .home()
        .map(|home| home.join("Pictures"))
        .unwrap_or_default();
    let stamp = SystemClock.now().0.to_string();
    ScreenshotSetup::new(
        OutputGrab::new(display, Box::new(LiveLayout), Box::new(ExtOpen)),
        pictures,
        stamp,
    )
    .with_picker(Box::new(ShellColorPicker::new(link.clone())))
    .with_asker(Box::new(ShellAsker::new(link)))
}
