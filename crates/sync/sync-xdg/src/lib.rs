//! Where syncd reads and writes, resolved from an environment the caller hands in: the one place the daemon names a path, so a test can give it scratch directories.
//!
//! An account is named on disk by its object-path segment (`base_ids::object_segment`), the form
//! the account's directories are named in, so a wipe needs no map back to the id.

pub use base_xdg::PathError;
use base_xdg::Xdg;
use std::path::PathBuf;

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
    ///
    /// # Errors
    ///
    /// Returns a [`PathError`] when a state, config, data or cache folder, or the home folder,
    /// cannot be found from the environment.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

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
}
