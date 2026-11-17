//! A program's schema file on disk: written as `<app-id>.settings.toml` where the Settings app
//! looks, and discovered under the data directories (design/22-SETTINGS.md sections 9.1 and 9.2).
//! The schema itself is `settings_types::schema::Schema`; only the files are here.

use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};

use base_fs::{NewFileMode, ReadCap, read_text_capped, write_atomic};
use base_xdg::Xdg;
use settings_types::schema::Schema;

/// Writes `<app-id>.settings.toml` into `dir` (creating it if needed) through
/// [`base_fs::write_atomic`], the same mechanism every settings file in this workspace
/// uses (design/22-SETTINGS.md section 2).
///
/// # Errors
///
/// Returns the I/O error when the directory or the file cannot be written.
pub fn write_schema(schema: &Schema, dir: &Path) -> io::Result<PathBuf> {
    write_named(schema, dir, &format!("{}.settings.toml", schema.app.0))
}

/// Writes `<app-id>.<part>.settings.toml` into `dir`.
///
/// It is the schema of a second file the same program owns, which [`discover`] admits beside its `<app-id>.settings.toml`
/// (`bardo.extensions.settings.toml` for `bardo/extensions.toml`).
///
/// # Errors
///
/// Returns the I/O error when the directory or the file cannot be written.
pub fn write_schema_part(schema: &Schema, dir: &Path, part: &str) -> io::Result<PathBuf> {
    write_named(
        schema,
        dir,
        &format!("{}.{part}.settings.toml", schema.app.0),
    )
}

fn write_named(schema: &Schema, dir: &Path, name: &str) -> io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let target = dir.join(name);
    write_atomic(&target, schema.to_toml().as_bytes(), NewFileMode::Umask)
        .map_err(|error| error.source)?;
    Ok(target)
}

/// The most a schema file may hold: a few KiB for a program with a hundred keys.
const SCHEMA_CAP: ReadCap = ReadCap::kib(256);

/// The suffix of a schema file's name; what precedes it is the program it describes.
const SCHEMA_SUFFIX: &str = ".settings.toml";

/// Every valid schema found directly under each of `dirs`, in directory order and then file name
/// order within a directory.
///
/// Section 9.3 says "discover every `*.settings.toml` under `$XDG_DATA_DIRS/bardo/settings/`...". A
/// directory that does not exist is skipped silently (a normal `$XDG_DATA_DIRS` entry with nothing
/// installed yet); a file that exists but will not parse is skipped with a warning on stderr
/// (section 9.2, "a schema without a program... is skipped with a warning").
///
/// A schema is admitted only for what its location says it is:
///
/// - its file is named `<app>.settings.toml` for the `app` it declares, and the settings file it
///   names (`file`) is inside that app's own directory, so a schema cannot describe another
///   program's file;
/// - the first schema found for an app wins, so the order of `dirs` is the order of trust:
///   [`data_dirs`] lists the system directories before the user's own, and a schema in the
///   user's directory can add a program but never replace an installed one's;
/// - it is no larger than 256 KiB and a regular file.
///
/// The `agent = "settable"` marks of the keys it returns are therefore those of a schema this
/// function admitted, read through [`KeySpec::agent`].
#[must_use]
pub fn discover(dirs: &[PathBuf]) -> Vec<Schema> {
    let mut found: Vec<Schema> = Vec::new();
    for dir in dirs {
        let Ok(mut entries) = std::fs::read_dir(dir).map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| {
                    path.is_file()
                        && path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .is_some_and(|name| name.ends_with(SCHEMA_SUFFIX))
                })
                .collect::<Vec<_>>()
        }) else {
            continue;
        };
        entries.sort();
        for path in entries {
            match admit(&path, &found) {
                Ok(schema) => found.push(schema),
                Err(why) => eprintln!(
                    "bardo settings: skipping stale schema {}: {why}",
                    path.display()
                ),
            }
        }
    }
    found
}

/// The schema at `path`, if it may join `found`.
fn admit(path: &Path, found: &[Schema]) -> Result<Schema, String> {
    let text = read_text_capped(path, SCHEMA_CAP).map_err(|error| error.to_string())?;
    let schema = Schema::from_toml(&text)?;
    let app = schema.app.0.as_str();
    let named_for = path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_suffix(SCHEMA_SUFFIX));
    // `<app>.settings.toml`, or `<app>.<part>.settings.toml` for a second file the same program
    // owns (Bardo's `extensions.toml` beside `appearance.toml`).
    let named = named_for.is_some_and(|name| {
        name == app
            || name
                .strip_prefix(app)
                .and_then(|rest| rest.strip_prefix('.'))
                .is_some_and(|part| !part.is_empty() && !part.contains('.'))
    });
    if !named {
        return Err(format!("the file is not named for its program, {app}"));
    }
    if schema.file.0.split('/').next() != Some(app) {
        return Err(format!(
            "it names {}, which is not inside {app}'s own directory",
            schema.file.0
        ));
    }
    if found
        .iter()
        .any(|earlier| earlier.app == schema.app && earlier.file == schema.file)
    {
        return Err(format!(
            "{app} already has a schema for {} in an earlier directory",
            schema.file.0
        ));
    }
    Ok(schema)
}

/// Every `$XDG_DATA_DIRS`-listed `<dir>/bardo/settings`, then `$XDG_DATA_HOME/bardo/settings` last,
/// the directories [`discover`] scans (section 9.2).
///
/// The user's directory comes last because [`discover`] keeps the first schema it finds for a
/// program: a user's own install can add a program but never shadow an installed one (and so never
/// change which of its keys an agent may set). Falls back to the XDG defaults (`~/.local/share`,
/// `/usr/local/share:/usr/share`) when the variable is unset, per the XDG base directory
/// specification.
#[must_use]
pub fn data_dirs() -> Vec<PathBuf> {
    data_dirs_from(
        std::env::var_os("XDG_DATA_HOME").as_deref(),
        std::env::var_os("XDG_DATA_DIRS").as_deref(),
        std::env::var_os("HOME").as_deref(),
    )
}

/// [`data_dirs`] with the environment handed in.
///
/// `xdg_data_home` (else `home`'s `.local/share`) and `xdg_data_dirs` (else
/// `/usr/local/share:/usr/share`), so a caller that owns its directories, or a test, names them
/// itself. The XDG rules (`base_xdg`) apply: an empty or relative value is ignored, and a value
/// that is not UTF-8 reads as unset.
#[must_use]
pub fn data_dirs_from(
    xdg_data_home: Option<&OsStr>,
    xdg_data_dirs: Option<&OsStr>,
    home: Option<&OsStr>,
) -> Vec<PathBuf> {
    data_search_from(xdg_data_home, xdg_data_dirs, home)
        .into_iter()
        .map(|base| base.join("bardo").join("settings"))
        .collect()
}

/// The XDG data search path with the environment handed in.
///
/// Each `$XDG_DATA_DIRS` entry first,
/// then `$XDG_DATA_HOME` (else `home`'s `.local/share`) last, so the person's own directory can
/// add to what is installed and never shadow it.
#[must_use]
pub fn data_search_from(
    xdg_data_home: Option<&OsStr>,
    xdg_data_dirs: Option<&OsStr>,
    home: Option<&OsStr>,
) -> Vec<PathBuf> {
    let xdg = xdg_of(xdg_data_home, xdg_data_dirs, home);
    xdg.data_dirs()
        .into_iter()
        .chain(xdg.data_home().ok())
        .collect()
}

/// `$XDG_DATA_HOME`, else `home`'s `.local/share`, with the environment handed in.
#[must_use]
pub fn data_home_from(xdg_data_home: Option<&OsStr>, home: Option<&OsStr>) -> Option<PathBuf> {
    xdg_of(xdg_data_home, None, home).data_home().ok()
}

/// The directories of `$XDG_DATA_DIRS` (else `/usr/local/share:/usr/share`), with the value
/// handed in.
#[must_use]
pub fn data_dirs_list_from(xdg_data_dirs: Option<&OsStr>) -> Vec<PathBuf> {
    xdg_of(None, xdg_data_dirs, None).data_dirs()
}

fn xdg_of<'a>(
    xdg_data_home: Option<&'a OsStr>,
    xdg_data_dirs: Option<&'a OsStr>,
    home: Option<&'a OsStr>,
) -> Xdg<impl Fn(&str) -> Option<String> + 'a> {
    let text = |value: Option<&OsStr>| value.and_then(OsStr::to_str).map(str::to_owned);
    Xdg::new(move |name: &str| match name {
        "XDG_DATA_HOME" => text(xdg_data_home),
        "XDG_DATA_DIRS" => text(xdg_data_dirs),
        "HOME" => text(home),
        _ => None,
    })
}

/// The `--write-schema <dir>` flag (section 9.2: "developers run
/// `cargo run -p <app> -- --write-schema <dir>` for a local install"), a helper a program's own
/// `main` wires into its argument parsing.
///
/// Returns `Ok(true)` (and has written the file) when `args` asked for it, `Ok(false)` when the
/// flag was absent so the caller's normal startup should continue.
///
/// # Errors
///
/// Returns the I/O error when the schema file cannot be written.
pub fn maybe_write_schema(schema: &Schema, args: &[String]) -> io::Result<bool> {
    let Some(position) = args.iter().position(|arg| arg == "--write-schema") else {
        return Ok(false);
    };
    let Some(dir) = args.get(position + 1) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--write-schema needs a directory argument",
        ));
    };
    write_schema(schema, Path::new(dir))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::{data_dirs_from, discover, maybe_write_schema, write_schema};
    use settings_types::schema::{
        AgentSetting, AppId, Exposure, FilePath, ForeignTables, Help, KeyKind, KeyPath, KeySpec,
        Label, Page, Schema, Section, WordLabels,
    };
    use std::ffi::OsString;

    fn sample() -> Schema {
        Schema {
            footnotes: Vec::new(),
            app: AppId("bardo".to_owned()),
            file: FilePath("bardo/appearance.toml".to_owned()),
            version: 1,
            foreign: ForeignTables::default(),
            pages: Vec::default(),
            key: vec![KeySpec {
                ends: None,
                path: KeyPath("appearance.theme".to_owned()),
                kind: KeyKind::Segmented {
                    variants: vec!["system".to_owned(), "light".to_owned(), "dark".to_owned()],
                },
                default: toml::Value::String("system".to_owned()),
                label: Label("Theme".to_owned()),
                help: Help(String::new()),
                page: Page::Appearance,
                section: Section("Appearance".to_owned()),
                exposure: Exposure::Basic,
                labels: WordLabels::default(),
                agent: AgentSetting::default(),
                unavailable: std::collections::BTreeMap::default(),
                groups: std::collections::BTreeMap::default(),
                places: std::collections::BTreeMap::default(),
                note: None,
                control: None,
            }],
        }
    }

    #[test]
    fn discover_skips_a_schema_that_marks_a_never_key_settable() {
        let dir = std::env::temp_dir().join(format!(
            "ds-settings-schema-test-{}-{}",
            std::process::id(),
            "never-settable"
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let mut schema = sample();
        schema.key[0].path = KeyPath("computer_use.enabled".to_owned());
        schema.key[0].agent = AgentSetting::Settable;
        write_schema(&schema, &dir).unwrap();
        assert_eq!(discover(std::slice::from_ref(&dir)), Vec::new());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn write_to_then_discover_finds_it() {
        let dir = std::env::temp_dir().join(format!(
            "ds-settings-schema-test-{}-{}",
            std::process::id(),
            "write-then-discover"
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let schema = sample();
        write_schema(&schema, &dir).unwrap();
        let found = discover(std::slice::from_ref(&dir));
        assert_eq!(found, vec![schema]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ds-settings-schema-test-{}-{tag}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_user_schema_never_shadows_an_installed_one_of_the_same_program() {
        let system = scratch("shadow-system");
        let user = scratch("shadow-user");
        let mut installed = sample();
        installed.key[0].agent = AgentSetting::HandsOff;
        write_schema(&installed, &system).unwrap();
        let mut planted = sample();
        planted.key[0].agent = AgentSetting::Settable;
        write_schema(&planted, &user).unwrap();
        // The system directory is listed first, as `data_dirs` lists it.
        let found = discover(&[system.clone(), user.clone()]);
        assert_eq!(found, vec![installed]);
        // A user schema for a program nobody installed is still found.
        let mut own = sample();
        own.app = AppId("mine".to_owned());
        own.file = FilePath("mine/settings.toml".to_owned());
        write_schema(&own, &user).unwrap();
        assert_eq!(discover(&[system.clone(), user.clone()]).len(), 2);
        std::fs::remove_dir_all(&system).unwrap();
        std::fs::remove_dir_all(&user).unwrap();
    }

    #[test]
    fn a_schema_that_names_another_programs_file_or_name_is_not_admitted() {
        let dir = scratch("binding");
        let mut other_file = sample();
        other_file.file = FilePath("shell/settings.toml".to_owned());
        write_schema(&other_file, &dir).unwrap();
        assert_eq!(discover(std::slice::from_ref(&dir)), Vec::new());

        // Named for one program, declaring another.
        std::fs::remove_file(dir.join("bardo.settings.toml")).unwrap();
        std::fs::write(dir.join("shell.settings.toml"), sample().to_toml()).unwrap();
        assert_eq!(discover(std::slice::from_ref(&dir)), Vec::new());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_huge_schema_file_is_not_read() {
        let dir = scratch("huge");
        let text = format!("{}\n# {}", sample().to_toml(), "x".repeat(300 * 1024));
        std::fs::write(dir.join("bardo.settings.toml"), text).unwrap();
        assert_eq!(discover(std::slice::from_ref(&dir)), Vec::new());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_missing_directory_and_a_malformed_file_are_skipped() {
        let dir = std::env::temp_dir().join("ds-settings-schema-test-does-not-exist");
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(discover(&[dir]), Vec::new());

        let dir = scratch("malformed");
        std::fs::write(dir.join("broken.settings.toml"), "not = [valid").unwrap();
        assert_eq!(discover(std::slice::from_ref(&dir)), Vec::new());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_data_dirs_follow_the_xdg_rules() {
        let os = |text: &str| Some(OsString::from(text));
        // (name, XDG_DATA_HOME, XDG_DATA_DIRS, HOME, the folders)
        let cases = [
            (
                "XDG_DATA_HOME comes after XDG_DATA_DIRS so it never shadows them",
                os("/xdg-data-home"),
                os("/a:/b"),
                os("/home/ada"),
                vec![
                    "/a/bardo/settings",
                    "/b/bardo/settings",
                    "/xdg-data-home/bardo/settings",
                ],
            ),
            (
                "XDG_DATA_HOME falls back to HOME/.local/share",
                None,
                os("/a"),
                os("/home/ada"),
                vec!["/a/bardo/settings", "/home/ada/.local/share/bardo/settings"],
            ),
            (
                "empty values are ignored",
                os(""),
                os(""),
                os("/home/ada"),
                vec![
                    "/usr/local/share/bardo/settings",
                    "/usr/share/bardo/settings",
                    "/home/ada/.local/share/bardo/settings",
                ],
            ),
            (
                "relative values are ignored",
                os("rel"),
                os("rel:/a"),
                None,
                vec!["/a/bardo/settings"],
            ),
        ];
        for (name, home, dirs, user, want) in cases {
            let want: Vec<std::path::PathBuf> =
                want.into_iter().map(std::path::PathBuf::from).collect();
            assert_eq!(
                data_dirs_from(home.as_deref(), dirs.as_deref(), user.as_deref()),
                want,
                "{name}"
            );
        }
    }

    #[test]
    fn maybe_write_schema_writes_only_when_the_flag_is_present() {
        let dir = std::env::temp_dir().join(format!(
            "ds-settings-schema-test-{}-{}",
            std::process::id(),
            "cli"
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let schema = sample();

        let absent = maybe_write_schema(&schema, &["bardo".to_owned()]).unwrap();
        assert!(!absent);
        assert!(!dir.exists());

        let present = maybe_write_schema(
            &schema,
            &[
                "bardo".to_owned(),
                "--write-schema".to_owned(),
                dir.to_string_lossy().into_owned(),
            ],
        )
        .unwrap();
        assert!(present);
        assert!(dir.join("bardo.settings.toml").exists());
        std::fs::remove_dir_all(&dir).unwrap();

        // The flag without a directory is an error.
        let args = vec!["bardo".to_owned(), "--write-schema".to_owned()];
        maybe_write_schema(&schema, &args).unwrap_err();
    }
}
