//! The files under `dist/` agree with the code: the activation file and the unit name the bus name
//! the daemon claims, and the sample configuration reads.

use inference_daemon::config::InferdConfig;
use inference_daemon::peers::CallerTable;
use inference_daemon::settings::{self, CLASSES};
use settings_types::schema::{ForeignTables, KeyKind, KeySpec, Schema};
use std::path::PathBuf;

fn dist(file: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../dist")
        .join(file);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The shipped caller table, `dist/inferd-callers.toml`.
fn shipped_callers() -> CallerTable {
    CallerTable::from_toml_text(&dist("inferd-callers.toml")).expect("the shipped table reads")
}

fn key<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    text.lines()
        .find_map(|line| line.strip_prefix(&format!("{name}=")))
}

#[test]
fn the_activation_file_and_the_unit_name_the_bus_name_and_each_other() {
    let activation = dist("dbus/org.bardo.Inference1.service");
    let unit = dist("bardo-inferd.service");
    assert_eq!(
        key(&activation, "Name"),
        Some(inference_dbus::inference::INFERENCE_BUS)
    );
    assert_eq!(
        key(&unit, "BusName"),
        Some(inference_dbus::inference::INFERENCE_BUS)
    );
    assert_eq!(
        key(&activation, "SystemdService"),
        Some("bardo-inferd.service")
    );
    assert_eq!(key(&activation, "Exec"), key(&unit, "ExecStart"));
    assert_eq!(key(&unit, "Type"), Some("dbus"));
}

/// sec-3 follow-up: the shipped table gives the settings module to the Settings app's unit
/// (`org.bardo.Settings.service`, as accountd's callers.toml), never by the app's name alone.
#[test]
fn the_shipped_table_names_settings_by_its_unit() {
    let rows = shipped_callers().rows();
    let settings: Vec<_> = rows
        .callers
        .iter()
        .filter(|row| row.role == account_callers::CallerRole::Settings)
        .map(|row| (row.unit.as_deref(), row.app.to_string()))
        .collect();
    assert_eq!(
        settings,
        [(
            Some("org.bardo.Settings.service"),
            "org.bardo.Settings".to_owned()
        )]
    );
}

/// The shipped table lets the companion, reader and intents daemons choose where the assistant
/// runs (by unit, as placers), gives the shell its scope, and leaves the other apps plain.
#[test]
fn the_shipped_table_names_the_placers_and_the_shell_by_unit() {
    use inference_daemon::peers::Role;
    let callers = shipped_callers();
    let role = |unit: &str| {
        callers
            .resolve(unit)
            .map(|c| (c.app.name.to_string(), c.role))
    };
    for (unit, app) in [
        ("bardo-companiond.service", "org.bardo.Companion"),
        ("bardo-readerd.service", "org.bardo.Reader"),
        ("bardo-actiond.service", "org.bardo.Intents"),
    ] {
        assert_eq!(role(unit), Some((app.to_owned(), Role::Placer)), "{unit}");
    }
    assert_eq!(
        role("bardo-shell.service"),
        Some(("org.bardo.Shell".to_owned(), Role::Shell))
    );
    assert_eq!(
        role("shell-shell.scope"),
        None,
        "the old scope is no longer the shell"
    );
    assert_eq!(
        role("bardo-memoryd.service"),
        Some(("org.bardo.Memory".to_owned(), Role::App))
    );
}

#[test]
fn the_unit_gives_engines_the_gpu_and_only_this_computers_own_addresses() {
    let unit = dist("bardo-inferd.service");
    // Loopback is reachable (the local runtimes the probe asks, the agent endpoint's listener):
    // no private network, the internet families open, and every address but loopback denied.
    assert_eq!(key(&unit, "PrivateNetwork"), None);
    assert_eq!(
        key(&unit, "RestrictAddressFamilies"),
        Some("AF_UNIX AF_INET AF_INET6")
    );
    assert_eq!(key(&unit, "IPAddressDeny"), Some("any"));
    assert_eq!(key(&unit, "IPAddressAllow"), Some("localhost"));
    assert_eq!(key(&unit, "DevicePolicy"), Some("closed"));
    // Engines' JIT needs writable and executable memory, so the one line cuad's unit has is absent.
    assert_eq!(key(&unit, "MemoryDenyWriteExecute"), None);
    assert!(unit.lines().any(|line| line.starts_with("DeviceAllow=")));
    // What inferd writes is created by systemd before the sandbox starts.
    // The sockets' directory, and nothing for credentials: the AI keys are accountd's and a key
    // reaches inferd on a descriptor, never in a file.
    assert_eq!(key(&unit, "RuntimeDirectory"), Some("bardo/inference"));
    assert_eq!(key(&unit, "RuntimeDirectoryMode"), Some("0700"));
    assert_eq!(key(&unit, "StateDirectory"), Some("bardo/inference"));
    assert_eq!(key(&unit, "ReadWritePaths"), None);
    // The mount-namespace keys are the drop-in's, which the installer leaves out where the system
    // refuses a user service its own mount namespace.
    let sandbox = dist("bardo-inferd.service.d/sandbox.conf");
    assert_eq!(key(&sandbox, "ProtectHome"), Some("read-only"));
    assert_eq!(key(&sandbox, "ProtectSystem"), Some("strict"));
    assert_eq!(key(&sandbox, "ReadWritePaths"), None);
    for moved in ["ProtectHome", "ProtectSystem", "PrivateTmp"] {
        assert_eq!(key(&unit, moved), None, "{moved} belongs in the drop-in");
    }
}

#[test]
fn the_cloud_drop_in_opens_every_address_and_nothing_else() {
    let drop_in = dist("inferd-cloud.conf");
    assert_eq!(key(&drop_in, "IPAddressAllow"), Some("any"));
    // Only that line: the families and the rest of the sandbox stay as the unit has them.
    let settings: Vec<_> = drop_in
        .lines()
        .filter(|line| !line.starts_with('#') && line.contains('='))
        .collect();
    assert_eq!(settings.len(), 1, "{settings:?}");
}

#[test]
fn the_tailnet_drop_in_opens_the_networks_own_addresses_and_nothing_else() {
    let drop_in = dist("inferd-tailnet.conf");
    // The two blocks Tailscale gives a computer an address from, which are the ones
    // `inference_tailnet::is_network_address` accepts: the unit lets exactly those through.
    assert_eq!(
        key(&drop_in, "IPAddressAllow"),
        Some("100.64.0.0/10 fd7a:115c:a1e0::/48")
    );
    assert!(inference_tailnet::is_network_address(
        "100.64.0.1".parse().unwrap()
    ));
    assert!(inference_tailnet::is_network_address(
        "100.127.255.254".parse().unwrap()
    ));
    assert!(inference_tailnet::is_network_address(
        "fd7a:115c:a1e0::1".parse().unwrap()
    ));
    assert!(!inference_tailnet::is_network_address(
        "100.128.0.1".parse().unwrap()
    ));
    // Only that line, and not the internet: the base unit is untouched.
    let settings: Vec<_> = drop_in
        .lines()
        .filter(|line| !line.starts_with('#') && line.contains('='))
        .collect();
    assert_eq!(settings.len(), 1, "{settings:?}");
    assert!(
        settings.iter().all(|line| !line.contains("any")),
        "the internet stays out of reach"
    );
    assert_eq!(
        key(&dist("bardo-inferd.service"), "IPAddressAllow"),
        Some("localhost")
    );
}

#[test]
fn the_shipped_table_names_the_callers_the_design_calls_for() {
    let callers = shipped_callers();
    let cua = callers.resolve("bardo-computer-use.service").expect("cuad");
    assert_eq!(cua.role, inference_daemon::peers::Role::Cua);
    for exe in [
        "bardo-memoryd",
        "bardo-actiond",
        "bardo-companiond",
        "bardo-readerd",
        "bardo-voiced",
    ] {
        let caller = callers
            .resolve(&format!("{exe}.service"))
            .unwrap_or_else(|| panic!("{exe}"));
        // The three that docket's places reach through are placers: they open as an app does.
        let want = match exe {
            "bardo-companiond" | "bardo-readerd" | "bardo-actiond" => {
                inference_daemon::peers::Role::Placer
            }
            _ => inference_daemon::peers::Role::App,
        };
        assert_eq!(caller.role, want, "{exe}");
    }
}

#[test]
fn the_agent_launcher_is_listed_by_the_machine_that_runs_it_never_by_default() {
    let shipped = dist("inferd-callers.toml");
    assert!(
        shipped_callers()
            .rows()
            .callers
            .iter()
            .all(|row| row.role != account_callers::CallerRole::AgentLauncher),
        "no launcher by default"
    );
    // The line the table's comment shows is the one a machine writes, uncommented.
    let documented = shipped
        .lines()
        .find_map(|line| line.strip_prefix("# agent_launcher = "))
        .map(|rest| format!("agent_launcher = {rest}"))
        .expect("the documented line");
    let mine = CallerTable::from_toml_text(&documented).expect("the documented line reads");
    let launcher = mine.resolve("bardo-agent-acp.service").expect("the unit");
    assert_eq!(launcher.role, inference_daemon::peers::Role::AgentLauncher);
}

/// Who is calling is the system's table's to say: the sample configuration names no caller.
#[test]
fn the_sample_configuration_gives_no_callers() {
    let shipped = dist("inferd.toml");
    InferdConfig::from_toml(&shipped).expect("the sample reads");
    assert!(
        !shipped.lines().any(|line| line.starts_with("[callers")),
        "no callers table"
    );
}

fn schema() -> Schema {
    Schema::from_toml(&dist("inference.settings.toml")).expect("the schema parses and is hands-off")
}

#[test]
fn the_schema_names_the_three_tables_inferd_keeps_beside_its_rows() {
    let expected = ForeignTables(vec![
        "engines".to_owned(),
        "probe".to_owned(),
        "callers".to_owned(),
    ]);
    assert_eq!(schema().foreign, expected);
}

fn schema_keys() -> Vec<KeySpec> {
    schema().key
}

/// What a configuration makes inferd do, as far as these rows go: the policy as it answers for
/// every class, the tier map, Automatic, the spend line and the structured limits.
fn effect(text: &str) -> (String, Vec<String>) {
    let config = InferdConfig::from_toml(text).unwrap_or_else(|e| panic!("{text}: {e}"));
    let resolved = settings::resolve(&config);
    let s = resolved.settings;
    let floors: Vec<_> = CLASSES.iter().map(|class| s.policy.floor(*class)).collect();
    let limits = config.ai.resolve().limits;
    let mut rejected = resolved.rejected;
    rejected.extend(config.ai.resolve().rejected.iter().map(|p| (*p).to_owned()));
    (
        format!(
            "{:?} {floors:?} {:?} {:?} {:?} {:?} {:?} {:?} {:?} {:?} {limits:?}",
            s.policy.local_only,
            s.tiers,
            s.auto,
            s.spend,
            s.describe_images,
            s.my_network,
            s.agent_endpoint,
            s.tailnet_serve,
            s.lengths
        ),
        rejected,
    )
}

fn file_with(path: &str, value: toml::Value) -> String {
    let mut table = toml::Table::new();
    let mut at = &mut table;
    let parts: Vec<&str> = path.split('.').collect();
    for part in &parts[..parts.len() - 1] {
        at = at
            .entry((*part).to_owned())
            .or_insert_with(|| toml::Value::Table(toml::Table::new()))
            .as_table_mut()
            .expect("table");
    }
    at.insert(parts[parts.len() - 1].to_owned(), value);
    toml::to_string(&table).expect("toml")
}

/// A value of the row other than its default, when it has one.
fn other_than_default(key: &KeySpec) -> Option<toml::Value> {
    let text = |v: &str| toml::Value::String(v.to_owned());
    match &key.kind {
        KeyKind::Toggle => key.default.as_bool().map(|on| toml::Value::Boolean(!on)),
        KeyKind::Segmented { variants } | KeyKind::Menu { variants } => variants
            .iter()
            .find(|v| toml::Value::String((*v).clone()) != key.default)
            .map(|v| text(v)),
        KeyKind::Bounded { min, max, .. } => Some(toml::Value::Integer(
            if key.default == toml::Value::Integer(*min) {
                *max
            } else {
                *min
            },
        )),
        _ => None,
    }
}

#[test]
fn every_row_of_the_schema_is_read_by_inferd_at_its_path_and_changes_what_it_does() {
    let nothing = effect("");
    assert_eq!(nothing.1, Vec::<String>::new());
    for key in schema_keys() {
        let path = key.path.0.as_str();
        // Its default, written at its path, is accepted and is what no file says.
        let said_default = effect(&file_with(path, key.default.clone()));
        assert_eq!(said_default.1, Vec::<String>::new(), "{path} default");
        assert_eq!(
            said_default.0, nothing.0,
            "{path}: the schema default is the code's"
        );
        if let Some(value) = other_than_default(&key) {
            let changed = effect(&file_with(path, value.clone()));
            assert_eq!(changed.1, Vec::<String>::new(), "{path} = {value}");
            assert_ne!(changed.0, nothing.0, "{path} = {value} changes nothing");
        } else {
            // A row with one value (`ai.auto.mode`): inferd still reads it at its path, and
            // names it when it holds anything else.
            let refused = effect(&file_with(path, toml::Value::String("bogus".into())));
            assert_eq!(refused.1, vec![path.to_owned()], "{path}");
        }
    }
}

#[test]
fn the_sample_configuration_never_enables_the_prompt_recorder() {
    let live: Vec<_> = dist("inferd.toml")
        .lines()
        .filter(|line| line.trim_start().starts_with("record"))
        .map(str::to_owned)
        .collect();
    assert!(live.is_empty(), "{live:?}");
}

#[test]
fn the_test_proc_root_is_in_no_default_feature_and_no_unit() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("manifest");
    let manifest: toml::Table = manifest.parse().expect("toml");
    let features = manifest["features"].as_table().expect("features");
    let default = features.get("default").and_then(toml::Value::as_array);
    assert!(
        default.is_none_or(|on| on.iter().all(|f| f.as_str() != Some("test-proc-root"))),
        "default features must not enable test-proc-root"
    );
    for file in [
        "bardo-inferd.service",
        "inferd.toml",
        "dbus/org.bardo.Inference1.service",
    ] {
        assert!(!dist(file).contains("BARDO_INFERD_PROC_ROOT"), "{file}");
    }
}

/// The shipped table knows bardo-agent-acp in its scope by its program, so the editor-agent launcher's
/// planner turns reach inferd on an installed desktop; a scope's name alone gives no program.
#[test]
fn the_shipped_table_names_docket_acps_programs() {
    let rows = shipped_callers().rows();
    let acp = rows
        .callers
        .iter()
        .find(|row| row.unit.as_deref() == Some("bardo-agent-acp.scope"))
        .expect("a bardo-agent-acp.scope row");
    assert_eq!(acp.app.to_string(), "org.bardo.Companion");
    assert_eq!(acp.role, account_callers::CallerRole::App);
    assert_eq!(
        acp.exe,
        [
            "/usr/bin/bardo-agent-acp",
            "/usr/local/bin/bardo-agent-acp",
            "/opt/bardo/releases/*/bin/bardo-agent-acp"
        ]
    );
}
