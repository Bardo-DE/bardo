//! The schema of `bardo/appearance.toml`, as the UI kit ships it (design/22-SETTINGS.md section 9.2).

use super::settings::{AppearanceSettings, IconsSettings};
use settings_types::RegionSettings;
use settings_types::schema::{AppId, FilePath, ForeignTables, Schema, SettingsSchema};

/// `bardo.settings.toml`: the keys of `appearance.toml`, the `appearance`, `icons` and `region`
/// domains in one schema.
///
/// The UI kit has no program of its own, so `examples/write_schema.rs` writes it (`cargo run -p
/// ui-settings --example write_schema -- --write-schema <dir>`).
#[must_use]
pub fn bardo_schema() -> Schema {
    Schema {
        footnotes: Vec::new(),
        app: AppId("bardo".to_owned()),
        file: FilePath("bardo/appearance.toml".to_owned()),
        version: 1,
        foreign: ForeignTables::default(),
        pages: Vec::default(),
        key: [
            AppearanceSettings::schema(),
            IconsSettings::schema(),
            RegionSettings::schema(),
        ]
        .into_iter()
        .flat_map(|schema| schema.key)
        .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::bardo_schema;
    use settings_types::schema::Schema;

    #[test]
    fn the_bardo_schema_holds_every_domain_and_round_trips() {
        let schema = bardo_schema();
        let paths: Vec<&str> = schema.key.iter().map(|key| key.path.0.as_str()).collect();
        assert!(paths.contains(&"appearance.theme"));
        assert!(paths.contains(&"icons.style"));
        assert!(paths.contains(&"region.hours"));
        assert_eq!(Schema::from_toml(&schema.to_toml()), Ok(schema));
    }
}
