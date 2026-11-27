use super::super::{Install, installed};
use super::*;
use account_core::AppName;

fn row(app: &str, unit: Option<&str>, role: CallerRole, exe: &[&str]) -> CallerRow {
    CallerRow {
        app: AppName::parse(app).expect("name"),
        unit: unit.map(str::to_owned),
        role,
        exe: exe.iter().map(|image| (*image).to_owned()).collect(),
        name: None,
    }
}

fn table_of(rows: Vec<CallerRow>) -> Table {
    caller_identity_table(&CallerTable { callers: rows })
}

fn unit(name: &str) -> caller_identity_core::UnitName {
    caller_identity_core::UnitName::new(name).expect("unit")
}

fn images_of(table: &Table, name: &str) -> Option<Vec<String>> {
    table
        .row_for_unit(&unit(name))
        .map(|row| row.exe.iter().map(|p| p.as_str().to_owned()).collect())
}

#[test]
fn a_daemon_unit_without_a_program_is_taken_to_be_installed_as_a_daemon() {
    let table = table_of(vec![row(
        "org.bardo.Inference",
        Some("bardo-inferd.service"),
        CallerRole::AccountDaemon,
        &[],
    )]);
    let found = table
        .row_for_unit(&unit("bardo-inferd.service"))
        .expect("a row");
    assert_eq!(found.app.as_str(), "org.bardo.Inference");
    assert_eq!(found.role.as_str(), "account_daemon");
    assert_eq!(
        images_of(&table, "bardo-inferd.service"),
        Some(installed(Install::Libexec, "bardo-inferd"))
    );
}

#[test]
fn a_rows_own_programs_replace_the_convention() {
    let table = table_of(vec![row(
        "org.bardo.Shell",
        Some("bardo-shell.service"),
        CallerRole::SheetHost,
        &[
            "/usr/bin/bardo-shell",
            "/opt/bardo/releases/*/bin/bardo-shell",
        ],
    )]);
    assert_eq!(
        images_of(&table, "bardo-shell.service"),
        Some(vec![
            "/usr/bin/bardo-shell".to_owned(),
            "/opt/bardo/releases/*/bin/bardo-shell".to_owned()
        ])
    );
}

#[test]
fn a_row_caller_identity_could_not_judge_is_left_out_and_the_others_stand() {
    let table = table_of(vec![
        // An app by name: no unit.
        row("org.bardo.Settings", None, CallerRole::Settings, &[]),
        // A unit that is not a daemon's name, and no program: nothing proves it.
        row(
            "org.bardo.Temor",
            Some("org.bardo.Temor.service"),
            CallerRole::Terminal,
            &[],
        ),
        // A program path the caller-identity library refuses (two stars in a segment).
        row(
            "org.bardo.Bad",
            Some("bad.service"),
            CallerRole::Agent,
            &["/opt/*/*x*/bad"],
        ),
        row(
            "org.bardo.Intents",
            Some("bardo-actiond.service"),
            CallerRole::Agent,
            &[],
        ),
    ]);
    assert!(
        table
            .row_for_unit(&unit("org.bardo.Temor.service"))
            .is_none()
    );
    assert!(table.row_for_unit(&unit("bad.service")).is_none());
    assert!(table.row_for_unit(&unit("bardo-actiond.service")).is_some());
}

#[test]
fn the_first_row_for_a_unit_wins() {
    let table = table_of(vec![
        row(
            "org.bardo.ComputerUse",
            Some("bardo-computer-use.service"),
            CallerRole::ComputerUse,
            &[],
        ),
        row(
            "org.example.Mine",
            Some("bardo-computer-use.service"),
            CallerRole::App,
            &[],
        ),
    ]);
    let found = table
        .row_for_unit(&unit("bardo-computer-use.service"))
        .expect("a row");
    assert_eq!(
        (found.app.as_str(), found.role.as_str()),
        ("org.bardo.ComputerUse", "computer_use")
    );
}

#[test]
fn the_table_always_holds_the_unit_managers_row() {
    let table = table_of(Vec::new());
    assert!(table.has_role(&peers_callers::MANAGER));
    assert_eq!(table.rows().len(), 1);
}
