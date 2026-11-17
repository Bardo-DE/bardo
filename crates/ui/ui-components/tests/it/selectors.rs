//! The public selector table: every class it lists is in the stylesheet, its names are unique,
//! and `docs/selectors.md` lists every name in the table.

use std::collections::HashSet;
use std::path::PathBuf;
use ui_components::selectors::{AXES, COMPONENTS};

fn page() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../docs/selectors.md")
}

/// Whether the stylesheet has a rule that selects `.class`.
fn styled(sheet: &str, class: &str) -> bool {
    let needle = format!(".{class}");
    sheet.match_indices(&needle).any(|(at, _)| {
        !sheet[at + needle.len()..]
            .starts_with(|c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    })
}

#[test]
fn every_listed_class_is_in_the_stylesheet() {
    let sheet = ui_components::stylesheet();
    let mut missing = Vec::new();
    for component in COMPONENTS {
        let classes = std::iter::once(component.root_selector()).chain(component.part_selectors());
        for selector in classes {
            let class = selector.trim_start_matches('.');
            if !styled(sheet, class) {
                missing.push(format!("{}: {selector}", component.component));
            }
        }
    }
    assert_eq!(missing, Vec::<String>::new(), "listed but not styled");
}

#[test]
fn the_table_has_no_duplicates_and_every_root_is_a_ds_class() {
    let roots: Vec<&str> = COMPONENTS.iter().map(|c| c.root).collect();
    let unique: HashSet<&str> = roots.iter().copied().collect();
    assert_eq!(
        roots.len(),
        unique.len(),
        "a root is listed twice: {roots:?}"
    );
    for component in COMPONENTS {
        assert!(component.root.starts_with("ds-"), "{}", component.root);
        let parts: HashSet<&str> = component.parts.iter().copied().collect();
        assert_eq!(
            parts.len(),
            component.parts.len(),
            "{}",
            component.component
        );
    }
    let axes: HashSet<&str> = AXES.iter().map(|axis| axis.attribute).collect();
    assert_eq!(axes.len(), AXES.len(), "an attribute is listed twice");
}

#[test]
fn the_doc_page_lists_every_selector() {
    let path = page();
    let page = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut missing = Vec::new();
    for component in COMPONENTS {
        let selectors =
            std::iter::once(component.root_selector()).chain(component.part_selectors());
        for selector in selectors {
            if !page.contains(&format!("`{selector}`")) {
                missing.push(selector);
            }
        }
    }
    for axis in AXES {
        if !page.contains(&format!("`{}`", axis.attribute)) {
            missing.push(axis.attribute.to_string());
        }
    }
    assert_eq!(missing, Vec::<String>::new(), "not in docs/selectors.md");
}

#[test]
fn the_report_accepts_every_listed_selector_and_notes_one_that_is_not() {
    let kits = ui_components::kits();
    let listed: Vec<String> = COMPONENTS
        .iter()
        .flat_map(|c| std::iter::once(c.root_selector()).chain(c.part_selectors()))
        .chain(
            AXES.iter()
                .filter(|a| !a.attribute.ends_with('*'))
                .map(|a| format!("[{}]", a.attribute)),
        )
        .chain(["[data-surface=bar]".to_owned(), "[aria-label]".to_owned()])
        .collect();
    for selector in listed {
        let notes = ui_lint::user_stylesheet(&format!("{selector} {{ color: red }}"), &kits);
        assert_eq!(notes, vec![], "{selector} is listed and must be accepted");
    }
    let notes = ui_lint::user_stylesheet(".ds-not-listed { color: red }", &kits);
    assert_eq!(notes.len(), 1, "{notes:?}");
}

#[test]
fn a_renamed_part_keeps_its_old_name_for_one_release() {
    let classes = ui_components::selectors::classes();
    for rename in ui_components::selectors::RENAMES {
        let component = COMPONENTS.iter().find(|c| c.root == rename.root);
        let component = component.unwrap_or_else(|| panic!("{}: no such component", rename.root));
        assert!(
            component.parts.contains(&rename.part),
            "{}: the new part is not in the table",
            rename.new_class()
        );
        assert!(
            classes.contains(&rename.old_class()),
            "{}: the old class is not public",
            rename.old_class()
        );
        assert!(
            styled(ui_components::stylesheet(), &rename.old_class()),
            "{}: the old class is not styled",
            rename.old_class()
        );
    }
    const { assert!(ui_components::selectors::SURFACE >= 1) };
}
