//! The desktop's own files agree with the library's constants: the callers table that
//! `packaging/dist/desktop-callers.toml` ships (read with the real reader), the unit files of
//! the services it names, and the portal frontend's drop-in. The files are read, nothing else.

use caller_identity_core::{ExePath, LOADER_VARIABLES, Table, UnitName};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The workspace root: the nearest parent of this crate whose `Cargo.toml` has `[workspace]`.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|dir| {
            std::fs::read_to_string(dir.join("Cargo.toml"))
                .is_ok_and(|text| text.lines().any(|l| l.trim() == "[workspace]"))
        })
        .expect("a workspace root above caller-identity-core")
        .to_owned()
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn table() -> Table {
    let path = workspace_root().join("packaging/dist/desktop-callers.toml");
    Table::from_toml(&read(&path)).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn subdirs(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir).map_or_default(|entries| {
        entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect()
    });
    found.sort();
    found
}

/// The unit files of the components: `crates/<domain>[/<group>]/{dist,dbus,dist/systemd}/*.service`,
/// by file name (the first in path order when two have the same name).
fn unit_files() -> BTreeMap<String, PathBuf> {
    let mut found = BTreeMap::new();
    for domain in subdirs(&workspace_root().join("crates")) {
        let tops: Vec<PathBuf> = std::iter::once(domain.clone())
            .chain(subdirs(&domain))
            .collect();
        for top in tops {
            for sub in ["dist", "dbus", "dist/systemd"] {
                let mut files: Vec<PathBuf> =
                    std::fs::read_dir(top.join(sub)).map_or_default(|entries| {
                        entries.filter_map(Result::ok).map(|e| e.path()).collect()
                    });
                files.sort();
                for file in files {
                    let name = file.file_name().expect("a name").to_string_lossy();
                    if name.ends_with(".service") && file.is_file() {
                        found.entry(name.into_owned()).or_insert(file);
                    }
                }
            }
        }
    }
    found
}

/// The names `UnsetEnvironment=` holds in `text`. Lines add up; an empty assignment resets the
/// list (systemd.exec(5)); an entry `NAME=value` is kept as it is and so is not a bare name.
fn unset_names(text: &str) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for line in text.lines() {
        if let Some(rest) = line.trim().strip_prefix("UnsetEnvironment=") {
            if rest.trim().is_empty() {
                names.clear();
            }
            names.extend(rest.split_whitespace().map(str::to_owned));
        }
    }
    names
}

fn missing_loader_names(text: &str) -> Vec<&'static str> {
    let have = unset_names(text);
    LOADER_VARIABLES
        .iter()
        .copied()
        .filter(|name| !have.iter().any(|h| h == name))
        .collect()
}

#[test]
fn only_programs_have_no_unit_and_services_have_one_row() {
    // Programs known by their image alone, as opposed to services.
    const UNITLESS: [&str; 4] = ["settings", "updater_cli", "sandbox_launcher", "manager"];
    let table = table();
    for row in table.rows() {
        assert!(
            row.unit.is_some() || UNITLESS.contains(&row.role.as_str()),
            "{} has no unit, and only {UNITLESS:?} are known by their image alone",
            row.role
        );
    }
    assert!(
        table
            .rows()
            .iter()
            .any(|r| r.role.as_str() == "settings" && r.unit.is_none()),
        "Settings started by a launcher would not be known to the update service"
    );
    for role in table.rows().iter().map(|r| &r.role) {
        if UNITLESS.contains(&role.as_str()) {
            continue;
        }
        let rows = table.rows().iter().filter(|r| r.role == *role).count();
        assert_eq!(
            rows, 1,
            "the role {role} has {rows} rows; services have one"
        );
    }
}

#[test]
fn each_units_exec_start_is_one_of_its_rows_programs() {
    let table = table();
    let files = unit_files();
    let mut problems = Vec::new();
    for row in table.rows() {
        let Some(unit) = &row.unit else { continue };
        let name = unit.as_str();
        let Some(path) = files.get(name) else {
            // The distro's own unit (the portal frontend) is not ours to read.
            if name != "xdg-desktop-portal.service" {
                problems.push(format!("{name}: no unit file under crates/"));
            }
            continue;
        };
        let text = read(path);
        let start = text
            .lines()
            .find_map(|l| l.strip_prefix("ExecStart="))
            .and_then(|l| l.split_whitespace().next())
            .map(|w| w.trim_start_matches(['-', '@', '+', '!', ':']));
        match start.map(ExePath::new) {
            None => problems.push(format!("{name} ({}): no ExecStart", path.display())),
            Some(Err(e)) => problems.push(format!("{name}: ExecStart is not an image path: {e}")),
            Some(Ok(exe)) if !row.matches_image(&exe) => problems.push(format!(
                "{name}: ExecStart {} (in {}) is none of its row's exe paths",
                exe.to_text(),
                path.display()
            )),
            Some(Ok(_)) => {}
        }
    }
    assert!(problems.is_empty(), "{problems:#?}");
}

#[test]
fn every_trusted_unit_and_the_portal_drop_in_unset_all_loader_variables() {
    let table = table();
    let files = unit_files();
    // The table's units, but the portal frontend's (the distro's; its drop-in is checked
    // instead), plus the input service: it is not a table row, but root runs it and the
    // keymap daemon trusts its socket.
    let mut units: Vec<&str> = table
        .rows()
        .iter()
        .filter_map(|r| r.unit.as_ref().map(UnitName::as_str))
        .filter(|u| *u != "xdg-desktop-portal.service")
        .collect();
    units.push("bardo-input-broker.service");
    let mut problems = Vec::new();
    for unit in units {
        match files.get(unit) {
            None => problems.push(format!("{unit}: no unit file under crates/")),
            Some(path) => {
                let missing = missing_loader_names(&read(path));
                if !missing.is_empty() {
                    problems.push(format!(
                        "{unit} ({}) does not unset: {}",
                        path.display(),
                        missing.join(" ")
                    ));
                }
            }
        }
    }
    let dropin =
        workspace_root().join("packaging/session/xdg-desktop-portal.service.d/10-bardo.conf");
    let text = read(&dropin);
    let missing = missing_loader_names(&text);
    if !missing.is_empty() {
        problems.push(format!(
            "the portal drop-in does not unset: {}",
            missing.join(" ")
        ));
    }
    if !text.lines().any(|l| l.trim() == "[Service]") {
        problems.push("the portal drop-in has no [Service] section".to_owned());
    }
    assert!(problems.is_empty(), "{problems:#?}");
}
