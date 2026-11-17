//! `org.bardo.SettingsModule1` at `/org/bardo/Sync1/settings`, served through quire's `settings_dbus`: the accounts the person has (accountd's) and what syncd keeps of them.
//!
//! Only the `Settings` role may `Get`, `Describe` or `Set`, as in accountd's module.
//!
//! One group of rows per account accountd lists: `accounts.<id>.label`, `.provider` and `.state`
//! (read-outs), `.sync.files` and `.sync.photos` (what syncd keeps of the account, shown for what
//! it offers; the app folder's `.sync.files` is hidden once the account shows in Files and that
//! switch is off, its folder left where it is), `.storage.files` ("Show in Files", for an account whose provider asks the whole
//! drive on use), `.storage.keep` ("Keep Everything Downloaded", only while Show in Files is on;
//! kept in `syncd.toml`, with a line under it when the drive does not fit this computer's free
//! space) and `.remove`. Turning "Show in Files" on may open
//! the provider's page, so it answers at once and the row says how it went afterwards: on, or
//! off with a line under it when the person did not give the access. One more row, `accounts.sync.add`, opens
//! accountd's add-account sheet; the switch for a new account turns on only once it was added, and
//! accountd's consent sheet asks the first time. Removing an account here deletes its journals and
//! mirrors and takes back what syncd was allowed; the account stays in accountd, and accountd's
//! own removal of it does not touch syncd's copies. One more row, `storage.cache_limit` ("Downloaded Files Limit", an Advanced
//! text row of bytes, "0" for automatic), is how much the drives shown in Files keep of what is
//! not kept downloaded; it is written to `syncd.toml` and every drive follows it at once. No key
//! is agent-settable.

use crate::datasets::drive::CacheKnob;
use crate::keep::{
    ContinueError, Keep, Snapshot, SwitchError, SyncClass, drive_allowed, sync_allowed, sync_offers,
};
use account_callers::{CallerRole, Callers};
use account_client::{AccountsLink, GrantsLink};
use account_core::AccountId;
use account_core::{Account, Family};
use account_wire::ParentWindow;
use drive_core::CacheLimit;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use toml_edit::{Array, DocumentMut, Item};
use settings_dbus::{Access, Caller, LiveError, LiveModule, LiveSchema, Served, Verdict, serve};
use settings_types::schema::{ActionLabel, ActionWeight, AgentSetting, Exposure, Help, KeyKind, KeyPath, KeySpec, Label, LiveAction, Note, Page, Section, WordLabels};
use zbus::Connection;

/// The settings module's path.
pub const SETTINGS_PATH: &str = "/org/bardo/Sync1/settings";

/// What a key names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Key {
    /// `accounts.sync.add`: "Add Account", accountd's sheet.
    Add,
    /// `accounts.<id>.label`.
    Label(AccountId),
    /// `accounts.<id>.provider`.
    Provider(AccountId),
    /// `accounts.<id>.state`.
    State(AccountId),
    /// `accounts.<id>.sync.files` and `.sync.photos`.
    Sync(AccountId, SyncClass),
    /// `accounts.<id>.storage.files`: "Show in Files".
    Files(AccountId),
    /// `accounts.<id>.storage.keep`: "Keep Everything Downloaded".
    KeepAll(AccountId),
    /// `accounts.<id>.remove`.
    Remove(AccountId),
    /// `storage.cache_limit`.
    CacheLimit,
}

const ADD: &str = "accounts.sync.add";

/// The cache limit's row.
pub const CACHE_LIMIT: &str = "storage.cache_limit";

/// The cache limit's label and help line.
const CACHE_LIMIT_WORDS: (&str, &str) = (
    "Downloaded Files Limit",
    "Files you haven't kept downloaded are removed, oldest first, past this size.",
);

/// The key `path` names, among the accounts of `snapshot` (an id may hold dots, so the account is
/// found by prefix, the longest id first).
pub(crate) fn parse(path: &str, snapshot: &Snapshot) -> Option<Key> {
    if path == ADD {
        return Some(Key::Add);
    }
    if path == CACHE_LIMIT {
        return Some(Key::CacheLimit);
    }
    let rest = path.strip_prefix("accounts.")?;
    let mut ids: Vec<&AccountId> = snapshot.accounts.iter().map(|a| &a.account.id).collect();
    ids.sort_by_key(|id| std::cmp::Reverse(id.as_str().len()));
    ids.into_iter().find_map(|id| {
        let tail = rest.strip_prefix(id.as_str())?.strip_prefix('.')?;
        let id = id.clone();
        match tail {
            "label" => Some(Key::Label(id)),
            "provider" => Some(Key::Provider(id)),
            "state" => Some(Key::State(id)),
            "remove" => Some(Key::Remove(id)),
            "storage.files" => Some(Key::Files(id)),
            "storage.keep" => Some(Key::KeepAll(id)),
            _ => {
                let what = tail.strip_prefix("sync.")?;
                SyncClass::ALL
                    .into_iter()
                    .find(|c| c.slug() == what)
                    .map(|c| Key::Sync(id, c))
            }
        }
    })
}

/// The path of `key`.
pub(crate) fn path(key: &Key) -> String {
    match key {
        Key::Add => ADD.to_owned(),
        Key::CacheLimit => CACHE_LIMIT.to_owned(),
        Key::Label(id) => format!("accounts.{id}.label"),
        Key::Provider(id) => format!("accounts.{id}.provider"),
        Key::State(id) => format!("accounts.{id}.state"),
        Key::Sync(id, class) => format!("accounts.{id}.sync.{}", class.slug()),
        Key::Files(id) => format!("accounts.{id}.storage.files"),
        Key::KeepAll(id) => format!("accounts.{id}.storage.keep"),
        Key::Remove(id) => format!("accounts.{id}.remove"),
    }
}

fn spec(
    key: &Key,
    section: &str,
    label: &str,
    help: &str,
    kind: KeyKind,
    default: toml::Value,
) -> KeySpec {
    KeySpec {
        control: None,
        path: KeyPath(path(key)),
        kind,
        default,
        label: Label(label.to_owned()),
        help: Help(help.to_owned()),
        page: Page::Accounts,
        section: Section(section.to_owned()),
        exposure: Exposure::Basic,
        labels: WordLabels::default(),
        unavailable: BTreeMap::default(),
        groups: BTreeMap::default(),
        places: BTreeMap::default(),
        note: None,
        ends: None,
        agent: AgentSetting::HandsOff,
    }
}

fn action(name: &str, weight: ActionWeight) -> KeyKind {
    KeyKind::Live {
        action: LiveAction {
            label: ActionLabel(name.to_owned()),
            weight,
        },
    }
}

fn readout(key: &Key, section: &str, label: &str, word: &str) -> KeySpec {
    spec(
        key,
        section,
        label,
        "",
        KeyKind::Fixed {
            variant: word.to_owned(),
        },
        toml::Value::String(word.to_owned()),
    )
}

fn off() -> toml::Value {
    toml::Value::Boolean(false)
}

/// What a switch says about a class, in plain words.
fn sync_words(account: &Account, class: SyncClass) -> (&'static str, &'static str) {
    match class {
        SyncClass::Files => (
            "Keep this account's files on this computer",
            "Lets the sync service keep a copy of this account's app folder here.",
        ),
        // Google Photos is not a backup of a library: it is an upload folder, and what the
        // person picks in Google's own window. The row says so.
        SyncClass::Photos
            if account
                .endpoints
                .iter()
                .any(|e| e.family == Family::GooglePhotosUpload) =>
        {
            (
                "Upload new photos from a folder on this computer to Google Photos",
                "Sends each new photo you put in this account's upload folder, a folder on this computer, to an album made for it in Google Photos. It never reads or deletes anything else in your Google Photos.",
            )
        }
        SyncClass::Photos => (
            "Back up photos",
            "Lets the sync service keep this account's photos here and back them up.",
        ),
    }
}

/// "Show in Files": its label and help line.
const FILES_WORDS: (&str, &str) = (
    "Show in Files",
    "See this account's files in Files. They download when you open them.",
);

/// The line under "Show in Files" when turning it on did not end in it on.
pub const FILES_NOT_ON: &str = "Files wasn't turned on. You can try again.";

/// "Keep Everything Downloaded": its label and help line.
const KEEP_ALL_WORDS: (&str, &str) = (
    "Keep Everything Downloaded",
    "Downloads all of this account's files and keeps them on this computer, so they open without a connection.",
);

/// How an account's rows are drawn beyond what accountd says of it: whether "Show in Files" is offered,
/// whether its last turning on did not go through, whether it is on (which offers "Keep
/// Everything Downloaded" and hides the app folder's switch once that is off), whether keeping
/// everything does not fit the free space, and whether the app folder's mirror is on.
#[derive(Debug, Clone, Copy, Default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "five independent facts the Files row tells apart, each read by its name"
)]
pub(crate) struct FilesRow {
    pub offered: bool,
    pub not_on: bool,
    pub shown: bool,
    pub short_of_room: bool,
    pub app_folder: bool,
}

impl FilesRow {
    /// Whether the app folder's switch ("Keep this account's files on this computer") is offered:
    /// while the account is not shown in Files, or while its mirror still runs, so it can be
    /// turned off. Once it is off and the account is shown in Files it stays hidden; its folder
    /// stays where it is.
    fn offers_app_folder(self) -> bool {
        !self.shown || self.app_folder
    }
}

fn account_keys(account: &Account, provider_label: Option<&str>, files: FilesRow) -> Vec<KeySpec> {
    let section = account.label.0.as_str();
    let id = &account.id;
    let provider = provider_label.unwrap_or_else(|| account.provider.as_str());
    let mut provider_row = readout(
        &Key::Provider(id.clone()),
        section,
        "Account type",
        account.provider.as_str(),
    );
    provider_row.labels = WordLabels(
        [(account.provider.as_str().to_owned(), provider.to_owned())]
            .into_iter()
            .collect(),
    );
    let mut keys = vec![
        readout(
            &Key::Label(id.clone()),
            section,
            "Account",
            &account.label.0,
        ),
        provider_row,
        readout(
            &Key::State(id.clone()),
            section,
            "Status",
            base_ids::word::word(&account.state),
        ),
    ];
    for class in sync_offers(account) {
        if class == SyncClass::Files && !files.offers_app_folder() {
            continue;
        }
        let (label, help) = sync_words(account, class);
        keys.push(spec(
            &Key::Sync(id.clone(), class),
            section,
            label,
            help,
            KeyKind::Toggle,
            toml::Value::Boolean(false),
        ));
    }
    if files.offered {
        let (label, help) = FILES_WORDS;
        let mut row = spec(
            &Key::Files(id.clone()),
            section,
            label,
            help,
            KeyKind::Toggle,
            toml::Value::Boolean(false),
        );
        row.note = files.not_on.then(|| Note(FILES_NOT_ON.to_owned()));
        keys.push(row);
    }
    if files.offered && files.shown {
        let (label, help) = KEEP_ALL_WORDS;
        let mut row = spec(
            &Key::KeepAll(id.clone()),
            section,
            label,
            help,
            KeyKind::Toggle,
            toml::Value::Boolean(false),
        );
        row.note = files
            .short_of_room
            .then(|| Note(crate::words::reason(drive_core::Reason::NoRoomForAll).to_owned()));
        keys.push(row);
    }
    keys.push(spec(
        &Key::Remove(id.clone()),
        section,
        "Stop syncing",
        "Stops syncing this account and deletes its copies on this computer. The account itself stays in Internet Accounts.",
        action("Stop Syncing", ActionWeight::Destructive),
        off(),
    ));
    keys
}

/// The schema of the module for `snapshot`, each account's "Show in Files" row as `files` says.
pub(crate) fn schema(snapshot: &Snapshot, files: impl Fn(&Account) -> FilesRow) -> LiveSchema {
    let mut key = vec![spec(
        &Key::Add,
        "Sync",
        "Add an account to sync",
        "Sign in to another account. Sync keeps its files here once you allow it.",
        action("Add Account", ActionWeight::Plain),
        off(),
    )];
    key.extend(snapshot.accounts.iter().flat_map(|listed| {
        let account = &listed.account;
        account_keys(account, listed.provider_label.as_deref(), files(account))
    }));
    let (label, help) = CACHE_LIMIT_WORDS;
    let mut limit = spec(
        &Key::CacheLimit,
        "Files",
        label,
        help,
        KeyKind::Text,
        toml::Value::String("0".to_owned()),
    );
    limit.exposure = Exposure::Advanced;
    key.push(limit);
    LiveSchema { version: 1, key }
}

/// How an action ended, as the word its row changes to.
fn word_of(outcome: &Result<(), ContinueError>) -> &'static str {
    match outcome {
        Ok(()) => "done",
        Err(ContinueError::Cancelled) => "cancelled",
        Err(_) => "failed",
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "takes anything printable by value, as its callers pass a message, a `format!` or an error value"
)]
fn failed(why: impl ToString) -> LiveError {
    LiveError::Failed(why.to_string())
}

fn toggle_of(value: &toml::Value) -> Option<bool> {
    value.as_bool()
}

/// The module over the accounts and what syncd keeps of them.
pub struct SyncSettings<T, C> {
    keep: Arc<Keep<T>>,
    callers: Arc<C>,
    served: Arc<OnceLock<Served>>,
    /// The accounts whose "Show in Files" was last turned on without it ending on.
    not_on: Arc<Mutex<BTreeSet<AccountId>>>,
    /// The cache limit the drives follow, and the file it is kept in.
    cache: Option<(CacheKnob, PathBuf)>,
    /// What the last removal kept of what was not uploaded, said under the first row.
    kept: Arc<Mutex<Option<String>>>,
}

impl<T, C> std::fmt::Debug for SyncSettings<T, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyncSettings").finish_non_exhaustive()
    }
}

impl<T, C> SyncSettings<T, C> {
    /// The module over `keep`, deciding who may use it through `callers`.
    pub fn new(keep: Arc<Keep<T>>, callers: Arc<C>) -> Self {
        Self {
            keep,
            callers,
            served: Arc::new(OnceLock::new()),
            not_on: Arc::default(),
            cache: None,
            kept: Arc::default(),
        }
    }

    /// With the cache limit row: setting it writes `file` and moves `knob`, which the drives
    /// follow.
    #[must_use]
    pub fn with_cache_limit(mut self, knob: CacheKnob, file: PathBuf) -> Self {
        self.cache = Some((knob, file));
        self
    }
}

/// The text of the settings file `file`, when there is one.
fn settings_text(file: &Path) -> Option<String> {
    std::fs::read_to_string(file).ok()
}

/// The cache limit `file` keeps (`[storage] cache_limit = "<bytes>"`); automatic when there is
/// no file, or it says nothing this build reads.
pub fn read_cache_limit(file: &Path) -> CacheLimit {
    let Some(text) = settings_text(file) else {
        return CacheLimit::Automatic;
    };
    let Ok(table) = text.parse::<toml::Table>() else {
        return CacheLimit::Automatic;
    };
    table
        .get("storage")
        .and_then(|storage| storage.get("cache_limit"))
        .and_then(bytes_of)
        .map_or(CacheLimit::Automatic, CacheLimit::from_setting)
}

/// The drives `file` keeps paused (`[storage] paused = ["<drive id>", ...]`).
#[must_use]
pub fn read_paused(file: &Path) -> BTreeSet<String> {
    read_drives(file, "paused")
}

/// Writes to `file` that the drive `id` is paused, or not, keeping the rest of it.
///
/// # Errors
///
/// Returns the I/O error when the file cannot be read, parsed or written.
pub fn write_paused(file: &Path, id: &str, paused: bool) -> std::io::Result<()> {
    write_drives(file, "paused", id, paused)
}

/// The drives `file` keeps downloaded whole (`[storage] keep_all = ["<drive id>", ...]`).
#[must_use]
pub fn read_keep_all(file: &Path) -> BTreeSet<String> {
    read_drives(file, "keep_all")
}

/// Writes to `file` that the drive `id` is kept downloaded whole, or not, keeping the rest of
/// it.
///
/// # Errors
///
/// Returns the I/O error when the file cannot be read, parsed or written.
pub fn write_keep_all(file: &Path, id: &str, on: bool) -> std::io::Result<()> {
    write_drives(file, "keep_all", id, on)
}

/// The drive ids `file` lists under `[storage] <list>`.
fn read_drives(file: &Path, list: &str) -> BTreeSet<String> {
    settings_text(file)
        .and_then(|text| text.parse::<toml::Table>().ok())
        .and_then(|table| {
            let ids = table.get("storage")?.get(list)?.as_array()?.clone();
            Some(
                ids.iter()
                    .filter_map(|id| id.as_str().map(str::to_owned))
                    .collect(),
            )
        })
        .unwrap_or_default()
}

/// Writes to `file` that the drive `id` is in `[storage] <list>`, or not, keeping the rest of it.
fn write_drives(file: &Path, list: &str, id: &str, listed: bool) -> std::io::Result<()> {
    edit_storage(file, |storage| {
        let mut ids: BTreeSet<String> = storage
            .get(list)
            .and_then(Item::as_array)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| id.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        if listed {
            ids.insert(id.to_owned())
        } else {
            ids.remove(id)
        };
        storage.insert(list, toml_edit::value(ids.into_iter().collect::<Array>()));
    })
}

/// Edits the `[storage]` table of `file` and nothing else: the read, the edit and the rename
/// happen under the file's lock (`base_fs::update_atomic`, the one every writer of the shared
/// settings file takes), so another writer's key is never put back old, and every other
/// section, key and comment keeps the text it had. A file that exists and does not parse as
/// TOML (or whose `storage` is not a table) is left untouched and the error says so.
fn edit_storage(
    file: &Path,
    edit: impl FnOnce(&mut dyn toml_edit::TableLike),
) -> std::io::Result<()> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    base_fs::update_atomic(
        file,
        base_fs::ReadCap::mib(1),
        base_fs::NewFileMode::OwnerOnly,
        |text| -> Result<String, std::io::Error> {
            let mut doc: DocumentMut = text.unwrap_or_default().parse().map_err(|why| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("{} is not valid TOML, left as it is: {why}", file.display()),
                )
            })?;
            let storage = doc
                .as_table_mut()
                .entry("storage")
                .or_insert_with(|| {
                    let mut made = toml_edit::Table::new();
                    made.set_implicit(true);
                    Item::Table(made)
                })
                .as_table_like_mut()
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!(
                            "{}: `storage` is not a table, left as it is",
                            file.display()
                        ),
                    )
                })?;
            edit(storage);
            Ok(doc.to_string())
        },
    )
    .map_err(|why| match why {
        base_fs::UpdateError::Edit(why) => why,
        other => std::io::Error::other(other),
    })
}

/// A drive's folder as people see it: each is a folder of `~/Cloud` ([`crate::paths::Paths`]).
fn shown_folder(path: &Path) -> String {
    match path.file_name() {
        Some(name) => format!("~/Cloud/{}", name.to_string_lossy()),
        None => path.display().to_string(),
    }
}

/// Writes `limit` to `file`, keeping the rest of it.
fn write_cache_limit(file: &Path, limit: CacheLimit) -> std::io::Result<()> {
    edit_storage(file, |storage| {
        storage.insert("cache_limit", toml_edit::value(limit.setting().to_string()));
    })
}

/// Bytes as the row takes them: text of digits (or a whole number).
fn bytes_of(value: &toml::Value) -> Option<u64> {
    match value {
        toml::Value::String(text) => text.trim().parse().ok(),
        toml::Value::Integer(n) => u64::try_from(*n).ok(),
        _ => None,
    }
}

/// Tells the pane what changed: `Changed(<key>, <word>)`. The pane reads the schema again on any
/// `Changed`, so a new or removed account shows by the news of its state row.
async fn announce(served: &OnceLock<Served>, key: &Key, value: impl Into<toml::Value>) {
    if let Some(served) = served.get() {
        let _ = served.changed(&KeyPath(path(key)), &value.into()).await;
    }
}

impl<T: GrantsLink + AccountsLink, C: Callers> LiveModule for SyncSettings<T, C> {
    async fn permit(&self, caller: &Caller, _access: Access) -> Verdict {
        match self.callers.caller_of(&caller.sender.0).await {
            Some(who) if who.role == CallerRole::Settings => Verdict::Allow,
            _ => Verdict::Refuse,
        }
    }

    async fn describe(&self) -> LiveSchema {
        let not_on = self
            .not_on
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        // accountd could not be asked: no account is known, so none is drawn.
        let snapshot = self.keep.snapshot().await.unwrap_or_default();
        let mut rows = BTreeMap::new();
        for listed in &snapshot.accounts {
            let account = &listed.account;
            let row = FilesRow {
                offered: self.keep.offers_files(account).await,
                not_on: not_on.contains(&account.id),
                shown: drive_allowed(&snapshot.grants, &account.id),
                short_of_room: self.keep.short_of_room(&account.id),
                app_folder: sync_allowed(&snapshot.grants, &account.id, SyncClass::Files),
            };
            rows.insert(account.id.clone(), row);
        }
        let mut schema = schema(&snapshot, |account| {
            rows.get(&account.id).copied().unwrap_or_default()
        });
        let kept = self
            .kept
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        if let (Some(kept), Some(first)) = (kept, schema.key.first_mut()) {
            first.note = Some(Note(kept));
        }
        schema
    }

    async fn get(&self, key: &KeyPath) -> Result<toml::Value, LiveError> {
        let snapshot = self.read().await?;
        let unknown = || LiveError::UnknownKey(key.0.clone());
        let account = |id: &AccountId| snapshot.account(id).ok_or_else(unknown);
        let text = |word: &str| Ok(toml::Value::String(word.to_owned()));
        match parse(&key.0, &snapshot).ok_or_else(unknown)? {
            Key::Label(id) => text(&account(&id)?.label.0),
            Key::Provider(id) => text(account(&id)?.provider.as_str()),
            Key::State(id) => text(base_ids::word::word(&account(&id)?.state)),
            Key::Sync(id, class) => {
                account(&id)?;
                Ok(toml::Value::Boolean(sync_allowed(
                    &snapshot.grants,
                    &id,
                    class,
                )))
            }
            Key::Files(id) => {
                account(&id)?;
                Ok(toml::Value::Boolean(drive_allowed(&snapshot.grants, &id)))
            }
            Key::KeepAll(id) => {
                account(&id)?;
                Ok(toml::Value::Boolean(self.keep.keeps_everything(&id)))
            }
            Key::Add | Key::Remove(_) => Ok(off()),
            Key::CacheLimit => {
                let limit = self
                    .cache
                    .as_ref()
                    .map_or(CacheLimit::Automatic, |(knob, _)| *knob.borrow());
                text(&limit.setting().to_string())
            }
        }
    }

    async fn set(&self, key: &KeyPath, value: toml::Value) -> Result<(), LiveError> {
        let snapshot = self.read().await?;
        let parsed =
            parse(&key.0, &snapshot).ok_or_else(|| LiveError::UnknownKey(key.0.clone()))?;
        match parsed {
            Key::Add => {
                let (keep, served) = (Arc::clone(&self.keep), Arc::clone(&self.served));
                tokio::spawn(async move {
                    let before = keep.snapshot().await.map(|now| now.accounts.len());
                    let outcome = keep.add_and_keep(&ParentWindow::Unparented).await.map(drop);
                    if let Err(why) = &outcome {
                        eprintln!("syncd: add account: {why}");
                    }
                    announce(&served, &Key::Add, word_of(&outcome)).await;
                    // A new account appears with its state, which a pane reads the schema for.
                    if let (Ok(before), Ok(now)) = (before, keep.snapshot().await)
                        && now.accounts.len() != before
                        && let Some(listed) = now.accounts.last()
                    {
                        announce(&served, &Key::State(listed.account.id.clone()), "ok").await;
                    }
                });
            }
            Key::Sync(id, class) => {
                let toggle = toggle_of(&value)
                    .ok_or_else(|| LiveError::BadValue("a sync row is `true` or `false`".into()))?;
                // The app folder's switch, hidden once the drive is shown in Files, is not
                // turned on again.
                if class == SyncClass::Files
                    && toggle
                    && drive_allowed(&snapshot.grants, &id)
                    && !sync_allowed(&snapshot.grants, &id, class)
                {
                    return Err(LiveError::NotPermitted(
                        "this account's files are in Files".into(),
                    ));
                }
                self.keep
                    .set_sync(&id, class, toggle)
                    .await
                    .map_err(|why| {
                        eprintln!("syncd: a Settings change was refused: {why}");
                        failed("That could not be done.")
                    })?;
            }
            Key::Files(id) => {
                let toggle = toggle_of(&value)
                    .ok_or_else(|| LiveError::BadValue("this row is `true` or `false`".into()))?;
                self.set_files(&snapshot, id, toggle).await?;
            }
            Key::KeepAll(id) => {
                let toggle = toggle_of(&value)
                    .ok_or_else(|| LiveError::BadValue("this row is `true` or `false`".into()))?;
                self.keep
                    .keep_everything(&id, toggle)
                    .await
                    .map_err(|why| match why {
                        SwitchError::NothingToKeep => {
                            LiveError::NotPermitted("this account isn't shown in Files".into())
                        }
                        why => {
                            eprintln!("syncd: a Settings change was refused: {why}");
                            failed("That could not be saved.")
                        }
                    })?;
                announce(&self.served, &Key::KeepAll(id), toggle).await;
            }
            Key::Remove(id) => {
                let kept = self.keep.remove(&id).await.map_err(failed)?;
                let words: Vec<String> = kept
                    .iter()
                    .filter(|kept| kept.count > 0)
                    .map(|kept| crate::words::kept_unsent(kept.count, &shown_folder(&kept.path)))
                    .collect();
                *self.kept.lock().unwrap_or_else(PoisonError::into_inner) =
                    (!words.is_empty()).then(|| words.join(" "));
                announce(&self.served, &Key::State(id), "removed").await;
            }
            Key::CacheLimit => {
                let bytes = bytes_of(&value).ok_or_else(|| {
                    LiveError::BadValue("the limit is a number of bytes, 0 for automatic".into())
                })?;
                let (knob, file) = self.cache.as_ref().ok_or_else(|| {
                    LiveError::NotPermitted("this computer shows no drives".into())
                })?;
                let limit = CacheLimit::from_setting(bytes);
                write_cache_limit(file, limit).map_err(|why| {
                    eprintln!("syncd: the cache limit could not be saved: {why}");
                    failed("That could not be saved.")
                })?;
                knob.send_replace(limit);
                announce(&self.served, &Key::CacheLimit, bytes.to_string()).await;
            }
            Key::Label(_) | Key::Provider(_) | Key::State(_) => {
                return Err(LiveError::NotPermitted("this row is a read-out".into()));
            }
        }
        Ok(())
    }
}

impl<T: GrantsLink + AccountsLink, C> SyncSettings<T, C> {
    /// The accounts and syncd's grants, or that accountd could not be asked.
    async fn read(&self) -> Result<Snapshot, LiveError> {
        self.keep.snapshot().await.map_err(|why| {
            eprintln!("syncd: the accounts could not be read: {why:?}");
            failed("The accounts could not be read.")
        })
    }

    /// "Show in Files". Off answers when the grant is taken back. On may open the provider's
    /// page and wait on the person, longer than a Settings call waits, so it answers at once and
    /// tells the row afterwards: `on`, or `off` with [`FILES_NOT_ON`] under it.
    async fn set_files(
        &self,
        snapshot: &Snapshot,
        id: AccountId,
        toggle: bool,
    ) -> Result<(), LiveError> {
        let forget = |not_on: &Mutex<BTreeSet<AccountId>>, id: &AccountId| {
            not_on
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .remove(id);
        };
        forget(&self.not_on, &id);
        if !toggle {
            self.keep.show_in_files(&id, toggle).await.map_err(|why| {
                eprintln!("syncd: a Settings change was refused: {why}");
                failed("That could not be done.")
            })?;
            return Ok(());
        }
        let row = snapshot
            .account(&id)
            .ok_or_else(|| LiveError::UnknownKey(path(&Key::Files(id.clone()))))?;
        if !self.keep.offers_files(row).await {
            return Err(LiveError::NotPermitted(
                "this account can't be shown in Files".into(),
            ));
        }
        let (keep, served, not_on) = (
            Arc::clone(&self.keep),
            Arc::clone(&self.served),
            Arc::clone(&self.not_on),
        );
        tokio::spawn(async move {
            let on = match keep.show_in_files(&id, true).await {
                Ok(()) => true,
                Err(why) => {
                    eprintln!("syncd: Show in Files was not turned on: {why}");
                    not_on
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .insert(id.clone());
                    false
                }
            };
            announce(&served, &Key::Files(id), on).await;
        });
        Ok(())
    }
}

/// Serves the module at [`SETTINGS_PATH`].
///
/// From then on, each time a drive comes to need more room than there is to keep everything
/// downloaded, or no longer does, its account's "Keep Everything Downloaded" row is told of, so the
/// pane draws its line again; nothing is told while that stands still.
///
/// # Errors
///
/// Returns the [`zbus::Error`] when the module cannot be registered.
pub async fn serve_settings<T: GrantsLink + AccountsLink, C: Callers>(
    connection: &Connection,
    module: SyncSettings<T, C>,
) -> zbus::Result<()> {
    let served_slot = Arc::clone(&module.served);
    let (keep, mut room) = (Arc::clone(&module.keep), module.keep.watch_room());
    let served = serve(connection, SETTINGS_PATH, module)
        .await
        .map_err(|e| zbus::Error::Failure(e.to_string()))?;
    // Served once; a second call leaves the first in place.
    let _ = served_slot.set(served);
    tokio::spawn(async move {
        loop {
            let accounts: Vec<AccountId> = match room.recv().await {
                Ok(drive) => keep.account_of_drive(&drive).await.into_iter().collect(),
                // Missed some: every account's row is told of again.
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => keep
                    .snapshot()
                    .await
                    .unwrap_or_default()
                    .accounts
                    .into_iter()
                    .map(|listed| listed.account.id)
                    .collect(),
                Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
            };
            for id in accounts {
                let all = keep.keeps_everything(&id);
                announce(&served_slot, &Key::KeepAll(id), all).await;
            }
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use account_client::ListedAccount;

    #[test]
    fn a_key_parses_back_from_its_path_and_the_add_row_is_not_an_account() {
        let mut account = account_fake::storage_account();
        account.id = AccountId::parse("graph.acct").expect("id");
        let snapshot = Snapshot {
            accounts: vec![ListedAccount::new(account.clone(), None)],
            grants: Vec::new(),
        };
        let id = account.id;
        let keys = [
            Key::Add,
            Key::Label(id.clone()),
            Key::Provider(id.clone()),
            Key::State(id.clone()),
            Key::Sync(id.clone(), SyncClass::Files),
            Key::Sync(id.clone(), SyncClass::Photos),
            Key::Files(id.clone()),
            Key::KeepAll(id.clone()),
            Key::Remove(id),
            Key::CacheLimit,
        ];
        for key in keys {
            assert_eq!(parse(&path(&key), &snapshot), Some(key.clone()), "{key:?}");
        }
        assert_eq!(parse("accounts.other.label", &snapshot), None);
        assert_eq!(parse("accounts.graph.acct.sync.video", &snapshot), None);
        assert_eq!(parse("accounts.graph.acct.reauth", &snapshot), None);
    }

    #[test]
    fn keep_everything_is_offered_only_while_shown_in_files_and_says_when_it_does_not_fit() {
        let account = account_fake::storage_account();
        let id = account.id.clone();
        let snapshot = Snapshot {
            accounts: vec![ListedAccount::new(account, None)],
            grants: Vec::new(),
        };
        let keep = path(&Key::KeepAll(id));
        let row = |files: FilesRow| {
            schema(&snapshot, |_| files)
                .key
                .into_iter()
                .find(|key| key.path.0 == keep)
        };
        let offered = FilesRow {
            offered: true,
            ..FilesRow::default()
        };
        assert!(row(offered).is_none(), "not while Show in Files is off");
        let shown = FilesRow {
            shown: true,
            ..offered
        };
        let it = row(shown).expect("offered while shown");
        assert_eq!(it.label.0, KEEP_ALL_WORDS.0);
        assert_eq!(it.exposure, Exposure::Basic);
        assert!(it.note.is_none());
        let short = row(FilesRow {
            short_of_room: true,
            ..shown
        })
        .expect("offered");
        assert_eq!(
            short.note.map(|note| note.0).as_deref(),
            Some("Not enough space to keep everything downloaded.")
        );
        for word in ["mount", "FUSE", "WebDAV", "delta", "scope", "token"] {
            assert!(!KEEP_ALL_WORDS.0.contains(word) && !KEEP_ALL_WORDS.1.contains(word));
        }
    }

    #[test]
    fn the_app_folder_switch_is_hidden_once_shown_in_files_and_off() {
        // Its files over Graph: an app folder syncd mirrors.
        let mut account = account_fake::storage_account();
        account.endpoints[0].family = Family::Graph;
        let id = account.id.clone();
        let snapshot = Snapshot {
            accounts: vec![ListedAccount::new(account, None)],
            grants: Vec::new(),
        };
        let app_folder = path(&Key::Sync(id, SyncClass::Files));
        let offered = |files: FilesRow| {
            schema(&snapshot, |_| files)
                .key
                .iter()
                .any(|key| key.path.0 == app_folder)
        };
        let not_shown = FilesRow {
            offered: true,
            ..FilesRow::default()
        };
        assert!(offered(not_shown), "Show in Files off: offered");
        assert!(
            offered(FilesRow {
                app_folder: true,
                ..not_shown
            }),
            "Show in Files off, the mirror on: offered"
        );
        let shown = FilesRow {
            shown: true,
            ..not_shown
        };
        assert!(
            offered(FilesRow {
                app_folder: true,
                ..shown
            }),
            "shown in Files with the mirror still on: offered, so it can be turned off"
        );
        assert!(!offered(shown), "shown in Files and off: hidden");
    }

    #[test]
    fn which_drives_are_kept_whole_is_saved_beside_the_rest_and_read_back() {
        let dir = base_testkit::Scratch::new("syncd-settings-keep-all");
        let file = dir.join("bardo/sync/syncd.toml");
        write_paused(&file, "one", true).expect("paused");
        write_keep_all(&file, "one", true).expect("save");
        write_keep_all(&file, "two", true).expect("save");
        write_keep_all(&file, "two", false).expect("save");
        assert_eq!(read_keep_all(&file), BTreeSet::from(["one".to_owned()]));
        assert_eq!(
            read_paused(&file),
            BTreeSet::from(["one".to_owned()]),
            "kept"
        );
    }

    #[test]
    fn a_file_that_does_not_parse_is_left_as_it_is_and_the_save_says_so() {
        let dir = base_testkit::Scratch::new("syncd-settings-garbled");
        let file = dir.join("syncd.toml");
        let garbled = "[appearance\ntheme = \"dark\"\n";
        std::fs::write(&file, garbled).expect("seed");
        assert!(write_paused(&file, "one", true).is_err());
        assert!(write_keep_all(&file, "one", true).is_err());
        assert!(write_cache_limit(&file, CacheLimit::Automatic).is_err());
        assert_eq!(std::fs::read_to_string(&file).expect("read"), garbled);
    }

    #[test]
    fn a_save_keeps_the_other_sections_and_the_comments() {
        let dir = base_testkit::Scratch::new("syncd-settings-keeps-rest");
        let file = dir.join("syncd.toml");
        let seeded = "# my settings\n[appearance]\ntheme = \"dark\" # night\n\n[storage]\n# drives\npaused = [\"a\"]\n";
        std::fs::write(&file, seeded).expect("seed");
        write_keep_all(&file, "b", true).expect("save");
        let text = std::fs::read_to_string(&file).expect("read");
        assert!(text.contains("# my settings"), "{text}");
        assert!(text.contains("theme = \"dark\" # night"), "{text}");
        assert!(text.contains("# drives"), "{text}");
        assert_eq!(read_paused(&file), BTreeSet::from(["a".to_owned()]));
        assert_eq!(read_keep_all(&file), BTreeSet::from(["b".to_owned()]));
    }

    #[test]
    fn saves_of_different_keys_from_many_threads_all_land() {
        let dir = base_testkit::Scratch::new("syncd-settings-interleaved");
        let file = dir.join("syncd.toml");
        let ids: Vec<String> = (0..8).map(|n| format!("drive{n}")).collect();
        std::thread::scope(|scope| {
            for id in &ids {
                let file = &file;
                scope.spawn(move || {
                    write_keep_all(file, id, true).expect("keep all");
                    write_paused(file, id, true).expect("paused");
                });
            }
            scope.spawn(|| {
                write_cache_limit(&file, CacheLimit::from_setting(5)).expect("limit");
            });
        });
        let all: BTreeSet<String> = ids.into_iter().collect();
        assert_eq!(read_keep_all(&file), all);
        assert_eq!(read_paused(&file), all);
        assert_eq!(read_cache_limit(&file), CacheLimit::from_setting(5));
    }
}
