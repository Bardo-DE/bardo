//! `agents.toml`: what is accepted, and what is refused outright.

use agent_launch::{AgentsFile, ConfigFault, Route};
use sandbox::NetworkMode;
use sandbox::egress::{Host, HostFault, Refusal};

use super::support::{HANDOFF, LOGIN};

#[test]
fn the_network_defaults_to_none_and_host_is_only_what_the_person_wrote() {
    let text = r#"
[[agent]]
program = "plain"
command = "/usr/bin/true"
route = "login"
"#;
    let file = AgentsFile::parse(text).expect("file");
    let entry = file.by_program(&"plain".parse_program()).expect("entry");
    assert_eq!(entry.network, NetworkMode::None);
    assert_eq!(AgentsFile::parse(LOGIN).expect("login").programs().len(), 1);
    assert!(AgentsFile::parse(HANDOFF).is_ok());
}

trait Program {
    fn parse_program(&self) -> account_capability::AgentProgram;
}

impl Program for str {
    fn parse_program(&self) -> account_capability::AgentProgram {
        account_capability::AgentProgram::parse(self).expect("program")
    }
}

fn refused(text: &str) -> ConfigFault {
    AgentsFile::parse(text).expect_err(text)
}

fn entry(extra: &str) -> String {
    format!("[[agent]]\nprogram = \"x\"\ncommand = \"/usr/bin/x\"\n{extra}\n")
}

/// What kind of refusal a fault is, as the table below names it.
fn kind_of(fault: &ConfigFault) -> &'static str {
    match fault {
        ConfigFault::Entry { .. } => "entry",
        ConfigFault::Syntax(why) if why.contains("delivery") => "delivery",
        ConfigFault::Syntax(_) => "syntax",
        ConfigFault::Twice(_) => "twice",
        _ => "other",
    }
}

fn plain(extra: &str) -> String {
    format!(
        "[[agent]]\nprogram = \"plain\"\ncommand = \"/usr/bin/true\"\nroute = \"login\"\n{extra}\n"
    )
}

/// Every way a file is refused whole, named by what is wrong with it.
#[test]
fn a_file_that_does_not_make_sense_is_refused_whole() {
    let long = "x".repeat(65);
    let login = |more: &str| entry(&format!("route = \"login\"\n{more}"));
    let set = |name: &str| format!("{}[agent.set]\n{name} = \"1\"\n", login(""));
    let key_set = format!(
        "{}[agent.set]\nK = \"1\"\n",
        entry("route = \"handoff\"\nnetwork = \"host\"\nkey_env = \"K\"")
    );
    let table: Vec<(&str, String, &str)> = vec![
        // The endpoint route cannot reach its endpoint with no network.
        (
            "endpoint without network",
            entry("route = \"endpoint\"\nnetwork = \"none\""),
            "entry",
        ),
        // An endpoint route needs its pair of variables and its target.
        (
            "endpoint without variables",
            entry("route = \"endpoint\"\nnetwork = \"endpoint_only\"\nkey_env = \"K\""),
            "entry",
        ),
        (
            "endpoint without target",
            entry(
                "route = \"endpoint\"\nnetwork = \"endpoint_only\"\nkey_env = \"K\"\nbase_url_env = \"B\"",
            ),
            "entry",
        ),
        // A handed-off key is for a program that reaches its provider, and says so.
        (
            "handoff without network",
            entry("route = \"handoff\"\nkey_env = \"K\""),
            "entry",
        ),
        // endpoint_only belongs to the endpoint route.
        (
            "endpoint_only elsewhere",
            login("network = \"endpoint_only\""),
            "entry",
        ),
        (
            "relative state",
            login("state = [\"relative/dir\"]"),
            "entry",
        ),
        ("root read", login("reads = [\"/\"]"), "entry"),
        (
            "relative command",
            "[[agent]]\nprogram = \"x\"\ncommand = \"bin/x\"\nroute = \"login\"\n".to_owned(),
            "entry",
        ),
        ("LD_PRELOAD", set("LD_PRELOAD"), "entry"),
        ("PATH", set("PATH"), "entry"),
        ("HOME", set("HOME"), "entry"),
        ("BASH_ENV", set("BASH_ENV"), "entry"),
        ("BARDO_ACTIONS_X", set("BARDO_ACTIONS_X"), "entry"),
        ("a key set in the clear", key_set, "entry"),
        ("unknown field", login("surprise = 1"), "syntax"),
        ("not toml", "not toml [".to_owned(), "syntax"),
        (
            "program name",
            "[[agent]]\nprogram = \"Not Lower\"\ncommand = \"/usr/bin/x\"\nroute = \"login\"\n"
                .to_owned(),
            "entry",
        ),
        (
            "listed twice",
            format!("{}{}", login(""), login("")),
            "twice",
        ),
        (
            "delivery line",
            entry("route = \"handoff\"\nkey_env = \"K\"\ndelivery = \"file\""),
            "delivery",
        ),
        (
            "model id with a space",
            login("model = \"two words\""),
            "entry",
        ),
    ];
    for (name, text, expected) in &table {
        let fault = refused(text);
        assert_eq!(kind_of(&fault), *expected, "{name}: {fault:?}");
    }
    assert!(AgentsFile::parse(&login("model = \"flash\"")).is_ok());
    // A label or a sign-in method that is empty, long or odd is refused naming the program.
    let labels = [
        "\"   \"".to_owned(),
        format!("\"{long}\""),
        "\"two\\nlines\"".to_owned(),
        "\"tab\\there\"".to_owned(),
    ];
    let methods = [
        "\"\"".to_owned(),
        format!("\"{long}\""),
        "\"two words\"".to_owned(),
        "\"new\\nline\"".to_owned(),
        "\"a/b\"".to_owned(),
    ];
    let odd = labels
        .iter()
        .map(|v| plain(&format!("label = {v}")))
        .chain(methods.iter().map(|v| plain(&format!("sign_in = {v}"))));
    for text in odd {
        match AgentsFile::parse(&text) {
            Err(ConfigFault::Entry { program, .. }) => assert_eq!(program, "plain", "{text}"),
            other => panic!("{text}: {other:?}"),
        }
    }
    let longest = "x".repeat(64);
    assert!(AgentsFile::parse(&plain(&format!("label = \"{longest}\""))).is_ok());
}

#[test]
fn an_empty_file_lists_nothing() {
    assert!(AgentsFile::parse("").expect("empty").is_empty());
}

#[test]
fn the_shipped_example_reads_in_both_of_its_forms() {
    let text = include_str!("../../../dist/agents.example.toml");
    let file = AgentsFile::parse(text).expect("the example");
    assert_eq!(file.len(), 2);
    // The commented entries, uncommented in place of the first, read too: the allowlist form
    // and the endpoint form.
    let uncomment = |block: &str| -> String {
        std::iter::once("[[agent]]\n".to_owned())
            .chain(block.lines().map(|l| {
                format!(
                    "{}\n",
                    l.strip_prefix("# ").unwrap_or(l.trim_start_matches('#'))
                )
            }))
            .collect()
    };
    let (rest, endpoint) = text.rsplit_once("# [[agent]]").expect("endpoint form");
    let file = AgentsFile::parse(&uncomment(endpoint)).expect("the endpoint form");
    assert_eq!(file.len(), 1);
    let (_, allowlist) = rest.rsplit_once("# [[agent]]").expect("allowlist form");
    let allowlist = allowlist.split("\n\n").next().expect("block");
    AgentsFile::parse(&uncomment(allowlist)).expect("the allowlist form");
}

#[test]
fn the_desktops_actions_are_offered_unless_the_entry_says_off() {
    let text = |tools: &str| {
        format!(
            "[[agent]]\nprogram = \"plain\"\ncommand = \"/usr/bin/true\"\nroute = \"login\"\n{tools}"
        )
    };
    let tools_of = |text: String| {
        AgentsFile::parse(&text)
            .map(|f| f.by_program(&"plain".parse_program()).expect("entry").tools)
    };
    assert_eq!(tools_of(text("")), Ok(true));
    assert_eq!(tools_of(text("tools = false\n")), Ok(false));
    assert!(tools_of(text("tools = \"yes\"\n")).is_err());
}

fn labelled(label: &str) -> String {
    format!(
        "[[agent]]\nprogram = \"plain\"\ncommand = \"/usr/bin/true\"\nroute = \"login\"\nlabel = {label}\n"
    )
}

fn label_of(text: &str) -> Option<String> {
    let file = AgentsFile::parse(text).expect("file");
    let entry = file.by_program(&"plain".parse_program()).expect("entry");
    entry.label.clone()
}

#[test]
fn a_label_is_the_trimmed_text_the_person_wrote_and_is_optional() {
    assert_eq!(
        label_of(&labelled(r#""  Claude Code ""#)),
        Some("Claude Code".to_owned())
    );
    let bare = "[[agent]]\nprogram = \"plain\"\ncommand = \"/usr/bin/true\"\nroute = \"login\"\n";
    assert_eq!(label_of(bare), None);
}

fn signed_in(value: &str) -> String {
    format!(
        "[[agent]]\nprogram = \"plain\"\ncommand = \"/usr/bin/true\"\nroute = \"login\"\nsign_in = {value}\n"
    )
}

#[test]
fn a_sign_in_method_is_a_short_id_and_is_optional() {
    let file = AgentsFile::parse(&signed_in(r#""oauth-personal""#)).expect("file");
    let entry = file.by_program(&"plain".parse_program()).expect("entry");
    assert_eq!(
        entry.sign_in.as_ref().map(|s| s.as_str()),
        Some("oauth-personal")
    );
    let bare = "[[agent]]\nprogram = \"plain\"\ncommand = \"/usr/bin/true\"\nroute = \"login\"\n";
    let file = AgentsFile::parse(bare).expect("file");
    let entry = file.by_program(&"plain".parse_program()).expect("entry");
    assert!(entry.sign_in.is_none());
}

// ---- agents from the registry ----

fn installed_agy(root: &std::path::Path) -> agent_registry::dirs::AgentsDir {
    use agent_registry::record::{Check, FILE, FILES, Record};
    let dir = agent_registry::dirs::AgentsDir::at(root);
    let slug = |t: &str| agent_registry::slug::Slug::parse(t).expect("slug");
    let at = dir.installed(&slug("agy-acp"), &slug("1.3.0"));
    std::fs::create_dir_all(at.join(FILES)).expect("dir");
    let record = Record {
        id: "agy-acp".to_owned(),
        version: "1.3.0".to_owned(),
        command: "agy".to_owned(),
        args: vec!["--uid=".to_owned()],
        env: Default::default(),
        digest: None,
        check: Check::FirstUse,
    };
    std::fs::write(at.join(FILE), record.render().expect("render")).expect("record");
    dir
}

const REGISTRY_ENTRY: &str = r#"
[[agent]]
program = "antigravity"
registry = "agy-acp"
version = "1.3.0"
network = "host"
sign_in = "google"
model = "gemini-pro-agent"
label = "Antigravity"
"#;

#[test]
fn a_registry_agent_runs_from_its_install_with_a_home_of_its_own() {
    let tmp = tempfile::tempdir().expect("tmp");
    let dir = installed_agy(tmp.path());
    let file = AgentsFile::parse_in(REGISTRY_ENTRY, Some(&dir)).expect("file");
    let entry = file
        .get(&permission_core::ProgramName::parse("antigravity").expect("name"))
        .expect("entry");
    let root = tmp.path().to_str().expect("utf-8");
    assert_eq!(entry.route, Route::Login);
    assert_eq!(entry.args, ["--uid="]);
    assert_eq!(
        entry.command.as_str(),
        format!("{root}/installed/agy-acp/1.3.0/files/agy")
    );
    let home = format!("{root}/state/agy-acp/home");
    assert_eq!(entry.home.as_ref().map(|h| h.as_str()), Some(home.as_str()));
    assert_eq!(entry.state[0].as_str(), home);
    assert_eq!(entry.reads.len(), 1);
    assert_eq!(
        entry.model.as_ref().map(|m| m.as_str()),
        Some("gemini-pro-agent")
    );
    assert_eq!(entry.sign_in.as_ref().map(|m| m.as_str()), Some("google"));
}

#[test]
fn a_registry_agent_added_in_settings_is_one_docket_starts() {
    let tmp = tempfile::tempdir().expect("tmp");
    let dir = installed_agy(tmp.path());
    let agent = action_settings::NewAgent {
        program: "antigravity".to_owned(),
        registry: "agy-acp".to_owned(),
        version: "1.3.0".to_owned(),
        label: "Antigravity".to_owned(),
    };
    let text = action_settings::edit_agent_add("", &agent).expect("add");
    let file = AgentsFile::parse_in(&text, Some(&dir)).expect("file");
    let entry = file
        .get(&permission_core::ProgramName::parse("antigravity").expect("name"))
        .expect("entry");
    assert_eq!(entry.route, Route::Login);
    assert_eq!(entry.network, NetworkMode::Host);
}

#[test]
fn a_registry_agent_that_is_not_installed_or_not_pinned_is_refused() {
    let tmp = tempfile::tempdir().expect("tmp");
    let dir = agent_registry::dirs::AgentsDir::at(tmp.path());
    let fault = AgentsFile::parse_in(REGISTRY_ENTRY, Some(&dir)).expect_err("not installed");
    assert!(matches!(fault, ConfigFault::Registry { .. }));
    let unpinned = REGISTRY_ENTRY.replace("version = \"1.3.0\"\n", "");
    assert!(AgentsFile::parse_in(&unpinned, Some(&dir)).is_err());
    // Without the agents directory a registry entry cannot be placed at all.
    assert!(AgentsFile::parse(REGISTRY_ENTRY).is_err());
    let both = REGISTRY_ENTRY.replace("registry =", "command = \"/usr/bin/x\"\nregistry =");
    assert!(AgentsFile::parse_in(&both, Some(&dir)).is_err());
}

#[test]
fn checkpoints_say_who_keeps_the_history_and_the_claude_code_profile_defaults_to_the_agent() {
    use action_core::Rewind;
    let rows = [
        ("nothing written", "", Rewind::Docket),
        ("docket", "checkpoints = \"docket\"", Rewind::Docket),
        ("agent", "checkpoints = \"agent\"", Rewind::Agent),
        ("the preset", "profile = \"claude-code\"", Rewind::Agent),
        (
            "the preset, overruled",
            "profile = \"claude-code\"\ncheckpoints = \"docket\"",
            Rewind::Docket,
        ),
    ];
    for (name, extra, want) in rows {
        let text = format!(
            "[[agent]]\nprogram = \"plain\"\ncommand = \"/usr/bin/true\"\nroute = \"login\"\n{extra}\n"
        );
        let file = AgentsFile::parse(&text).expect(name);
        let entry = file.by_program(&"plain".parse_program()).expect("entry");
        assert_eq!(entry.rewind, want, "{name}");
    }
    let odd = "[[agent]]\nprogram = \"plain\"\ncommand = \"/usr/bin/true\"\nroute = \"login\"\ncheckpoints = \"git\"\n";
    assert!(matches!(refused(odd), ConfigFault::Syntax(_)));
}

#[test]
fn resume_names_the_session_and_the_claude_code_profile_has_the_well_known_flag() {
    let text = |extra: &str| {
        format!(
            "[[agent]]\nprogram = \"plain\"\ncommand = \"/usr/bin/true\"\nroute = \"login\"\n{extra}\n"
        )
    };
    let resume = |extra: &str| {
        let file = AgentsFile::parse(&text(extra)).expect("parses");
        file.by_program(&"plain".parse_program())
            .expect("entry")
            .resume
            .clone()
    };
    assert_eq!(resume(""), None);
    // The app's own id goes where `{session}` stands, and nowhere else.
    let file = AgentsFile::parse(&text("profile = \"claude-code\"")).expect("parses");
    let entry = file.by_program(&"plain".parse_program()).expect("entry");
    assert_eq!(
        entry.resume_line("0a1b-2c3d"),
        Some(vec!["--resume".to_owned(), "0a1b-2c3d".to_owned()])
    );
    assert_eq!(
        resume("profile = \"claude-code\""),
        Some(vec!["--resume".to_owned(), "{session}".to_owned()])
    );
    assert_eq!(
        resume("profile = \"claude-code\"\nresume = [\"-r={session}\"]"),
        Some(vec!["-r={session}".to_owned()])
    );
    // Arguments that never name the session are no way to pick one up.
    refused(&text("resume = [\"--continue\"]"));
}

#[test]
fn a_command_line_a_terminal_saw_is_the_app_and_conversation_it_picks_up() {
    use companion_model::SeenCommand;
    let file = AgentsFile::parse(concat!(
        "[[agent]]\nprogram = \"claude-code\"\ncommand = \"/usr/bin/claude\"\nroute = \"login\"\n",
        "profile = \"claude-code\"\n",
        "[[agent]]\nprogram = \"plain\"\ncommand = \"/usr/bin/plain\"\nroute = \"login\"\n",
        "[[agent]]\nprogram = \"flagged\"\ncommand = \"/opt/flagged\"\nroute = \"login\"\n",
        "resume = [\"--session={session}\", \"--go\"]\n",
    ))
    .expect("parses");
    // The line, and the program (or none) and id it must give.
    const CASES: &[(&str, Option<(&str, &str)>)] = &[
        (
            "/usr/bin/claude --resume ab-1",
            Some(("claude-code", "ab-1")),
        ),
        (
            "/usr/bin/env -i K=V /usr/bin/bwrap --die-with-parent -- claude --resume ab-1",
            Some(("claude-code", "ab-1")),
        ),
        ("/opt/flagged --session=x_9 --go", Some(("flagged", "x_9"))),
        ("/opt/flagged --session=x_9", None),
        ("/usr/bin/other --resume ab-1", None),
        ("/usr/bin/claude --continue", None),
        ("/usr/bin/claude --resume", None),
        ("/usr/bin/claude --resume -x", None),
        ("/usr/bin/plain --resume ab-1", None),
    ];
    for (line, want) in CASES {
        let seen = SeenCommand::parse(line.split(' ').map(str::to_owned).collect()).expect(line);
        let got = file
            .recognise(&seen)
            .map(|agent| (agent.program.as_str().to_owned(), agent.session.to_string()));
        let want = want.map(|(program, id)| (program.to_owned(), id.to_owned()));
        assert_eq!(got, want, "{line}");
    }
    // What the host builds to resume is what it recognises.
    for entry in ["claude-code", "flagged"] {
        let entry = file
            .get(&permission_core::ProgramName::parse(entry).expect("name"))
            .expect("entry");
        let mut words = vec![entry.command.as_str().to_owned()];
        words.extend(entry.resume_line("0a1b-2c3d").expect("line"));
        let seen = SeenCommand::parse(words).expect("seen");
        let agent = file.recognise(&seen).expect("recognised");
        assert_eq!(agent.program, entry.name);
        assert_eq!(agent.session.as_str(), "0a1b-2c3d");
    }
}

const ALLOWLIST_LOGIN: &str = "route = \"login\"\nnetwork = \"allowlist\"\n";

fn site(list: &'static str, text: &str, why: HostFault) -> ConfigFault {
    ConfigFault::Site {
        program: "x".to_owned(),
        list,
        entry: text.to_owned(),
        why,
    }
}

#[test]
fn an_allowlist_entry_reads_its_lists_and_ports() {
    let text = entry(&format!(
        "{ALLOWLIST_LOGIN}allow = [\"api.example.com\", \"*.Example.org\", \"192.0.2.7\"]\ndeny = [\"bad.example.org\"]\nports = [8443]"
    ));
    let file = AgentsFile::parse(&text).expect("file");
    let found = file.by_program(&"x".parse_program()).expect("entry");
    assert_eq!(found.network, NetworkMode::Allowlist);
    let reach = found.reach.as_ref().expect("reach");
    assert_eq!(
        reach.sites(),
        ["api.example.com", "*.example.org", "192.0.2.7"]
    );
    let host = |t: &str| Host::parse(t).expect("host");
    let policy = reach.policy();
    assert_eq!(policy.check(&host("api.example.com"), 443), Ok(()));
    assert_eq!(policy.check(&host("api.example.com"), 80), Ok(()));
    assert_eq!(policy.check(&host("api.example.com"), 8443), Ok(()));
    assert_eq!(policy.check(&host("a.example.org"), 443), Ok(()));
    assert_eq!(
        policy.check(&host("api.example.com"), 22),
        Err(Refusal::Port)
    );
    assert_eq!(
        policy.check(&host("bad.example.org"), 443),
        Err(Refusal::Denied)
    );
    assert_eq!(
        policy.check(&host("other.example.net"), 443),
        Err(Refusal::NotListed)
    );
    // Only an allowlist entry has a reach; the other modes have none.
    let plain = AgentsFile::parse(LOGIN).expect("login");
    assert_eq!(
        plain
            .by_program(&"claude-code".parse_program())
            .expect("entry")
            .reach,
        None
    );
}

#[test]
fn a_bad_site_names_the_program_and_the_entry() {
    let with = |lists: &str| entry(&format!("{ALLOWLIST_LOGIN}{lists}"));
    assert_eq!(
        refused(&with("allow = [\"bad host\"]")),
        site("allow", "bad host", HostFault::Whitespace)
    );
    assert_eq!(
        refused(&with("allow = [\"a.example.com\"]\ndeny = [\"*\"]")),
        site("deny", "*", HostFault::BareStar)
    );
    assert_eq!(
        refused(&with("allow = [\"a*.example.com\"]")),
        site("allow", "a*.example.com", HostFault::StarNotLeading)
    );
    assert_eq!(
        refused(&with("allow = [\"127.1\"]")),
        site("allow", "127.1", HostFault::Numeric)
    );
}

#[test]
fn an_allowlist_without_an_allow_entry_or_with_a_bad_port_or_in_the_wrong_place_refuses() {
    // Nothing allowed reaches nothing: a mistake, not a closed network.
    assert!(matches!(
        refused(&entry(ALLOWLIST_LOGIN)),
        ConfigFault::Entry { .. }
    ));
    assert!(matches!(
        refused(&entry(&format!(
            "{ALLOWLIST_LOGIN}allow = []\ndeny = [\"a.example.com\"]"
        ))),
        ConfigFault::Entry { .. }
    ));
    assert!(matches!(
        refused(&entry(&format!(
            "{ALLOWLIST_LOGIN}allow = [\"a.example.com\"]\nports = [0]"
        ))),
        ConfigFault::Entry { .. }
    ));
    assert!(matches!(
        refused(&entry(&format!(
            "{ALLOWLIST_LOGIN}allow = [\"a.example.com\"]\nports = [70000]"
        ))),
        ConfigFault::Syntax(_)
    ));
    // The lists belong to the allowlist network and no other.
    for network in ["none", "host"] {
        assert!(
            matches!(
                refused(&entry(&format!(
                    "route = \"login\"\nnetwork = \"{network}\"\nallow = [\"a.example.com\"]"
                ))),
                ConfigFault::Entry { .. }
            ),
            "{network}"
        );
    }
    // A model endpoint is on this computer's loopback, which a filter never lets through.
    assert!(matches!(
        refused(&entry(
            "route = \"endpoint\"\nnetwork = \"allowlist\"\nallow = [\"a.example.com\"]\nkey_env = \"K\"\nbase_url_env = \"B\"\n[agent.endpoint]\nkind = \"account\"\nid = \"a\""
        )),
        ConfigFault::Entry { .. }
    ));
    // An unknown key is still refused.
    assert!(matches!(
        refused(&entry(&format!(
            "{ALLOWLIST_LOGIN}allow = [\"a.example.com\"]\nallowed = [\"b.example.com\"]"
        ))),
        ConfigFault::Syntax(_)
    ));
}

#[test]
fn a_handed_off_key_may_reach_only_the_sites_it_names() {
    let text = entry(
        "route = \"handoff\"\nnetwork = \"allowlist\"\nkey_env = \"K\"\nallow = [\"api.example.com\"]",
    );
    assert!(AgentsFile::parse(&text).is_ok());
}
