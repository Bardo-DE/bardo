//! Shared by the integration tests: the design doc's key tables.

use std::path::Path;

/// The UI kit's `design/22-SETTINGS.md`, a sibling checkout of this repository.
pub fn doc() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ui/design/22-SETTINGS.md");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The `Key` column of every data row between `heading` and `next_heading`, in file order. A
/// data row is a table row whose first cell is the key in backticks; the header and separator rows
/// do not start their first cell with a backtick. (The same parse as ui-settings' `tests/keys.rs`.)
pub fn keys_in_section(doc: &str, heading: &str, next_heading: &str) -> Vec<String> {
    let start = doc
        .find(heading)
        .unwrap_or_else(|| panic!("{heading:?} not found in 22-SETTINGS.md"));
    let after = &doc[start..];
    let end = after
        .find(next_heading)
        .unwrap_or_else(|| panic!("{next_heading:?} not found after {heading:?}"));
    after[..end]
        .lines()
        .filter_map(|line| {
            let mut cells = line.split('|').map(str::trim);
            let first_cell = cells.nth(1)?;
            let key = first_cell.strip_prefix('`')?.strip_suffix('`')?;
            // A retiring key has no field on purpose (its row says why).
            let status = line
                .trim_end_matches('|')
                .rsplit('|')
                .next()
                .unwrap_or("")
                .trim();
            (!status.starts_with("RETIRING")).then(|| key.to_owned())
        })
        .collect()
}
