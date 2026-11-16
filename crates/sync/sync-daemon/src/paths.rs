//! Where syncd reads and writes, resolved from the environment (PLAN §2.3): the one place the
//! daemon names a path, so a test can give it scratch directories. An account is named on disk
//! by its object-path segment (`base_ids::object_segment`), the form the account's
//! directories are named in, so a wipe needs no map back to the id.

pub use base_xdg::PathError;
use base_xdg::Xdg;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Every path the daemon uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    /// `$XDG_STATE_HOME/bardo/sync`: one directory per account, one journal per dataset.
    pub journals: PathBuf,
    /// `$XDG_DATA_HOME/bardo/sync/vdir`: the PIM mirrors, one directory per account (W6e).
    pub mirrors: PathBuf,
    /// The system caller table, `/etc/bardo/accountd-callers.toml` (another file in a test build:
    /// [`account_callers::SystemCallers`]).
    pub callers_system: PathBuf,
    /// The user's caller table, `$XDG_CONFIG_HOME/bardo/accounts/callers.toml`; its rows win.
    pub callers_user: PathBuf,
    /// `~/Cloud`: where each drive the person shows in Files is a folder (`~/Cloud/OneDrive`).
    pub cloud: PathBuf,
    /// `$XDG_CACHE_HOME/bardo/sync/drives`: each drive's downloaded files, one directory per account.
    pub drive_cache: PathBuf,
    /// `$XDG_CONFIG_HOME/bardo/sync/syncd.toml`: syncd's own settings (`storage.cache_limit`),
    /// in a folder of its own so that syncd's unit may write it and nothing else of the
    /// configuration (accountd's caller table sits in `bardo/accounts/`).
    pub settings: PathBuf,
}

impl Paths {
    /// The paths for the environment `var` reads.
    pub fn resolve(var: impl Fn(&str) -> Option<String>) -> Result<Self, PathError> {
        let xdg = Xdg::new(var);
        let state = xdg.dir("XDG_STATE_HOME", ".local/state")?;
        let config = xdg.dir("XDG_CONFIG_HOME", ".config")?;
        let data = xdg.dir("XDG_DATA_HOME", ".local/share")?;
        let cache = xdg.dir("XDG_CACHE_HOME", ".cache")?;
        let home = xdg.home().ok_or(PathError)?.to_path_buf();
        Ok(Self {
            journals: state.join("bardo/sync"),
            mirrors: data.join("bardo/sync/vdir"),
            callers_system: PathBuf::from(account_callers::SYSTEM_CALLERS),
            callers_user: config.join("bardo/accounts/callers.toml"),
            cloud: home.join("Cloud"),
            drive_cache: cache.join("bardo/sync/drives"),
            settings: config.join("bardo/sync/syncd.toml"),
        })
    }

    /// The journal file of one dataset of one account.
    #[must_use]
    pub fn journal(&self, account: &AccountDir, dataset: &str) -> PathBuf {
        self.journals
            .join(account.as_str())
            .join(format!("{dataset}.sqlite"))
    }

    /// The Photos library of one account, `$XDG_DATA_HOME/bardo/sync/photos/<account>` (W6f): beside
    /// the PIM mirrors, so `AccountRemoved` wipes it with them.
    #[must_use]
    pub fn photos_dir(&self, account: &AccountDir) -> PathBuf {
        self.mirrors
            .parent()
            .unwrap_or(&self.mirrors)
            .join("photos")
            .join(account.as_str())
    }

    /// The app folder mirror of one account, `$XDG_DATA_HOME/bardo/sync/storage/<account>`: beside
    /// the PIM mirrors, so `AccountRemoved` wipes it with them.
    #[must_use]
    pub fn storage_dir(&self, account: &AccountDir) -> PathBuf {
        self.mirrors
            .parent()
            .unwrap_or(&self.mirrors)
            .join("storage")
            .join(account.as_str())
    }

    /// The folder whose files syncd uploads to the account's Google Photos,
    /// `$XDG_DATA_HOME/bardo/sync/photos/<account>/upload`: inside the Photos library directory, so
    /// `AccountRemoved` wipes it with the rest.
    #[must_use]
    pub fn photos_upload_dir(&self, account: &AccountDir) -> PathBuf {
        self.photos_dir(account).join("upload")
    }

    /// Where the Google Photos picker puts what the person picked,
    /// `$XDG_DATA_HOME/bardo/sync/photos/<account>/picked`; one folder per session below it.
    #[must_use]
    pub fn photos_picked_dir(&self, account: &AccountDir) -> PathBuf {
        self.photos_dir(account).join("picked")
    }

    /// The ledger of what was uploaded to Google Photos (the album and every content hash sent),
    /// beside the account's journals so it goes with them.
    #[must_use]
    pub fn photos_ledger(&self, account: &AccountDir) -> PathBuf {
        self.journals
            .join(account.as_str())
            .join("google_photos_upload.ledger.json")
    }

    /// The downloaded files of one account's drive, `$XDG_CACHE_HOME/bardo/sync/drives/<account>`.
    #[must_use]
    pub fn drive_cache_dir(&self, account: &AccountDir) -> PathBuf {
        self.drive_cache.join(account.as_str())
    }

    /// Everything of one account that syncd keeps: its journals and its mirrors.
    #[must_use]
    pub fn account_dirs(&self, account: &AccountDir) -> [PathBuf; 2] {
        [
            self.journals.join(account.as_str()),
            self.mirrors.join(account.as_str()),
        ]
    }
}

/// An account as a directory name: the `[A-Za-z0-9_]+` segment of its object path.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AccountDir(String);

impl AccountDir {
    /// The segment, if it is one: nothing else may become a path component.
    #[must_use]
    pub fn parse(segment: &str) -> Option<Self> {
        let ok = !segment.is_empty()
            && segment.len() <= 64
            && segment
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_');
        ok.then(|| Self(segment.to_owned()))
    }

    /// The directory name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AccountDir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Which build this is, for the one knob a release must not have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Build {
    /// Built with the `test-proc-root` feature.
    Test,
    /// Every other build: the shipped one.
    Release,
}

/// The build this binary is.
pub const BUILD: Build = if cfg!(feature = "test-proc-root") {
    Build::Test
} else {
    Build::Release
};

/// Whether this binary honours `BARDO_SYNCD_PROC_ROOT`: only with the `test-proc-root` feature. A
/// release build ignores the variable.
pub const HONOURS_PROC_ROOT: bool = cfg!(feature = "test-proc-root");

/// How often the supervisors read grants again (`BARDO_SYNCD_RESCAN_S`, whole seconds, at least 1),
/// for a test build only, so a scenario need not wait the default ten minutes for a grant it
/// just made. A release build ignores the variable; so does a value that is not a number of
/// seconds. The default is `default`, and no build has a shorter one.
pub fn rescan(build: Build, value: Option<String>, default: Duration) -> Duration {
    match build {
        Build::Test => value
            .and_then(|v| v.trim().parse::<u64>().ok())
            .filter(|seconds| *seconds >= 1)
            .map_or(default, Duration::from_secs),
        Build::Release => default,
    }
}

/// Temporary: delete with the old-folders crate in the release after everyone has started once.
///
/// Moves what syncd kept under its pre-bardo folders into the ones [`Paths::resolve`] names, once,
/// before anything is read: the journals (`$XDG_STATE_HOME/porter/sync` and
/// `$XDG_STATE_HOME/quire/syncd`), the mirrors, app folders and photo libraries
/// (`$XDG_DATA_HOME/porter/{vdir,storage,photos}`), the drives' cache
/// (`$XDG_CACHE_HOME/porter/drives`) and its settings file (`$XDG_CONFIG_HOME/porter/syncd`).
/// What moved is said on standard error.
///
/// # Errors
/// There is no home to resolve the folders in.
pub fn move_pre_bardo_folders(var: impl Fn(&str) -> Option<String>) -> Result<(), PathError> {
    let xdg = Xdg::new(var);
    move_under(
        &xdg.dir("XDG_STATE_HOME", ".local/state")?,
        &xdg.dir("XDG_CONFIG_HOME", ".config")?,
        &xdg.dir("XDG_DATA_HOME", ".local/share")?,
        &xdg.dir("XDG_CACHE_HOME", ".cache")?,
    );
    Ok(())
}

/// [`move_pre_bardo_folders`] over the four base directories.
fn move_under(state: &Path, config: &Path, data: &Path, cache: &Path) {
    let folders = [
        (state.join("porter/sync"), state.join("bardo/sync")),
        (state.join("quire/syncd"), state.join("bardo/sync")),
        (data.join("porter/vdir"), data.join("bardo/sync/vdir")),
        (data.join("porter/storage"), data.join("bardo/sync/storage")),
        (data.join("porter/photos"), data.join("bardo/sync/photos")),
        (cache.join("porter/drives"), cache.join("bardo/sync/drives")),
    ];
    for (old, new) in &folders {
        match old_folders::move_once(old, new) {
            Ok(done) if done.moved > 0 => eprintln!(
                "syncd: moved {} from {} to {}",
                done.moved,
                old.display(),
                new.display()
            ),
            Ok(_) => {}
            Err(why) => eprintln!(
                "syncd: could not move {} to {}: {why}",
                old.display(),
                new.display()
            ),
        }
    }
    let (old, new) = (
        config.join("porter/syncd/syncd.toml"),
        config.join("bardo/sync/syncd.toml"),
    );
    match old_folders::move_file_once(&old, &new) {
        Ok(true) => eprintln!("syncd: moved {} to {}", old.display(), new.display()),
        Ok(false) => {}
        Err(why) => eprintln!(
            "syncd: could not move {} to {}: {why}",
            old.display(),
            new.display()
        ),
    }
    // What is left empty is not worth keeping; a folder with anything in it stays.
    for old in [
        config.join("porter/syncd"),
        state.join("quire"),
        data.join("porter"),
        cache.join("porter"),
    ] {
        let _ = std::fs::remove_dir(old);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn the_pre_bardo_folders_move_once_and_accountds_files_stay() {
        let scratch = base_testkit::Scratch::new("syncd-old-folders");
        let (state, config, data, cache) = (
            scratch.join("state"),
            scratch.join("config"),
            scratch.join("data"),
            scratch.join("cache"),
        );
        let write = |path: PathBuf| {
            std::fs::create_dir_all(path.parent().expect("parent")).expect("dir");
            std::fs::write(path, "x").expect("file");
        };
        write(state.join("porter/sync/a1/files.sqlite"));
        write(state.join("porter/registry.json"));
        write(state.join("quire/syncd/old.json"));
        write(data.join("porter/vdir/a1/cal/one.ics"));
        write(data.join("porter/providers/mine.toml"));
        write(cache.join("porter/drives/a1/file"));
        write(config.join("porter/syncd/syncd.toml"));
        write(config.join("porter/callers.toml"));
        move_under(&state, &config, &data, &cache);
        move_under(&state, &config, &data, &cache);
        for new in [
            state.join("bardo/sync/a1/files.sqlite"),
            state.join("bardo/sync/old.json"),
            data.join("bardo/sync/vdir/a1/cal/one.ics"),
            cache.join("bardo/sync/drives/a1/file"),
            config.join("bardo/sync/syncd.toml"),
            state.join("porter/registry.json"),
            data.join("porter/providers/mine.toml"),
            config.join("porter/callers.toml"),
        ] {
            assert!(new.exists(), "{}", new.display());
        }
        assert!(!state.join("porter/sync").exists());
        assert!(!state.join("quire").exists());
        assert!(!config.join("porter/syncd").exists());
        assert!(!cache.join("porter").exists());
    }

    #[test]
    fn the_rescan_variable_is_read_by_a_test_build_only_and_never_shortens_the_default() {
        let default = Duration::from_secs(600);
        let cases = [
            (Build::Test, Some("2"), Duration::from_secs(2)),
            (Build::Test, Some(" 5 "), Duration::from_secs(5)),
            (Build::Test, Some("0"), default),
            (Build::Test, Some("soon"), default),
            (Build::Test, Some("-3"), default),
            (Build::Test, None, default),
            (Build::Release, Some("2"), default),
            (Build::Release, None, default),
        ];
        for (build, value, want) in cases {
            assert_eq!(
                rescan(build, value.map(str::to_owned), default),
                want,
                "{build:?} {value:?}"
            );
        }
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
    fn home_alone_gives_the_xdg_defaults() {
        let paths = Paths::resolve(env(&[("HOME", "/home/ada")])).expect("paths");
        assert_eq!(
            paths.journals,
            Path::new("/home/ada/.local/state/bardo/sync")
        );
        assert_eq!(
            paths.mirrors,
            Path::new("/home/ada/.local/share/bardo/sync/vdir")
        );
        assert_eq!(
            paths.callers_user,
            Path::new("/home/ada/.config/bardo/accounts/callers.toml")
        );
    }

    #[test]
    fn xdg_variables_win_and_relative_ones_are_ignored() {
        let paths = Paths::resolve(env(&[
            ("HOME", "/home/ada"),
            ("XDG_STATE_HOME", "/scratch/state"),
            ("XDG_DATA_HOME", "relative/data"),
        ]))
        .expect("paths");
        assert_eq!(paths.journals, Path::new("/scratch/state/bardo/sync"));
        assert_eq!(
            paths.mirrors,
            Path::new("/home/ada/.local/share/bardo/sync/vdir")
        );
        assert_eq!(Paths::resolve(env(&[])), Err(PathError));
    }

    #[test]
    fn a_photos_library_lives_beside_the_vdir_under_its_account() {
        let paths = Paths::resolve(env(&[("HOME", "/h")])).expect("paths");
        let account = AccountDir::parse("a1").expect("segment");
        assert_eq!(
            paths.photos_dir(&account),
            Path::new("/h/.local/share/bardo/sync/photos/a1")
        );
    }

    #[test]
    fn a_journal_lives_under_its_account_and_a_wipe_names_journals_and_mirrors() {
        let paths = Paths::resolve(env(&[("HOME", "/h")])).expect("paths");
        let account = AccountDir::parse("67e55044_10b1").expect("segment");
        assert_eq!(
            paths.journal(&account, "files"),
            Path::new("/h/.local/state/bardo/sync/67e55044_10b1/files.sqlite")
        );
        let [journals, mirrors] = paths.account_dirs(&account);
        assert_eq!(
            journals,
            Path::new("/h/.local/state/bardo/sync/67e55044_10b1")
        );
        assert_eq!(
            mirrors,
            Path::new("/h/.local/share/bardo/sync/vdir/67e55044_10b1")
        );
    }

    #[test]
    fn only_a_plain_segment_becomes_a_directory_name() {
        const CASES: &[(&str, bool)] = &[
            ("abc_123", true),
            ("", false),
            ("..", false),
            ("a/b", false),
            ("a-b", false),
            ("a.b", false),
        ];
        for (text, ok) in CASES {
            assert_eq!(AccountDir::parse(text).is_some(), *ok, "{text:?}");
        }
    }

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
