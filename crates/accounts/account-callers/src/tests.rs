use super::*;

fn row(app: &str, unit: Option<&str>, role: CallerRole) -> CallerRow {
    CallerRow {
        app: AppName::parse(app).expect("name"),
        unit: unit.map(str::to_owned),
        role,
        exe: Vec::new(),
        name: None,
    }
}

fn name(text: &str) -> AppName {
    AppName::parse(text).expect("name")
}

fn table() -> CallerTable {
    CallerTable {
        callers: vec![
            row(
                "org.bardo.Inference",
                Some("bardo-inferd.service"),
                CallerRole::AccountDaemon,
            ),
            row("org.bardo.Settings", None, CallerRole::Settings),
            row(
                "org.bardo.ComputerUse",
                Some("bardo-computer-use.service"),
                CallerRole::ComputerUse,
            ),
        ],
    }
}

#[test]
fn a_unit_resolves_to_its_app_and_role() {
    let cases = [
        (
            "bardo-inferd.service",
            Some(("org.bardo.Inference", CallerRole::AccountDaemon)),
        ),
        (
            "bardo-computer-use.service",
            Some(("org.bardo.ComputerUse", CallerRole::ComputerUse)),
        ),
        // A row without a unit names no unit.
        ("bardo-settings.service", None),
        ("org.bardo.Settings", None),
    ];
    for (unit, want) in cases {
        let got = table()
            .resolve_unit(unit)
            .map(|c| (c.app.name.to_string(), c.role));
        assert_eq!(got, want.map(|(n, r)| (n.to_owned(), r)), "{unit}");
    }
    assert_eq!(
        table()
            .resolve_unit("bardo-inferd.service")
            .map(|c| c.app.isolation),
        Some(Isolation::Unsandboxed)
    );
}

#[test]
fn an_apps_role_is_its_row_s_and_app_for_an_unlisted_app() {
    assert_eq!(
        table().role_of(&name("org.bardo.Settings")),
        CallerRole::Settings
    );
    assert_eq!(table().role_of(&name("org.example.Any")), CallerRole::App);
}

#[test]
fn a_scope_named_after_a_units_app_gets_no_role_of_the_unit() {
    for app in ["org.bardo.Inference", "org.bardo.ComputerUse"] {
        assert_eq!(table().role_of(&name(app)), CallerRole::App, "{app}");
    }
}

#[test]
fn a_user_row_replaces_the_system_row_of_the_same_unit_or_app() {
    let user = CallerTable {
        callers: vec![
            row(
                "org.example.MyInferd",
                Some("bardo-inferd.service"),
                CallerRole::App,
            ),
            row("org.bardo.Settings", None, CallerRole::App),
        ],
    };
    let merged = CallerTable::layered(table(), user);
    let unit = merged.resolve_unit("bardo-inferd.service").expect("named");
    assert_eq!(
        (unit.app.name.as_str(), unit.role),
        ("org.example.MyInferd", CallerRole::App)
    );
    assert_eq!(merged.role_of(&name("org.bardo.Settings")), CallerRole::App);
    assert_eq!(merged.callers.len(), 3);
}

#[test]
fn a_table_round_trips_through_its_serde_form() {
    let json = serde_json::to_string(&table()).expect("json");
    assert_eq!(
        serde_json::from_str::<CallerTable>(&json).expect("table"),
        table()
    );
    assert!(json.contains(r#""role":"account_daemon""#));
}

#[test]
fn a_row_without_a_unit_deserializes() {
    let table: CallerTable =
        serde_json::from_str(r#"{"caller":[{"app":"org.bardo.Settings","role":"settings"}]}"#)
            .expect("table");
    assert_eq!(table.callers[0].unit, None);
}

#[test]
fn the_last_row_with_a_name_names_the_app_whatever_its_unit() {
    let named = |app: &str, unit: Option<&str>, title: &str| CallerRow {
        name: Some(AppTitle(title.to_owned())),
        ..row(app, unit, CallerRole::App)
    };
    let table = CallerTable {
        callers: vec![
            named("org.bardo.Sync", Some("bardo-syncd.service"), "Sync"),
            row("org.bardo.Settings", None, CallerRole::Settings),
            named("org.bardo.Sync", None, "Mine"),
            row("org.bardo.Sync", None, CallerRole::App),
        ],
    };
    assert_eq!(
        table.title_of(&name("org.bardo.Sync")),
        Some(&AppTitle("Mine".to_owned()))
    );
    assert_eq!(table.title_of(&name("org.bardo.Settings")), None);
    assert_eq!(table.title_of(&name("org.bardo.Nobody")), None);
}

#[test]
fn a_user_row_cannot_claim_a_fixed_role_for_a_new_app_or_a_unit() {
    let user = CallerTable {
        callers: vec![
            row(
                "org.example.Mine",
                Some("mine.service"),
                CallerRole::SheetHost,
            ),
            row("org.example.Other", None, CallerRole::Settings),
            row(
                "org.example.Fake",
                Some("bardo-shell.service"),
                CallerRole::SheetHost,
            ),
        ],
    };
    assert_eq!(CallerTable::layered(table(), user), table());
}

#[test]
fn a_user_row_cannot_raise_a_system_app_to_a_fixed_role() {
    let system = CallerTable {
        callers: vec![row("org.example.Notes", None, CallerRole::App)],
    };
    let user = CallerTable {
        callers: vec![row("org.example.Notes", None, CallerRole::ComputerUse)],
    };
    let merged = CallerTable::layered(system, user);
    assert_eq!(merged.role_of(&name("org.example.Notes")), CallerRole::App);
}

#[test]
fn a_user_row_cannot_replace_a_fixed_system_row_with_another_fixed_role() {
    let user = CallerTable {
        callers: vec![row(
            "org.example.Mine",
            Some("bardo-computer-use.service"),
            CallerRole::AccountDaemon,
        )],
    };
    let merged = CallerTable::layered(table(), user);
    assert_eq!(
        merged
            .resolve_unit("bardo-computer-use.service")
            .map(|c| c.role),
        Some(CallerRole::ComputerUse)
    );
}

#[test]
fn a_user_row_can_still_add_a_plain_app_or_an_agent() {
    let user = CallerTable {
        callers: vec![
            row("org.example.Plain", None, CallerRole::App),
            row(
                "org.example.Helper",
                Some("helper.service"),
                CallerRole::Agent,
            ),
        ],
    };
    let merged = CallerTable::layered(table(), user);
    assert_eq!(merged.callers.len(), 5);
    assert_eq!(merged.role_of(&name("org.example.Plain")), CallerRole::App);
    assert_eq!(
        merged.resolve_unit("helper.service").map(|c| c.role),
        Some(CallerRole::Agent)
    );
}

/// An app the system's table names is who its system rows say: a user row cannot give it
/// another unit or a row without one, though it may still lower one of its own rows.
#[test]
fn a_user_row_cannot_name_an_app_the_system_owns() {
    type Case<'a> = (
        &'a str,
        CallerRow,
        Option<(&'a str, CallerRole)>,
        Option<(&'a str, CallerRole)>,
    );

    let system = || {
        let mut system = table();
        system.callers.push(row(
            "org.bardo.Intents",
            Some("bardo-actiond.service"),
            CallerRole::Agent,
        ));
        system
    };
    // (case, the user's row, what bardo-actiond.service and helper.service resolve to after)
    let intents = Some(("org.bardo.Intents", CallerRole::Agent));
    let cases: [Case; 4] = [
        (
            "another unit as the agent",
            row(
                "org.bardo.Intents",
                Some("helper.service"),
                CallerRole::Agent,
            ),
            intents,
            None,
        ),
        (
            "another unit as an app",
            row("org.bardo.Intents", Some("helper.service"), CallerRole::App),
            intents,
            None,
        ),
        (
            "a row without a unit",
            row("org.bardo.Intents", None, CallerRole::Agent),
            intents,
            None,
        ),
        (
            "its own unit, lowered",
            row(
                "org.bardo.Intents",
                Some("bardo-actiond.service"),
                CallerRole::App,
            ),
            Some(("org.bardo.Intents", CallerRole::App)),
            None,
        ),
    ];
    for (case, mine, intentd, helper) in cases {
        let merged = CallerTable::layered(
            system(),
            CallerTable {
                callers: vec![mine],
            },
        );
        let found = |unit: &str| {
            merged
                .resolve_unit(unit)
                .map(|c| (c.app.name.to_string(), c.role))
        };
        let want = |w: Option<(&str, CallerRole)>| w.map(|(n, r)| (n.to_owned(), r));
        assert_eq!(found("bardo-actiond.service"), want(intentd), "{case}");
        assert_eq!(found("helper.service"), want(helper), "{case}");
        assert_eq!(
            merged.role_of(&name("org.bardo.Intents")),
            CallerRole::App,
            "{case}"
        );
    }
}
