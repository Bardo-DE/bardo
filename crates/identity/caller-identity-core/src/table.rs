//! The caller table: which programs are the desktop's own, and what role each plays. One shipped,
//! root-owned file. A row with a `unit` is a service; a row without one is a program known by its
//! image alone. Reading the file is the I/O crate's job; this is the text format and the rows.

#[cfg(feature = "toml")]
use crate::error::{IdError, RowError, TableError};
use crate::ids::{AppKey, ExePath, ExePattern, RoleName, UnitName};
#[cfg(feature = "toml")]
use serde::Deserialize;

/// The longest table text read. A real table is a few kilobytes.
pub const TABLE_BYTES_MAX: usize = 256 * 1024;
/// The most rows a table may hold.
pub const TABLE_ROWS_MAX: usize = 256;
/// The longest free-text row name.
pub const NAME_MAX: usize = 128;

/// One program of the desktop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The program's app key (`org.bardo.Shell`).
    pub app: AppKey,
    /// The service unit it runs as. `None` for a program matched by its image alone.
    pub unit: Option<UnitName>,
    /// The image paths it may run from; at least one.
    pub exe: Vec<ExePattern>,
    /// The role it plays.
    pub role: RoleName,
    /// A label for logs.
    pub name: Option<String>,
    /// Whether the row is a developer row ([`Table::with_dev_rows`]).
    dev: bool,
}

impl Row {
    /// Whether this row came from a developer's own file ([`Table::with_dev_rows`]). Such a row
    /// gives `Proof::Dev` at best and never a real proof.
    #[must_use]
    pub fn is_dev(&self) -> bool {
        self.dev
    }

    /// Whether this row can give a real proof (`Cgroup`, `Exe`, `Unit` or `Full`): every row
    /// but a developer row.
    pub(crate) fn proves(&self) -> bool {
        !self.dev
    }

    /// Whether the image `path` is one of this row's.
    #[must_use]
    pub fn matches_image(&self, path: &ExePath) -> bool {
        self.exe.iter().any(|pattern| pattern.matches(path))
    }
}

#[cfg(feature = "toml")]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFile {
    #[serde(default)]
    peer: Vec<RawRow>,
}

#[cfg(feature = "toml")]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRow {
    app: String,
    unit: Option<String>,
    #[serde(default)]
    exe: Vec<String>,
    role: String,
    name: Option<String>,
}

#[cfg(feature = "toml")]
impl RawRow {
    fn typed(&self) -> Result<Row, RowError> {
        if self.exe.is_empty() {
            return Err(RowError::NoExe);
        }
        let exe = self
            .exe
            .iter()
            .map(|text| ExePattern::new(text))
            .collect::<Result<Vec<_>, IdError>>()?;
        if self
            .name
            .as_ref()
            .is_some_and(|n| n.is_empty() || n.len() > NAME_MAX)
        {
            return Err(RowError::BadName { max: NAME_MAX });
        }
        Ok(Row {
            app: AppKey::new(&self.app)?,
            unit: self.unit.as_deref().map(UnitName::new).transpose()?,
            exe,
            role: RoleName::new(&self.role)?,
            name: self.name.clone(),
            dev: false,
        })
    }
}

/// The rows of the caller table. Within one table a unit appears once; across tables the first
/// row for a unit wins ([`Table::merge`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Table {
    rows: Vec<Row>,
}

impl Table {
    /// A table with no rows: nobody is a service.
    #[must_use]
    pub fn empty() -> Table {
        Table::default()
    }

    /// The table written as `text`: `[[peer]]` rows with `app`, `exe` (a non-empty list), `role`
    /// and optionally `unit` and `name`. A file is refused whole for: an unknown key, a row
    /// without `exe`, a relative or `..` path, a `**` or two `*` in a segment, a malformed id,
    /// or two rows naming one unit. Only with feature `toml` (on by default): a crate that names the
    /// rows but never parses a file turns it off and reaches no TOML parser.
    ///
    /// # Errors
    ///
    /// Returns a [`TableError`] when `text` is too long, has too many rows, does not parse as TOML,
    /// has a row that is not valid, or names one unit in two rows.
    #[cfg(feature = "toml")]
    pub fn from_toml(text: &str) -> Result<Table, TableError> {
        if text.len() > TABLE_BYTES_MAX {
            return Err(TableError::TooLong {
                max: TABLE_BYTES_MAX,
            });
        }
        let file: RawFile = toml::from_str(text).map_err(|e| TableError::Syntax(e.to_string()))?;
        if file.peer.len() > TABLE_ROWS_MAX {
            return Err(TableError::TooManyRows {
                max: TABLE_ROWS_MAX,
            });
        }
        let mut rows: Vec<Row> = Vec::with_capacity(file.peer.len());
        for (i, raw) in file.peer.iter().enumerate() {
            let row = raw.typed().map_err(|fault| TableError::Row {
                index: i + 1,
                app: raw.app.clone(),
                fault,
            })?;
            if let Some(unit) = &row.unit
                && rows.iter().any(|r| r.unit.as_ref() == Some(unit))
            {
                return Err(TableError::DuplicateUnit(unit.clone()));
            }
            rows.push(row);
        }
        Ok(Table { rows })
    }

    /// This table's rows, then the rows of `other` whose unit this table does not already name.
    /// Load the shipped file first and the administrator's files after it: a later file adds
    /// programs and units, and cannot take over a unit the shipped table names. Developer rows
    /// ([`Table::with_dev_rows`]) always end up after the shipped ones, whatever the order of the
    /// merge, and a shipped row displaces a developer row that names its unit.
    #[must_use]
    pub fn merge(mut self, other: Table) -> Table {
        for row in other.rows {
            if !row.dev
                && let Some(unit) = &row.unit
            {
                self.rows
                    .retain(|r| !(r.dev && r.unit.as_ref() == Some(unit)));
            }
            let taken = row
                .unit
                .as_ref()
                .is_some_and(|unit| self.rows.iter().any(|r| r.unit.as_ref() == Some(unit)));
            if !taken && self.rows.len() < TABLE_ROWS_MAX {
                self.rows.push(row);
            }
        }
        self.rows.sort_by_key(|row| row.dev);
        self
    }

    /// This table with the rows of `dev` added as developer rows: rows of a developer's own file,
    /// which let a stand-in the person built take a service's role at `Proof::Dev` and nothing
    /// above it. They come after every shipped row, never displace one, and never name a unit a
    /// shipped row names ([`Table::merge`]). Only `caller-identity-dev`, which reads such a file
    /// and is never part of a shipped program, calls this.
    #[must_use]
    pub fn with_dev_rows(self, dev: Table) -> Table {
        let marked = dev.rows.into_iter().map(|mut row| {
            row.dev = true;
            row
        });
        self.merge(Table {
            rows: marked.collect(),
        })
    }

    /// The rows, in order.
    #[must_use]
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// The row for the unit `unit`.
    #[must_use]
    pub fn row_for_unit(&self, unit: &UnitName) -> Option<&Row> {
        self.rows.iter().find(|r| r.unit.as_ref() == Some(unit))
    }

    /// The row for the unit `unit` among the rows that can give a real proof.
    pub(crate) fn proving_row_for_unit(&self, unit: &UnitName) -> Option<&Row> {
        self.rows
            .iter()
            .find(|r| r.proves() && r.unit.as_ref() == Some(unit))
    }

    /// Whether any row names `role`.
    #[must_use]
    pub fn has_role(&self, role: &RoleName) -> bool {
        self.rows.iter().any(|r| &r.role == role)
    }
}
