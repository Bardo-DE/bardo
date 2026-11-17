//! A whole program's settings schema: what a program ships, and what the Settings app
//! discovers (design/22-SETTINGS.md sections 9.1-9.2).

use super::footnote::Footnote;
use serde::{Deserialize, Serialize};

use super::agent::never_settable_violation;
use super::key::KeySpec;
use super::page_decl::PageDecl;

/// The directory name a schema's owning program installs under: `bardo`, `shell`, `mailo`.
///
/// The derive reads it off the first path segment of `#[settings(file = "...")]`
/// (`crates/ds-settings-derive/src/gen_struct.rs`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AppId(pub String);

/// The settings file a key lives in, relative to its app: `"bardo/appearance.toml"`,
/// `"shell/settings.toml"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FilePath(pub String);

/// The top-level tables of a program's settings file that the program owns but that are not
/// settings.
///
/// Examples are an engine list and a probe table: the Settings app and bardo-settings's unknown-key
/// check leave every key under them alone (design/22-SETTINGS.md section 9.2). Empty for most
/// files, and then not written.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ForeignTables(pub Vec<String>);

impl ForeignTables {
    /// Whether `path` (a dotted key path: `engines.local.cmd`) sits under one of these tables.
    #[must_use]
    pub fn holds(&self, path: &str) -> bool {
        let first = path.split('.').next().unwrap_or(path);
        self.0.iter().any(|table| table == first)
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// One program's whole schema, as `<app-id>.settings.toml` ships it (section 9.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Schema {
    pub app: AppId,
    pub file: FilePath,
    pub version: u16,
    /// `foreign = ["engines", "probe"]`: the file's tables that are the program's own data,
    /// not settings. Absent in the TOML means none.
    #[serde(default, skip_serializing_if = "ForeignTables::is_empty")]
    pub foreign: ForeignTables,
    /// One `[[page]]` table per page this program declares: its id, title, glyph and place in the
    /// sidebar (section 9.3). Absent means none; a key's page is then one of the built-in ones.
    #[serde(default, rename = "page", skip_serializing_if = "Vec::is_empty")]
    pub pages: Vec<PageDecl>,
    /// One `[[key]]` table per entry, TOML's array-of-tables (section 9.2).
    pub key: Vec<KeySpec>,
    /// One `[[footnote]]` table per section that has one. Absent means none, so older schemas
    /// parse and write unchanged.
    #[serde(default, rename = "footnote", skip_serializing_if = "Vec::is_empty")]
    pub footnotes: Vec<Footnote>,
}

impl Schema {
    /// The TOML text this schema writes as `<app-id>.settings.toml`.
    ///
    /// # Panics
    ///
    /// Panics only if the schema cannot be written as TOML, which its field types rule out.
    #[must_use]
    pub fn to_toml(&self) -> String {
        toml::to_string(self)
            .expect("Schema fields are all TOML-representable: strings, numbers, enums and Vec")
    }

    /// The schema `text` holds, or why it does not parse. A key under an
    /// [`AGENT_NEVER_SETTABLE`](super::AGENT_NEVER_SETTABLE) prefix that is marked
    /// `agent = "settable"` is an error, so a schema file any program installs cannot make
    /// the companion's own limits, computer use or a lock agent-settable.
    ///
    /// # Errors
    ///
    /// Returns the parser's message when `text` is not a valid schema.
    pub fn from_toml(text: &str) -> Result<Schema, String> {
        let schema: Schema = toml::from_str(text).map_err(|e| e.to_string())?;
        match never_settable_violation(&schema.key) {
            Some(key) => Err(format!(
                "key {} is under a never-agent-settable prefix but is marked agent = \"settable\"",
                key.path.0
            )),
            None => Ok(schema),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::schema::{AppId, Exposure, FilePath, KeyKind, KeyPath, Label, Page, Schema};

    fn sample() -> Schema {
        Schema {
            footnotes: Vec::new(),
            app: AppId("bardo".to_owned()),
            file: FilePath("bardo/appearance.toml".to_owned()),
            version: 1,
            foreign: crate::schema::ForeignTables::default(),
            pages: Vec::default(),
            key: vec![crate::schema::KeySpec {
                ends: None,
                path: KeyPath("appearance.theme".to_owned()),
                kind: KeyKind::Segmented {
                    variants: vec!["system".to_owned(), "light".to_owned(), "dark".to_owned()],
                },
                default: toml::Value::String("system".to_owned()),
                label: Label("Theme".to_owned()),
                help: crate::schema::Help(String::new()),
                page: Page::Appearance,
                section: crate::schema::Section("Appearance".to_owned()),
                exposure: Exposure::Basic,
                labels: crate::schema::WordLabels::default(),
                agent: crate::schema::AgentSetting::default(),
                unavailable: std::collections::BTreeMap::default(),
                groups: std::collections::BTreeMap::default(),
                places: std::collections::BTreeMap::default(),
                note: None,
                control: None,
            }],
        }
    }

    #[test]
    fn a_never_settable_key_marked_settable_is_rejected() {
        const NEVER: &[&str] = &[
            "agent.budget",
            "ai.model",
            "cua.enabled",
            "memory.on",
            "companion.mood",
            "session.lock_grace_s",
            "idle.dim_after_s",
            "accounts.signed_in",
            "privacy.share_usage",
            "network.proxy",
            "security.require_password",
        ];
        for path in NEVER {
            let mut schema = sample();
            schema.key[0].path = KeyPath((*path).to_owned());
            schema.key[0].agent = crate::schema::AgentSetting::Settable;
            let err = Schema::from_toml(&schema.to_toml()).expect_err(path);
            assert!(err.contains(path), "{err}");
            // The same key left hands-off is fine, and so is a settable key elsewhere.
            schema.key[0].agent = crate::schema::AgentSetting::HandsOff;
            assert_eq!(Schema::from_toml(&schema.to_toml()), Ok(schema));
        }
        let mut ok = sample();
        ok.key[0].agent = crate::schema::AgentSetting::Settable;
        assert_eq!(Schema::from_toml(&ok.to_toml()), Ok(ok));
    }
}

#[cfg(test)]
mod foreign_tests {
    use super::{ForeignTables, Schema};

    #[test]
    fn foreign_tables_round_trip_and_are_left_out_when_empty() {
        let text = "app = \"inferd\"\nfile = \"inferd/inferd.toml\"\nversion = 1\nforeign = [\"engines\", \"probe\"]\nkey = []\n";
        let schema = Schema::from_toml(text).expect("parses");
        assert_eq!(
            schema.foreign,
            ForeignTables(vec!["engines".to_owned(), "probe".to_owned()])
        );
        assert!(
            schema
                .to_toml()
                .contains("foreign = [\"engines\", \"probe\"]")
        );
        let plain = Schema {
            foreign: ForeignTables::default(),
            ..schema
        };
        assert!(!plain.to_toml().contains("foreign"));
    }

    #[test]
    fn a_path_is_held_by_its_first_segment_only() {
        let foreign = ForeignTables(vec!["engines".to_owned()]);
        let cases = [
            ("engines", true),
            ("engines.local.cmd", true),
            ("engine.local", false),
            ("probe.engines", false),
        ];
        for (path, held) in cases {
            assert_eq!(foreign.holds(path), held, "{path}");
        }
    }
}

#[cfg(test)]
mod page_tests {
    use super::Schema;
    use crate::schema::{Page, PageStatus, PageTable};

    const HEAD: &str = "app = \"shell\"\nfile = \"shell/settings.toml\"\nversion = 1\n";
    const PAGE: &str = "[[page]]\nid = \"lighting\"\ntitle = \"Lighting\"\nsymbol = \"sun\"\norder = 25\naliases = [\"light\"]\n";
    const KEY: &str = "[[key]]\npath = \"lamp.level\"\ndefault = \"x\"\nlabel = \"Level\"\nhelp = \"\"\nsection = \"\"\nexposure = \"basic\"\n[key.kind]\nkind = \"text\"\n[key.page]\nkind = \"lighting\"\n";

    #[test]
    fn a_key_on_a_page_nobody_declared_is_kept_and_the_page_is_unknown() {
        let schema = Schema::from_toml(&format!("{HEAD}{KEY}")).expect("the schema still reads");
        assert_eq!(schema.key.len(), 1);
        assert_eq!(schema.key[0].page, Page::App("lighting".to_owned()));
        let table = PageTable::of_schemas(std::slice::from_ref(&schema));
        assert_eq!(table.status(&schema.key[0].page), PageStatus::Unknown);
    }

    #[test]
    fn a_declared_page_reads_writes_and_makes_its_keys_known() {
        let schema = Schema::from_toml(&format!("{HEAD}{PAGE}{KEY}")).expect("parses");
        assert_eq!(schema.pages.len(), 1);
        let table = PageTable::of_schemas(std::slice::from_ref(&schema));
        assert_eq!(table.status(&schema.key[0].page), PageStatus::Declared);
        assert_eq!(
            table.resolve("light").map(|id| id.0),
            Some("lighting".to_owned())
        );
        let again = Schema::from_toml(&schema.to_toml()).expect("round trips");
        assert_eq!(again, schema);
    }

    #[test]
    fn a_schema_without_pages_writes_no_page_table() {
        let schema = Schema::from_toml(&format!("{HEAD}{KEY}")).expect("parses");
        assert!(!schema.to_toml().contains("[[page]]"));
    }
}
