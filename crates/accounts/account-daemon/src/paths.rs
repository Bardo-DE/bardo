//! Where accountd reads and writes, resolved from the environment (PLAN §2.3, §2.6): the one
//! place the daemon names a path, so a test can give it scratch directories.

use account_families::set::Layer;
pub use base_xdg::PathError;
use base_xdg::{Xdg, absolute};
use std::path::{Path, PathBuf};

/// Every path the daemon uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    /// The directory of `registry.json`: `$XDG_STATE_HOME/bardo/accounts`.
    pub registry_dir: PathBuf,
    /// `$XDG_STATE_HOME/bardo/accounts/audit.jsonl`.
    pub audit: PathBuf,
    /// The system caller table, `/etc/bardo/accountd-callers.toml` (another file in a test build:
    /// [`account_callers::SystemCallers`]).
    pub callers_system: PathBuf,
    /// The user's caller table, `$XDG_CONFIG_HOME/bardo/accounts/callers.toml`; its rows win.
    pub callers_user: PathBuf,
    /// The shipped clients: `bardo/accounts/clients.toml` under the first data directory that has it.
    pub clients_shipped: PathBuf,
    /// The user's clients, `$XDG_CONFIG_HOME/bardo/accounts/clients.toml` (Settings writes it).
    pub clients_user: PathBuf,
    /// Provider file directories, later ones winning: the system's (under the first
    /// data directory that has `bardo/accounts/providers`), then the user's.
    pub provider_dirs: Vec<PathBuf>,
    /// The `applications` directories that hold desktop entries, the person's first:
    /// `$XDG_DATA_HOME/applications`, then each absolute `$XDG_DATA_DIRS` entry's (default
    /// `/usr/local/share:/usr/share`).
    pub applications: Vec<PathBuf>,
    /// Tailscale's socket: `/var/run/tailscale/tailscaled.sock`, or the absolute path in
    /// `BARDO_ACCOUNTD_TAILSCALE_SOCKET` (a Tailscale kept elsewhere, and every test of the binary,
    /// which names a scratch path so that none ever reaches the real Tailscale).
    pub tailscale_socket: PathBuf,
    /// The directories of `PATH`, where Tailscale's program is looked for besides the usual
    /// ones (to tell "not installed" from "not running"); empty when `PATH` is not set.
    pub program_path: std::ffi::OsString,
}

/// The variable that names Tailscale's socket.
pub const TAILSCALE_SOCKET_VAR: &str = "BARDO_ACCOUNTD_TAILSCALE_SOCKET";

impl Paths {
    /// The paths for the environment `var` reads, with `extra_providers` laid over the system's
    /// and the user's provider directories.
    pub fn resolve(
        var: impl Fn(&str) -> Option<String>,
        extra_providers: &[PathBuf],
    ) -> Result<Self, PathError> {
        // Like the XDG rule: a relative path is invalid and ignored.
        let tailscale_socket = absolute(var(TAILSCALE_SOCKET_VAR))
            .unwrap_or_else(|| PathBuf::from(account_tailscale::DEFAULT_SOCKET));
        let program_path = var("PATH").unwrap_or_default().into();
        let xdg = Xdg::new(var);
        let state = xdg.dir("XDG_STATE_HOME", ".local/state")?;
        let config = xdg.dir("XDG_CONFIG_HOME", ".config")?;
        let data = xdg.dir("XDG_DATA_HOME", ".local/share")?;
        let mut provider_dirs = vec![
            xdg.shipped("bardo/accounts/providers"),
            data.join("bardo/accounts/providers"),
        ];
        provider_dirs.extend_from_slice(extra_providers);
        let applications = std::iter::once(data.clone())
            .chain(xdg.data_dirs())
            .map(|dir| dir.join("applications"))
            .collect();
        Ok(Self {
            registry_dir: state.join("bardo/accounts"),
            audit: state.join("bardo/accounts/audit.jsonl"),
            callers_system: PathBuf::from(account_callers::SYSTEM_CALLERS),
            callers_user: config.join("bardo/accounts/callers.toml"),
            clients_shipped: xdg.shipped("bardo/accounts/clients.toml"),
            clients_user: config.join("bardo/accounts/clients.toml"),
            provider_dirs,
            applications,
            tailscale_socket,
            program_path,
        })
    }

    /// [`Paths::provider_dirs`] with whose each is, as `resolve` laid them out: the system's,
    /// the person's, then the ones named on the command line.
    pub fn provider_layers(&self) -> Vec<(Layer, PathBuf)> {
        self.provider_dirs
            .iter()
            .enumerate()
            .map(|(at, dir)| {
                let layer = match at {
                    0 => Layer::Shipped,
                    1 => Layer::Person,
                    _ => Layer::Named,
                };
                (layer, dir.clone())
            })
            .collect()
    }

    /// The clients files the OAuth families read: the shipped one and the one Settings writes.
    pub fn client_files(&self) -> account_family_oauth::ClientFiles {
        account_family_oauth::ClientFiles {
            shipped: self.clients_shipped.clone(),
            own: self.clients_user.clone(),
        }
    }
}

/// Temporary: delete with the old-folders crate in the release after everyone has started once.
///
/// Moves what accountd kept under its pre-bardo folders into the ones [`Paths::resolve`] names,
/// once, before anything is read: the registry and its neighbours (`$XDG_STATE_HOME/porter`, but
/// not syncd's `sync` folder there), the audit file (`$XDG_STATE_HOME/quire/accountd`), the caller
/// table and the clients file (`$XDG_CONFIG_HOME/porter`) and the person's provider files
/// (`$XDG_DATA_HOME/porter/providers`). What moved is said on standard error.
///
/// # Errors
/// There is no home to resolve the folders in.
pub fn move_pre_bardo_folders(var: impl Fn(&str) -> Option<String>) -> Result<(), PathError> {
    let xdg = Xdg::new(var);
    move_under(
        &xdg.dir("XDG_STATE_HOME", ".local/state")?,
        &xdg.dir("XDG_CONFIG_HOME", ".config")?,
        &xdg.dir("XDG_DATA_HOME", ".local/share")?,
    );
    Ok(())
}

/// [`move_pre_bardo_folders`] over the three base directories.
fn move_under(state: &Path, config: &Path, data: &Path) {
    let new_state = state.join("bardo/accounts");
    // The registry's folder is shared with syncd's journals (`sync`), which are syncd's to move.
    let old_state = state.join("porter");
    if let Ok(entries) = std::fs::read_dir(&old_state) {
        let names: Vec<_> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.file_name())
            .filter(|name| name != "sync")
            .collect();
        for name in names {
            let (old, new) = (old_state.join(&name), new_state.join(&name));
            if old.is_dir() {
                moved(
                    &old,
                    &new,
                    old_folders::move_once(&old, &new).map(|m| m.moved),
                );
            } else {
                moved(
                    &old,
                    &new,
                    old_folders::move_file_once(&old, &new).map(usize::from),
                );
            }
        }
        let _ = std::fs::remove_dir(&old_state);
    }
    let audit = state.join("quire/accountd");
    moved(
        &audit,
        &new_state,
        old_folders::move_once(&audit, &new_state).map(|m| m.moved),
    );
    let _ = std::fs::remove_dir(state.join("quire"));
    for file in ["callers.toml", "clients.toml"] {
        let (old, new) = (
            config.join("porter").join(file),
            config.join("bardo/accounts").join(file),
        );
        moved(
            &old,
            &new,
            old_folders::move_file_once(&old, &new).map(usize::from),
        );
    }
    let providers = data.join("porter/providers");
    let new = data.join("bardo/accounts/providers");
    moved(
        &providers,
        &new,
        old_folders::move_once(&providers, &new).map(|m| m.moved),
    );
    let _ = std::fs::remove_dir(data.join("porter"));
}

/// Says on standard error what a move did.
fn moved(old: &Path, new: &Path, done: std::io::Result<usize>) {
    match done {
        Ok(0) => {}
        Ok(count) => eprintln!(
            "accountd: moved {count} from {} to {}",
            old.display(),
            new.display()
        ),
        Err(why) => eprintln!(
            "accountd: could not move {} to {}: {why}",
            old.display(),
            new.display()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn the_pre_bardo_folders_move_once_and_syncds_journals_stay() {
        let scratch = base_testkit::Scratch::new("accountd-old-folders");
        let (state, config, data) = (
            scratch.join("state"),
            scratch.join("config"),
            scratch.join("data"),
        );
        let write = |path: PathBuf| {
            std::fs::create_dir_all(path.parent().expect("parent")).expect("dir");
            std::fs::write(path, "x").expect("file");
        };
        write(state.join("porter/registry.json"));
        write(state.join("porter/sync/a1/files.sqlite"));
        write(state.join("quire/accountd/audit.jsonl"));
        write(config.join("porter/clients.toml"));
        write(config.join("porter/syncd/syncd.toml"));
        write(data.join("porter/providers/mine.toml"));
        move_under(&state, &config, &data);
        move_under(&state, &config, &data);
        for new in [
            state.join("bardo/accounts/registry.json"),
            state.join("bardo/accounts/audit.jsonl"),
            config.join("bardo/accounts/clients.toml"),
            data.join("bardo/accounts/providers/mine.toml"),
            state.join("porter/sync/a1/files.sqlite"),
            config.join("porter/syncd/syncd.toml"),
        ] {
            assert!(new.exists(), "{}", new.display());
        }
        assert!(!state.join("porter/registry.json").exists());
        assert!(!state.join("quire").exists());
        assert!(!config.join("porter/clients.toml").exists());
        assert!(!data.join("porter").exists());
    }

    #[test]
    fn the_provider_layers_are_the_system_the_person_then_the_named() {
        let paths =
            Paths::resolve(env(&[("HOME", "/home/ada")]), &[PathBuf::from("/x")]).expect("paths");
        let layers: Vec<Layer> = paths
            .provider_layers()
            .into_iter()
            .map(|(l, _)| l)
            .collect();
        assert_eq!(layers, [Layer::Shipped, Layer::Person, Layer::Named]);
    }

    fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| {
            pairs
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| (*v).to_owned())
        }
    }

    #[test]
    fn tailscales_socket_is_its_own_path_unless_an_absolute_one_is_named() {
        let socket = |pairs: &[(&str, &str)]| {
            Paths::resolve(env(pairs), &[])
                .expect("paths")
                .tailscale_socket
        };
        let home = ("HOME", "/home/ada");
        assert_eq!(
            socket(&[home]),
            Path::new("/var/run/tailscale/tailscaled.sock")
        );
        assert_eq!(
            socket(&[home, (TAILSCALE_SOCKET_VAR, "/run/ts/other.sock")]),
            Path::new("/run/ts/other.sock")
        );
        // A relative path is ignored, as an XDG one is.
        assert_eq!(
            socket(&[home, (TAILSCALE_SOCKET_VAR, "ts.sock")]),
            Path::new("/var/run/tailscale/tailscaled.sock")
        );
    }

    #[test]
    fn home_alone_gives_the_xdg_defaults() {
        // A data directory that holds nothing, so the porter installed on this machine does not
        // decide where the system's providers are.
        let paths = Paths::resolve(
            env(&[("HOME", "/home/ada"), ("XDG_DATA_DIRS", "/scratch/share")]),
            &[],
        )
        .expect("paths");
        assert_eq!(
            paths.registry_dir,
            Path::new("/home/ada/.local/state/bardo/accounts")
        );
        assert_eq!(
            paths.audit,
            Path::new("/home/ada/.local/state/bardo/accounts/audit.jsonl")
        );
        assert_eq!(
            paths.callers_user,
            Path::new("/home/ada/.config/bardo/accounts/callers.toml")
        );
        assert_eq!(
            paths.provider_dirs,
            vec![
                PathBuf::from("/usr/share/bardo/accounts/providers"),
                PathBuf::from("/home/ada/.local/share/bardo/accounts/providers")
            ]
        );
    }

    #[test]
    fn the_families_read_the_clients_file_settings_writes() {
        let paths = Paths::resolve(
            env(&[
                ("HOME", "/home/ada"),
                ("XDG_CONFIG_HOME", "/scratch/config"),
            ]),
            &[],
        )
        .expect("paths");
        let files = paths.client_files();
        assert_eq!(files.own, paths.clients_user);
        assert_eq!(
            files.own,
            Path::new("/scratch/config/bardo/accounts/clients.toml")
        );
        assert_eq!(files.shipped, paths.clients_shipped);
    }

    #[test]
    fn xdg_variables_win_and_relative_ones_are_ignored() {
        let paths = Paths::resolve(
            env(&[
                ("HOME", "/home/ada"),
                ("XDG_STATE_HOME", "/scratch/state"),
                ("XDG_CONFIG_HOME", "relative/config"),
            ]),
            &[PathBuf::from("/extra")],
        )
        .expect("paths");
        assert_eq!(
            paths.registry_dir,
            Path::new("/scratch/state/bardo/accounts")
        );
        assert_eq!(
            paths.clients_user,
            Path::new("/home/ada/.config/bardo/accounts/clients.toml")
        );
        assert_eq!(paths.provider_dirs.last(), Some(&PathBuf::from("/extra")));
    }

    #[test]
    fn desktop_entries_are_looked_for_in_the_data_home_then_the_data_dirs() {
        let defaults = Paths::resolve(env(&[("HOME", "/home/ada")]), &[]).expect("paths");
        assert_eq!(
            defaults.applications,
            [
                "/home/ada/.local/share/applications",
                "/usr/local/share/applications",
                "/usr/share/applications"
            ]
            .map(PathBuf::from)
        );
        let set = Paths::resolve(
            env(&[
                ("HOME", "/home/ada"),
                ("XDG_DATA_HOME", "/scratch/data"),
                ("XDG_DATA_DIRS", "/one:relative:/two"),
            ]),
            &[],
        )
        .expect("paths");
        assert_eq!(
            set.applications,
            [
                "/scratch/data/applications",
                "/one/applications",
                "/two/applications"
            ]
            .map(PathBuf::from)
        );
    }

    #[test]
    fn no_home_and_no_xdg_is_an_error() {
        assert_eq!(Paths::resolve(env(&[]), &[]), Err(PathError));
        assert!(
            Paths::resolve(
                env(&[
                    ("XDG_STATE_HOME", "/s"),
                    ("XDG_CONFIG_HOME", "/c"),
                    ("XDG_DATA_HOME", "/d")
                ]),
                &[]
            )
            .is_ok()
        );
    }
}

/// Whether this binary honours `BARDO_ACCOUNTD_PROC_ROOT`: only with the `test-proc-root` feature (the
/// jailed and acceptance tests run processes outside service cgroups). A release build ignores
/// the variable.
pub const HONOURS_PROC_ROOT: bool = cfg!(feature = "test-proc-root");

#[cfg(test)]
mod proc_root_tests {
    #[test]
    fn a_shipped_build_is_a_release_build() {
        let manifest = include_str!("../Cargo.toml");
        let default = manifest
            .lines()
            .skip_while(|l| !l.starts_with("[features]"))
            .any(|l| l.trim_start().starts_with("default"));
        assert!(
            !default,
            "no feature is default, so test-proc-root never is"
        );
    }
}
