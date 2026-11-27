//! Every provider file the repo ships parses, round-trips through TOML, and lays over the
//! others by id.

use account_provider::{ProviderSet, ProviderSpec, parse_provider};
use base_ids::word::word as word_of;
use std::path::PathBuf;

fn shipped() -> Vec<(PathBuf, ProviderSpec)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../providers");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("providers directory")
        .map(|entry| entry.expect("entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let text = std::fs::read_to_string(&path).expect("readable");
            let spec = parse_provider(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            (path, spec)
        })
        .collect()
}

#[test]
fn every_shipped_provider_file_parses_and_is_named_by_its_id() {
    let files = shipped();
    for (path, spec) in files {
        let stem = path.file_stem().and_then(|s| s.to_str()).expect("stem");
        assert_eq!(spec.id.as_str(), stem, "{}", path.display());
    }
}

#[test]
fn a_provider_spec_round_trips_through_toml() {
    for (path, spec) in shipped() {
        let text = toml::to_string(&spec).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(
            parse_provider(&text).as_ref(),
            Ok(&spec),
            "{}",
            path.display()
        );
    }
}

#[test]
fn a_user_file_replaces_the_system_file_of_the_same_id() {
    let specs: Vec<ProviderSpec> = shipped().into_iter().map(|(_, spec)| spec).collect();
    let mut mine = specs[0].clone();
    mine.label = "My own".into();
    let set = ProviderSet::layered(specs.clone(), vec![mine.clone()]);
    assert_eq!(set.specs().len(), specs.len());
    assert_eq!(set.get(&mine.id).map(|s| s.label.as_str()), Some("My own"));
}

fn shipped_spec(id: &str) -> ProviderSpec {
    shipped()
        .into_iter()
        .map(|(_, spec)| spec)
        .find(|spec| spec.id.as_str() == id)
        .unwrap_or_else(|| panic!("{id} ships"))
}

#[test]
fn the_compiled_in_files_are_the_whole_providers_directory_byte_for_byte() {
    let on_disk = shipped();
    assert_eq!(account_provider::SHIPPED_FILES.len(), on_disk.len());
    // By id, not by position: `google-ai.toml` sorts before `google.toml` as a path.
    for (path, _) in &on_disk {
        let stem = path.file_stem().and_then(|s| s.to_str()).expect("stem");
        let (_, text) = account_provider::SHIPPED_FILES
            .iter()
            .find(|(id, _)| *id == stem)
            .unwrap_or_else(|| panic!("{stem} is not compiled in"));
        assert_eq!(
            *text,
            std::fs::read_to_string(path).expect("readable"),
            "{stem}"
        );
    }
    let set = account_provider::shipped();
    assert_eq!(set.specs().len(), on_disk.len());
    for (id, _) in account_provider::SHIPPED_FILES {
        let id = account_core::ProviderId::parse(id).expect("id");
        assert!(set.get(&id).is_some(), "{id}");
    }
}

#[test]
fn only_microsofts_storage_rows_declare_linked_origins_the_app_folder_at_sign_in() {
    use account_capability::{Ask, StorageScope};
    use account_capability::{Capability, CapabilityKind};
    type Row = (
        String,
        CapabilityKind,
        Option<StorageScope>,
        Ask,
        Vec<String>,
    );
    let rows: Vec<Row> = shipped()
        .into_iter()
        .flat_map(|(_, spec)| {
            let id = spec.id.as_str().to_owned();
            spec.capabilities.into_iter().map(move |row| {
                let origins = row.linked_origins.iter().map(ToString::to_string).collect();
                let scope = match &row.capability {
                    Capability::Storage(storage) => Some(storage.scope),
                    _ => None,
                };
                (id.clone(), row.capability.kind(), scope, row.ask, origins)
            })
        })
        .filter(|row: &Row| !row.4.is_empty())
        .collect();
    let origins = vec![
        "*.up.1drv.com".to_owned(),
        "*.files.1drv.com".to_owned(),
        "*.sharepoint.com".to_owned(),
    ];
    let microsoft = |scope, ask| {
        (
            "microsoft".to_owned(),
            CapabilityKind::Storage,
            Some(scope),
            ask,
            origins.clone(),
        )
    };
    assert_eq!(
        rows,
        [
            microsoft(StorageScope::AppFolder, Ask::SignIn),
            microsoft(StorageScope::Full, Ask::OnUse),
        ]
    );
}

#[test]
fn a_whole_drive_is_asked_only_on_use() {
    use account_capability::Capability;
    use account_capability::{Ask, StorageScope};
    for (_, spec) in shipped() {
        let id = spec.id.as_str();
        for row in &spec.capabilities {
            if let Capability::Storage(storage) = &row.capability
                && storage.scope == StorageScope::Full
                && matches!(id, "google" | "microsoft")
            {
                assert_eq!(row.ask, Ask::OnUse, "{id}");
            }
        }
        // A grant of the app folder is served by the app-folder row asked at sign-in, a grant
        // of the whole store by the on-use row.
        if matches!(id, "google" | "microsoft") {
            use account_capability::CapabilityKind as K;
            assert_eq!(
                spec.ask_for(K::Storage, StorageScope::AppFolder),
                Some(Ask::SignIn),
                "{id}"
            );
            assert_eq!(
                spec.ask_for(K::Storage, StorageScope::Full),
                Some(Ask::OnUse),
                "{id}"
            );
        }
        // What sign-in asks leaves the on-use rows out.
        assert!(
            spec.sign_in_rows().all(|row| row.ask == Ask::SignIn),
            "{id}"
        );
    }
}

#[test]
fn ask_reads_on_use_and_is_sign_in_when_left_out() {
    use account_capability::CapabilityKind as K;
    use account_capability::{Ask, StorageScope};
    let spec = account_provider::parse_provider(
        r#"
id = "asks"
label = "Asks"
mark = "generic"
[auth]
kind = "password"
[discovery]
kind = "fixed"
[[capability]]
family = "imap"
kind = "mail"
v = { access = "read_write", send = "present", delta = "poll", transport = "imap", labels = "folders" }
[[capability]]
family = "webdav"
kind = "storage"
ask = "on_use"
v = { access = "read_write", delta = "poll", quota = "reported", scope = "full", hashes = "sha1", ranges = "present", chunked_upload = "present" }
"#,
    )
    .expect("spec");
    let asks: Vec<Ask> = spec.capabilities.iter().map(|row| row.ask).collect();
    assert_eq!(asks, [Ask::SignIn, Ask::OnUse]);
    assert_eq!(spec.sign_in_kinds(), [K::Mail]);
    let (folder, whole) = (StorageScope::AppFolder, StorageScope::Full);
    assert_eq!(spec.ask_for(K::Mail, folder), Some(Ask::SignIn));
    assert_eq!(spec.ask_for(K::Storage, folder), Some(Ask::OnUse));
    assert_eq!(spec.ask_for(K::Storage, whole), Some(Ask::OnUse));
    assert_eq!(spec.ask_for(K::Calendar, folder), None);
    assert_eq!(spec.on_use_rows(K::Storage).count(), 1);
    let bad = account_provider::parse_provider(
        r#"
id = "asks"
label = "Asks"
mark = "generic"
[auth]
kind = "password"
[discovery]
kind = "fixed"
[[capability]]
family = "imap"
kind = "mail"
ask = "sometimes"
v = { access = "read_write", send = "present", delta = "poll", transport = "imap", labels = "folders" }
"#,
    );
    bad.unwrap_err();
}

#[test]
fn the_generic_files_make_generic_rows_and_every_other_file_a_provider_row() {
    use account_sheet::RowKind;
    let rows: Vec<_> = shipped().iter().map(|(_, spec)| spec.sheet_row()).collect();
    let generic: Vec<_> = rows
        .iter()
        .filter(|r| r.kind == RowKind::Generic)
        .map(|r| r.id.as_str().to_owned())
        .collect();
    assert!(!generic.is_empty());
    assert!(generic.iter().all(|id| id.starts_with("generic-")));
    assert!(
        rows.iter()
            .filter(|r| r.id.as_str().starts_with("generic-"))
            .all(|r| r.kind == RowKind::Generic)
    );
    assert!(
        rows.iter()
            .filter(|r| !r.id.as_str().starts_with("generic-"))
            .all(|r| r.kind == RowKind::Provider)
    );
}

#[test]
fn the_agent_files_hold_no_secret_and_name_their_program_and_variables() {
    use account_capability::Capability;
    use account_capability::{AgentProtocol as P, CapabilityKind as K};
    use account_core::{AuthKind, Family};
    use account_provider::Discovery;
    // (id, program, key variable, base-url variable, protocols)
    type Row = (
        &'static str,
        &'static str,
        Option<&'static str>,
        Option<&'static str>,
        &'static [P],
    );
    const CASES: &[Row] = &[
        (
            "claude-code",
            "claude-code",
            Some("ANTHROPIC_API_KEY"),
            Some("ANTHROPIC_BASE_URL"),
            &[P::AnthropicMessages],
        ),
        (
            "gemini-cli",
            "gemini-cli",
            Some("GEMINI_API_KEY"),
            None,
            &[P::GenerateContent],
        ),
        (
            "codex",
            "codex",
            Some("OPENAI_API_KEY"),
            Some("OPENAI_BASE_URL"),
            &[P::OpenAiCompatible],
        ),
        ("acp-agent", "acp-agent", None, None, &[P::OpenAiCompatible]),
    ];
    for (id, program, key, base_url, protocols) in CASES {
        let spec = shipped_spec(id);
        assert_eq!(
            (spec.auth.kind, spec.auth.issuer),
            (AuthKind::AgentLogin, None),
            "{id}"
        );
        assert_eq!(spec.discovery, Discovery::Fixed, "{id}");
        assert!(spec.ai.is_none(), "{id}");
        assert_eq!(spec.capabilities.len(), 1, "{id}");
        let row = &spec.capabilities[0];
        assert_eq!(
            (row.capability.kind(), row.family),
            (K::Agent, Family::AcpAgent)
        );
        assert!(
            row.endpoint.is_none() && row.linked_origins.is_empty(),
            "{id}"
        );
        let Capability::Agent(agent) = &row.capability else {
            panic!("{id}: not an agent row");
        };
        assert_eq!(agent.program.as_str(), *program, "{id}");
        assert_eq!(
            agent
                .key_env
                .as_ref()
                .map(account_capability::EnvName::as_str),
            *key,
            "{id}"
        );
        assert_eq!(
            agent
                .base_url_env
                .as_ref()
                .map(account_capability::EnvName::as_str),
            *base_url,
            "{id}"
        );
        assert_eq!(agent.protocols, protocols.iter().copied().collect(), "{id}");
    }
}

#[test]
fn a_key_provider_names_the_agent_programs_that_may_use_its_key() {
    use account_capability::Capability;
    use account_capability::CapabilityKind as K;
    // Every key provider's named programs, by file id; a key file that names none is listed empty.
    let named = |id: &str| -> Vec<String> {
        shipped_spec(id)
            .capabilities
            .iter()
            .filter_map(|row| match &row.capability {
                Capability::Agent(agent) => Some(agent.program.to_string()),
                _ => None,
            })
            .collect()
    };
    assert_eq!(named("anthropic"), ["claude-code"]);
    assert_eq!(named("openai"), ["codex"]);
    assert_eq!(named("google-ai"), ["gemini-cli"]);
    assert!(named("openrouter").is_empty());
    assert!(named("moonshot").is_empty());
    // The model row is still the first row of a key provider.
    for id in ["anthropic", "openai", "google-ai"] {
        assert_eq!(shipped_spec(id).capabilities[0].capability.kind(), K::Llm);
    }
    // The program each key file names is the program of the agent file that shares its vendor,
    // with the same variables: one fact, written twice, must agree.
    for (key_file, agent_file) in [
        ("anthropic", "claude-code"),
        ("openai", "codex"),
        ("google-ai", "gemini-cli"),
    ] {
        let agent_cap = |id: &str| {
            shipped_spec(id)
                .capabilities
                .iter()
                .find_map(|row| match &row.capability {
                    Capability::Agent(agent) => Some(agent.clone()),
                    _ => None,
                })
                .expect("an agent row")
        };
        assert_eq!(agent_cap(key_file), agent_cap(agent_file), "{key_file}");
    }
}

#[test]
fn the_four_agent_files_make_agent_login_rows_named_by_their_labels() {
    use account_sheet::ProviderKind;
    let rows: Vec<_> = shipped().iter().map(|(_, spec)| spec.sheet_row()).collect();
    let mut agents: Vec<_> = rows
        .iter()
        .filter(|r| r.auth == ProviderKind::AgentLogin)
        .map(|r| (r.id.as_str().to_owned(), r.label.clone()))
        .collect();
    agents.sort();
    assert_eq!(
        agents,
        [
            ("acp-agent", "Other assistant"),
            ("claude-code", "Claude Code"),
            ("codex", "Codex"),
            ("gemini-cli", "Gemini assistant"),
        ]
        .map(|(id, label)| (id.to_owned(), label.to_owned()))
    );
    assert!(
        rows.iter()
            .filter(|r| r.auth == ProviderKind::Service)
            .all(|r| !["claude-code", "codex", "gemini-cli", "acp-agent"].contains(&r.id.as_str()))
    );
}

#[test]
fn no_shipped_provider_label_uses_a_developer_word() {
    const JARGON: [&str; 17] = [
        "acp", "mcp", "oauth", "pkce", "imap", "smtp", "pop", "pop3", "jmap", "dav", "caldav",
        "carddav", "cli", "token", "endpoint", "relay", "scope",
    ];
    for (path, spec) in &shipped() {
        let words: Vec<String> = spec
            .label
            .split(|c: char| !c.is_alphanumeric())
            .map(str::to_lowercase)
            .collect();
        for word in JARGON {
            assert!(
                !words.iter().any(|w| w == word),
                "{}: {:?} says {word}",
                path.display(),
                spec.label
            );
        }
        assert!(
            !words.iter().any(|w| w == "api" || w == "grant"),
            "{:?}",
            spec.label
        );
    }
    let labels: std::collections::BTreeMap<_, _> = shipped()
        .iter()
        .map(|(_, spec)| (spec.id.as_str().to_owned(), spec.label.clone()))
        .collect();
    for (id, label) in [
        ("acp-agent", "Other assistant"),
        ("generic-imap", "Other email account"),
        ("generic-dav", "Other calendar and contacts"),
        ("generic-jmap", "Other email account (newer servers)"),
        ("openai-compatible", "Other provider (OpenAI-compatible)"),
    ] {
        assert_eq!(labels[id], label, "{id}");
    }
}

#[test]
fn only_googles_picker_row_lists_an_authenticated_origin_and_it_is_one_exact_host() {
    let listed: Vec<(String, String, Vec<String>)> = shipped()
        .iter()
        .flat_map(|(_, spec)| {
            spec.capabilities
                .iter()
                .filter(|row| !row.auth_origins.is_empty())
                .map(|row| {
                    (
                        spec.id.as_str().to_owned(),
                        word_of(&row.family).to_owned(),
                        row.auth_origins.iter().map(ToString::to_string).collect(),
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect();
    assert_eq!(
        listed,
        [(
            "google".to_owned(),
            "google_photos_picker".to_owned(),
            vec!["lh3.googleusercontent.com".to_owned()]
        )]
    );
}
