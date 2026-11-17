use super::*;
use account_core::Isolation;

fn table() -> CallerTable {
    CallerTable::from_toml_text(
        r#"
cua = ["bardo-computer-use.service"]
agent_launcher = ["bardo-agent-acp.service"]
[apps]
"org.bardo.Memory" = ["bardo-memoryd.service"]
"org.bardo.Mail" = ["mailo.service", "mailo-dev.service"]
"org.bardo.Sneaky" = ["bardo-computer-use.service", "bardo-agent-acp.service"]
"#,
    )
    .expect("table")
}

fn named(caller: &Caller) -> (&str, Role, Isolation) {
    (caller.app.name.as_str(), caller.role, caller.app.isolation)
}

#[test]
fn a_unit_is_the_caller_the_table_names_for_it() {
    let table = table();
    let memory = table.resolve("bardo-memoryd.service").expect("memoryd");
    assert_eq!(
        named(&memory),
        ("org.bardo.Memory", Role::App, Isolation::Unsandboxed)
    );
    for exe in ["mailo.service", "mailo-dev.service"] {
        let mail = table.resolve(exe).expect("mailo");
        assert_eq!(mail.app.name.as_str(), "org.bardo.Mail");
    }
}

#[test]
fn cuad_is_the_one_computer_use_caller_and_an_app_entry_cannot_claim_its_unit() {
    let cuad = table().resolve("bardo-computer-use.service").expect("cuad");
    assert_eq!(
        named(&cuad),
        ("org.bardo.ComputerUse", Role::Cua, Isolation::Unsandboxed)
    );
}

#[test]
fn the_agent_launcher_is_the_unit_the_table_names_and_an_app_entry_cannot_claim_it() {
    let launcher = table()
        .resolve("bardo-agent-acp.service")
        .expect("the launcher");
    assert_eq!(
        named(&launcher),
        (
            "org.bardo.AgentLauncher",
            Role::AgentLauncher,
            Isolation::Unsandboxed
        )
    );
    assert_eq!(
        CallerTable::default()
            .with_agent_launcher(["x.service".to_owned()].into())
            .resolve("x.service")
            .map(|c| c.role),
        Some(Role::AgentLauncher)
    );
}

#[test]
fn a_program_the_table_does_not_name_is_nobody() {
    let table = table();
    for exe in ["bash.service", "bardo-memoryd.service.d", ""] {
        assert_eq!(table.resolve(exe), None, "{exe:?}");
    }
    assert_eq!(CallerTable::default().resolve("mailo.service"), None);
}

#[test]
fn a_table_with_a_malformed_app_name_does_not_parse() {
    CallerTable::from_toml_text("[apps]\n\"not a name\" = [\"x.service\"]\n").unwrap_err();
    CallerTable::from_toml_text("cua = 3").unwrap_err();
}

#[test]
fn a_refused_table_says_which_way_it_was_refused_and_keeps_the_parser_error() {
    assert!(matches!(
        CallerTable::from_toml_text("cua = 3"),
        Err(TableError::Toml(_))
    ));
    assert!(matches!(
        CallerTable::from_toml_text("settings = [\"not a name\"]\n"),
        Err(TableError::BadSettingsEntry(bad)) if bad == "not a name"
    ));
}

fn placed_table() -> CallerTable {
    CallerTable::from_toml_text(
        r#"
cua = ["bardo-computer-use.service"]
shell = ["bardo-shell.service"]
places = ["bardo-companiond.service", "bardo-readerd.service"]
[apps]
"org.bardo.Companion" = ["bardo-companiond.service"]
"org.bardo.Reader" = ["bardo-readerd.service"]
"org.bardo.Memory" = ["bardo-memoryd.service"]
"#,
    )
    .expect("table")
}

#[test]
fn the_places_rows_make_placers_the_shell_row_makes_the_shell_and_the_rest_stay_apps() {
    let table = placed_table();
    let role = |unit: &str| table.resolve(unit).map(|c| c.role);
    assert_eq!(role("bardo-companiond.service"), Some(Role::Placer));
    assert_eq!(role("bardo-readerd.service"), Some(Role::Placer));
    assert_eq!(role("bardo-memoryd.service"), Some(Role::App));
    assert_eq!(role("bardo-shell.service"), Some(Role::Shell));
    assert_eq!(role("bardo-actiond.service"), None);
}

#[test]
fn settings_the_shell_the_companion_and_cuad_list_places_and_only_placers_choose_them() {
    let table = placed_table().with_settings(["org.bardo.Settings.service".to_owned()].into());
    let who = |unit: &str| table.resolve(unit).expect(unit);
    let list = |unit: &str| who(unit).may_list_places();
    let choose = |unit: &str| who(unit).may_choose_places();
    assert!(list("org.bardo.Settings.service"));
    assert!(list("bardo-shell.service"));
    assert!(list("bardo-companiond.service"));
    assert!(
        list("bardo-computer-use.service"),
        "cuad reads whether a ready model can see"
    );
    assert!(!who("bardo-computer-use.service").may_list_engines());
    assert!(!list("bardo-readerd.service"));
    assert!(!list("bardo-memoryd.service"));
    assert!(choose("bardo-companiond.service") && choose("bardo-readerd.service"));
    for unit in [
        "bardo-memoryd.service",
        "bardo-shell.service",
        "org.bardo.Settings.service",
    ] {
        assert!(!choose(unit), "{unit}");
    }
}

#[test]
fn a_places_unit_that_is_no_app_and_a_shell_entry_that_is_no_service_are_refused() {
    assert!(matches!(
        CallerTable::from_toml_text("places = [\"x.service\"]\n[apps]\n\"org.bardo.A\" = [\"y.service\"]\n"),
        Err(TableError::PlacesNotAnApp(unit)) if unit == "x.service"
    ));
    assert!(matches!(
        CallerTable::from_toml_text("shell = [\"bardo-shell\"]\n"),
        Err(TableError::BadShellEntry(bad)) if bad == "bardo-shell"
    ));
    // A scope has no main process to check, so it cannot be the shell.
    assert!(matches!(
        CallerTable::from_toml_text("shell = [\"bardo-shell.scope\"]\n"),
        Err(TableError::BadShellEntry(bad)) if bad == "bardo-shell.scope"
    ));
}

#[tokio::test]
async fn introduced_connections_are_known_and_others_are_not() {
    let peers = TablePeers::new();
    let caller = table().resolve("mailo.service").expect("mailo");
    peers.introduce(":1.42", caller.clone());
    assert_eq!(peers.caller_of(":1.42").await, Some(caller.clone()));
    assert_eq!(peers.caller_of(":1.43").await, None);
    let shared = std::sync::Arc::new(peers);
    assert_eq!(shared.caller_of(":1.42").await, Some(caller));
}

#[test]
fn the_table_is_rows_of_the_shared_table_with_cuad_first() {
    let rows = table().rows();
    let roles: Vec<_> = rows.callers.iter().map(|row| row.role).collect();
    assert_eq!(roles[0], caller_role_types::CallerRole::Cua);
    assert_eq!(roles[1], caller_role_types::CallerRole::AgentLauncher);
    assert!(
        roles[2..]
            .iter()
            .all(|r| *r == caller_role_types::CallerRole::App)
    );
    assert_eq!(rows.callers.len(), 7);
    // The first row for cuad's unit is cuad's, not the sneaky app's.
    let cuad = rows
        .resolve_unit("bardo-computer-use.service")
        .expect("cuad");
    assert_eq!(cuad.role, caller_role_types::CallerRole::Cua);
    let launcher = rows
        .resolve_unit("bardo-agent-acp.service")
        .expect("launcher");
    assert_eq!(launcher.role, caller_role_types::CallerRole::AgentLauncher);
}

/// sec-3 follow-up: the settings module's caller is the Settings app's unit, not its name. An
/// app-name entry (the old form) stands for `<app>.service`; the row comes before the apps', so
/// an app entry cannot claim the unit; a malformed entry is refused.
#[test]
fn settings_is_a_unit_and_an_app_name_stands_for_its_service() {
    for entry in ["org.bardo.Settings.service", "org.bardo.Settings"] {
        let table = CallerTable::from_toml_text(&format!(
            "settings = [\"{entry}\"]\n[apps]\n\"org.example.Sneaky\" = [\"org.bardo.Settings.service\"]\n"
        ))
        .expect("table");
        let rows = table.rows();
        let settings: Vec<_> = rows
            .callers
            .iter()
            .filter(|row| row.role == caller_role_types::CallerRole::Settings)
            .map(|row| (row.unit.clone(), row.app.to_string()))
            .collect();
        assert_eq!(
            settings,
            [(
                Some("org.bardo.Settings.service".to_owned()),
                "org.bardo.Settings".to_owned()
            )],
            "{entry}"
        );
        let caller = table
            .resolve("org.bardo.Settings.service")
            .expect("the unit");
        assert_eq!(
            named(&caller),
            ("org.bardo.Settings", Role::Settings, Isolation::Unsandboxed),
            "{entry}"
        );
    }
    for bad in ["not a name", ".service"] {
        assert!(
            CallerTable::from_toml_text(&format!("settings = [\"{bad}\"]\n")).is_err(),
            "{bad}"
        );
    }
}

/// sec-3 follow-up, through `account_dbus`'s fixture tree (`<root>/<pid>/cgroup`,
/// `<root>/units/<unit>` for a unit's main process): only the main process of the Settings unit
/// has the role; a process moved into its cgroup, and one in an app scope named like the app,
/// do not.
#[cfg(feature = "test-proc-root")]
#[test]
fn only_the_settings_units_main_process_is_settings() {
    let root = base_testkit::Scratch::new("inferd-settings-unit");
    let slice = "/user.slice/user-1000.slice/user@1000.service/app.slice";
    let process = |pid: u32, unit: &str| {
        let dir = root.join(pid.to_string());
        std::fs::create_dir_all(&dir).expect("proc dir");
        std::fs::write(dir.join("cgroup"), format!("0::{slice}/{unit}\n")).expect("cgroup");
    };
    process(20, "org.bardo.Settings.service");
    process(21, "org.bardo.Settings.service");
    process(22, "app-org.bardo.Settings-7.scope");
    std::fs::create_dir_all(root.join("units")).expect("units");
    std::fs::write(root.join("units/org.bardo.Settings.service"), "20\n").expect("main pid");
    let rows = CallerTable::from_toml_text("settings = [\"org.bardo.Settings.service\"]\n")
        .expect("table")
        .rows();
    let role = |pid| {
        account_callers::ProcCallers::caller_of_pid(&root, pid, &rows).map(|caller| caller.role)
    };
    assert_eq!(role(20), Some(caller_role_types::CallerRole::Settings));
    assert_eq!(role(21), None, "in the unit's cgroup, not its main process");
    assert_ne!(
        role(22),
        Some(caller_role_types::CallerRole::Settings),
        "an app scope of the app's name"
    );
}

#[test]
fn an_entry_may_name_the_programs_that_run_in_its_unit() {
    let table = CallerTable::from_toml_text(
        r#"
agent_launcher = [{ unit = "bardo-agent-acp.scope", exe = ["/usr/bin/bardo-agent-acp"] }]
places = ["bardo-companiond.service"]
[apps]
"org.bardo.Companion" = [
    "bardo-companiond.service",
    { unit = "bardo-agent-acp-turn.scope", exe = ["/usr/bin/bardo-agent-acp", "/opt/bardo/releases/*/bin/bardo-agent-acp"] },
]
"#,
    )
    .expect("table");
    let rows = table.rows().callers;
    let row = |unit: &str| {
        rows.iter()
            .find(|row| row.unit.as_deref() == Some(unit))
            .map(|row| (row.role, row.exe.clone()))
    };
    assert_eq!(
        row("bardo-agent-acp.scope"),
        Some((
            CallerRole::AgentLauncher,
            vec!["/usr/bin/bardo-agent-acp".to_owned()]
        ))
    );
    assert_eq!(
        row("bardo-agent-acp-turn.scope"),
        Some((
            CallerRole::App,
            vec![
                "/usr/bin/bardo-agent-acp".to_owned(),
                "/opt/bardo/releases/*/bin/bardo-agent-acp".to_owned()
            ]
        ))
    );
    // A bare name is as before: its program is the one the shared table gives the unit's name.
    assert_eq!(
        row("bardo-companiond.service"),
        Some((CallerRole::Agent, Vec::new()))
    );
}
