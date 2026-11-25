//! `dist/install-dev.sh` in a jail: HOME, every XDG directory and the prefix are directories of a
//! scratch tempdir, the binaries are fake files named by `INSTALL_DEV_BIN_DIR` (nothing builds,
//! nothing is compiled), and the environment is cleared, so the person's real `~/.config`,
//! `~/.local` and `/etc` are never named. The script reads the sibling checkouts' `dist` files, so the tests that
//! install skip (and say so) when the accounts service, the inference service, the memory service and the model layer are not checked out beside the actions service: a plain
//! clone builds with no sibling, and this is a developer tool.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const BINARIES: [&str; 12] = [
    "bardo-actiond",
    "bardo-companiond",
    "bardo-readerd",
    "bardo-actions-mcp",
    "bardo-do",
    "agent-eval",
    "bardo-memoryd",
    "bardo-inferd",
    "bardo-accountd",
    "bardo-agent",
    "bardo-agent-net-forward",
    "bardo-agent-net-enter",
];

/// The script's sources are the sibling checkouts; without them there is nothing to install from.
fn siblings_present() -> bool {
    let beside = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../actions/crates");
    let here = [
        "accounts/dist",
        "inference/dist",
        "memory/dbus",
        "models/catalog",
    ]
    .iter()
    .all(|dir| beside.join(dir).is_dir());
    if !here {
        eprintln!(
            "skipped: the accounts service, the inference service, the memory service and the model layer are not checked out beside actions"
        );
    }
    here
}

struct Jail {
    dir: tempfile::TempDir,
}

impl Jail {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("scratch");
        let fakes = dir.path().join("fakes");
        std::fs::create_dir_all(&fakes).expect("fakes");
        // bardo-agent-acp is installed by scripts/install.sh only, not by install-dev.sh.
        for name in BINARIES.iter().copied().chain(["bardo-agent-acp"]) {
            std::fs::write(fakes.join(name), format!("#!/bin/sh\necho fake {name}\n"))
                .expect("fake");
        }
        std::fs::create_dir_all(dir.path().join("home")).expect("home");
        Self { dir }
    }

    fn home(&self) -> PathBuf {
        self.dir.path().join("home")
    }

    fn config(&self) -> PathBuf {
        self.home().join(".config")
    }

    fn prefix(&self) -> PathBuf {
        self.home().join(".local")
    }

    fn script() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../dist/install-dev.sh")
    }

    fn run(&self, args: &[&str]) -> Output {
        let home = self.home();
        Command::new("bash")
            .arg(Self::script())
            .args(args)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", &home)
            .env("XDG_CONFIG_HOME", home.join(".config"))
            .env("XDG_DATA_HOME", home.join(".local/share"))
            .env("XDG_STATE_HOME", home.join(".local/state"))
            .env("PREFIX", home.join(".local"))
            // No question put to the real user manager: the jail says yes.
            .env("INSTALL_DEV_SANDBOX", "on")
            // No question put to the real user manager: the jail says yes (or no, in the test that needs it).
            .env("INSTALL_DEV_SANDBOX", "on")
            .env("INSTALL_DEV_BIN_DIR", self.dir.path().join("fakes"))
            .output()
            .expect("bash runs")
    }

    /// `scripts/install.sh` into a package root inside HOME: prefix `/opt/bardo-test`, fake programs.
    fn run_system(&self, args: &[&str]) -> Output {
        let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../scripts/install.sh");
        Command::new("bash")
            .arg(script)
            .args(["--prefix", "/opt/bardo-test", "--destdir"])
            .arg(self.home().join("root"))
            .arg("--bin-dir")
            .arg(self.dir.path().join("fakes"))
            .args(args)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", self.home())
            .output()
            .expect("bash runs")
    }

    fn ok(&self, args: &[&str]) -> String {
        let out = self.run(args);
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        assert!(
            out.status.success(),
            "{args:?}: {:?}\nstdout: {text}\nstderr: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        );
        text
    }

    /// Every file and directory under HOME, relative to it.
    fn tree(&self) -> BTreeSet<String> {
        fn walk(root: &Path, dir: &Path, into: &mut BTreeSet<String>) {
            for entry in std::fs::read_dir(dir).expect("dir") {
                let path = entry.expect("entry").path();
                let rel = path
                    .strip_prefix(root)
                    .expect("under")
                    .display()
                    .to_string();
                if path.is_dir() {
                    into.insert(format!("{rel}/"));
                    walk(root, &path, into);
                } else {
                    into.insert(rel);
                }
            }
        }
        let mut found = BTreeSet::new();
        walk(&self.home(), &self.home(), &mut found);
        found
    }

    fn read(&self, rel: &str) -> String {
        std::fs::read_to_string(self.home().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
    }
}

#[test]
fn an_install_puts_every_piece_where_the_session_reads_it() {
    if !siblings_present() {
        return;
    }
    let jail = Jail::new();
    let out = jail.ok(&[]);
    let tree = jail.tree();
    let has = |rel: &str| assert!(tree.contains(rel), "missing {rel}\n{tree:#?}");
    for name in BINARIES {
        has(&format!(".local/bin/{name}"));
    }
    for unit in [
        "bardo-actiond",
        "bardo-companiond",
        "bardo-readerd",
        "bardo-actions-mcp",
        "bardo-inferd",
        "bardo-accountd",
        "bardo-memoryd",
    ] {
        has(&format!(".config/systemd/user/{unit}.service"));
    }
    for name in [
        "Companion1",
        "Intents1",
        "Reader1",
        "Accounts1",
        "Inference1",
        "Memory1",
    ] {
        has(&format!(
            ".local/share/dbus-1/services/org.bardo.{name}.service"
        ));
    }
    has(".local/share/bardo/skills/desktop-basics/SKILL.md");
    has(".local/share/bardo/actions/permissions/default/grid.cedar");
    has(".local/share/bardo/settings/actions.settings.toml");
    has(".local/share/bardo/settings/memory.settings.toml");
    has(".local/share/bardo/settings/inference.settings.toml");
    has(".local/share/bardo/accounts/providers/openrouter.toml");
    has(".local/share/bardo/models/catalog/claude-haiku-4.5.toml");
    for config in ["actiond", "companiond", "actions-mcp"] {
        has(&format!(".config/bardo/actions/{config}.toml"));
    }
    has(".config/bardo/memoryd/callers.toml");
    has(".config/bardo/inference/inferd.toml");
    // Offline unless asked: no drop-in that opens inferd's network (its sandbox drop-in, which
    // only narrows it, is checked below).
    assert!(
        !tree.iter().any(|p| p.contains("bardo-inferd.service.d/")
            && !p.ends_with("bardo-inferd.service.d/")
            && !p.ends_with("bardo-inferd.service.d/sandbox.conf")),
        "{tree:#?}"
    );
    // The mount-namespace protections ride along as drop-ins.
    for unit in [
        "bardo-actiond",
        "bardo-companiond",
        "bardo-readerd",
        "bardo-actions-mcp",
        "bardo-memoryd",
        "bardo-inferd",
    ] {
        has(&format!(
            ".config/systemd/user/{unit}.service.d/sandbox.conf"
        ));
    }
    has(".local/state/bardo/dev/install-dev.manifest");
    // Nothing for bardo-voiced (a skeleton), bardo-syncd or bardo-computer-use, and nothing outside HOME.
    assert!(
        !tree
            .iter()
            .any(|p| p.contains("voiced") || p.contains("syncd")),
        "{tree:#?}"
    );
    assert!(
        out.contains("sudo install -D -m644 ")
            && out.contains("/dist/callers.toml /etc/bardo/accountd-callers.toml"),
        "{out}"
    );
    assert!(
        out.contains("/dist/inferd-callers.toml /etc/bardo/inferd-callers.toml"),
        "{out}"
    );
    // The agent daemons' callers files are the system's: printed for root, never put in the
    // person's configuration, where they could not give the shell its role.
    assert!(
        out.contains("/dist/actiond-callers.toml /etc/bardo/actiond-callers.toml"),
        "{out}"
    );
    assert!(
        !tree.iter().any(|p| p.contains("-callers.toml")),
        "{tree:#?}"
    );
    // The daemons read the table only at startup, so the printed steps restart them.
    assert!(
        out.contains("systemctl --user restart bardo-accountd bardo-inferd"),
        "{out}"
    );
}

#[test]
fn the_exec_lines_name_the_prefix_and_a_home_prefix_keeps_the_binary_reachable() {
    if !siblings_present() {
        return;
    }
    let jail = Jail::new();
    jail.ok(&[]);
    let bin = jail.prefix().join("bin");
    let bin = bin.display();
    for (unit, name) in [
        ("bardo-companiond", "bardo-companiond"),
        ("bardo-actiond", "bardo-actiond"),
        ("bardo-inferd", "bardo-inferd"),
        ("bardo-memoryd", "bardo-memoryd"),
    ] {
        let text = jail.read(&format!(".config/systemd/user/{unit}.service"));
        assert!(
            text.contains(&format!("\nExecStart={bin}/{name}")),
            "{unit}\n{text}"
        );
        let code: Vec<&str> = text.lines().filter(|l| !l.starts_with('#')).collect();
        assert!(
            !code.iter().any(|l| l.contains("/usr/libexec")),
            "{unit}\n{text}"
        );
    }
    let dbus = jail.read(".local/share/dbus-1/services/org.bardo.Companion1.service");
    assert!(
        dbus.contains(&format!("Exec={bin}/bardo-companiond\n")),
        "{dbus}"
    );
    // With the binary under HOME, `ProtectHome=yes` would hide it from its own unit.
    for unit in ["bardo-companiond", "bardo-readerd"] {
        let text = jail.read(&format!(
            ".config/systemd/user/{unit}.service.d/sandbox.conf"
        ));
        assert!(!text.contains("\nProtectHome=yes"), "{unit}");
        assert!(text.contains("\nProtectHome=read-only"), "{unit}");
    }
    let mode = std::os::unix::fs::PermissionsExt::mode(
        &std::fs::metadata(jail.prefix().join("bin/bardo-inferd"))
            .expect("meta")
            .permissions(),
    );
    assert_eq!(mode & 0o777, 0o755);
}

#[test]
fn a_config_of_the_persons_own_is_never_overwritten_and_a_second_run_changes_nothing() {
    if !siblings_present() {
        return;
    }
    let jail = Jail::new();
    let mine = jail.config().join("bardo/inference/inferd.toml");
    std::fs::create_dir_all(mine.parent().expect("dir")).expect("dir");
    std::fs::write(&mine, "# mine\n[ai]\nlocal_only = false\n").expect("mine");
    jail.ok(&[]);
    assert_eq!(
        jail.read(".config/bardo/inference/inferd.toml"),
        "# mine\n[ai]\nlocal_only = false\n"
    );
    let first = jail.tree();
    let again = jail.ok(&[]);
    assert_eq!(jail.tree(), first, "idempotent");
    assert!(again.contains("kept (yours)"), "{again}");
    assert_eq!(
        jail.read(".config/bardo/inference/inferd.toml"),
        "# mine\n[ai]\nlocal_only = false\n"
    );
}

#[test]
fn a_dry_run_prints_every_action_and_writes_nothing() {
    if !siblings_present() {
        return;
    }
    let jail = Jail::new();
    let out = jail.ok(&["--dry-run"]);
    assert!(jail.tree().is_empty(), "{:#?}", jail.tree());
    for expect in [
        "install -m 755",
        "bardo-actiond",
        "dbus-1/services/org.bardo.Companion1.service",
        "bardo/skills/desktop-basics/SKILL.md",
        "bardo/accounts/providers/openrouter.toml",
        "sudo install -D -m644",
    ] {
        assert!(out.contains(expect), "{expect}\n{out}");
    }
    jail.ok(&[]);
    let before = jail.tree();
    let removal = jail.ok(&["--uninstall", "--dry-run"]);
    assert_eq!(jail.tree(), before, "a dry uninstall removes nothing");
    assert!(removal.contains("remove "), "{removal}");
}

#[test]
fn an_uninstall_removes_exactly_what_was_installed_and_keeps_what_is_the_persons() {
    if !siblings_present() {
        return;
    }
    let jail = Jail::new();
    // Something of the person's in a directory the install also uses.
    let theirs = jail.prefix().join("bin/their-tool");
    std::fs::create_dir_all(theirs.parent().expect("dir")).expect("dir");
    std::fs::write(&theirs, "mine").expect("tool");
    let before = jail.tree();
    jail.ok(&[]);
    assert_ne!(jail.tree(), before);
    // The runbook edits inferd.toml: that edit survives.
    let inferd = jail.config().join("bardo/inference/inferd.toml");
    let edited = format!(
        "{}\n[ai]\nlocal_only = false\n",
        std::fs::read_to_string(&inferd).expect("read")
    );
    std::fs::write(&inferd, &edited).expect("edit");
    let out = jail.ok(&["--uninstall"]);
    assert!(out.contains("kept (you edited it)"), "{out}");
    let left = jail.tree();
    let expected: BTreeSet<String> = before
        .into_iter()
        .chain([
            ".config/".to_owned(),
            ".config/bardo/".to_owned(),
            ".config/bardo/inference/".to_owned(),
            ".config/bardo/inference/inferd.toml".to_owned(),
        ])
        .collect();
    assert_eq!(left, expected, "only the person's own files remain");
    assert_eq!(std::fs::read_to_string(&inferd).expect("read"), edited);
    assert_eq!(std::fs::read_to_string(&theirs).expect("read"), "mine");
    // Uninstalling again is a no-op.
    let again = jail.ok(&["--uninstall"]);
    assert!(again.contains("nothing to uninstall"), "{again}");
}

#[test]
fn a_missing_binary_stops_the_install_before_anything_is_written() {
    let jail = Jail::new();
    std::fs::remove_file(jail.dir.path().join("fakes/bardo-memoryd")).expect("remove");
    let out = jail.run(&[]);
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("missing: "),
        "{out:?}"
    );
    assert!(jail.tree().is_empty(), "{:#?}", jail.tree());
}

#[test]
fn the_script_never_calls_sudo_or_names_the_real_system() {
    let text = std::fs::read_to_string(Jail::script()).expect("script");
    for (n, line) in text.lines().enumerate() {
        let code = line.trim_start();
        // The one `sudo` is the line the script prints, indented inside its here-document.
        assert!(
            !line.starts_with("sudo ") && !code.starts_with("$ sudo"),
            "line {}: {line}",
            n + 1
        );
    }
    assert!(text.contains("set -euo pipefail"));
}

#[test]
fn cloud_is_opt_in_and_an_uninstall_removes_it() {
    if !siblings_present() {
        return;
    }
    let jail = Jail::new();
    jail.ok(&[]);
    let offline = jail.tree();
    // The packaged unit stays as shipped: this computer's own addresses only.
    let unit = jail.read(".config/systemd/user/bardo-inferd.service");
    assert!(unit.contains("IPAddressDeny=any"), "{unit}");
    assert!(unit.contains("IPAddressAllow=localhost"), "{unit}");
    let dry = jail.ok(&["--cloud", "--dry-run"]);
    assert!(dry.contains("bardo-inferd.service.d/cloud.conf"), "{dry}");
    assert_eq!(jail.tree(), offline, "a dry run writes nothing");
    jail.ok(&["--cloud"]);
    let dropin = jail.read(".config/systemd/user/bardo-inferd.service.d/cloud.conf");
    // The accounts service's drop-in opens every address (the families are already open in the unit).
    assert!(dropin.contains("IPAddressAllow=any"), "{dropin}");
    jail.ok(&["--uninstall"]);
    assert!(jail.tree().is_empty(), "{:#?}", jail.tree());
}

#[test]
fn the_system_installer_puts_each_piece_under_the_prefix_and_the_protections_are_a_choice() {
    let jail = Jail::new();
    let out = jail.run_system(&["--sandbox", "on"]);
    assert!(out.status.success(), "{out:?}");
    let tree = jail.tree();
    let has = |rel: &str| assert!(tree.contains(rel), "missing {rel}\n{tree:#?}");
    has("root/opt/bardo-test/libexec/bardo/bardo-actiond");
    has("root/opt/bardo-test/bin/bardo-do");
    has("root/opt/bardo-test/lib/systemd/user/bardo-actiond.service");
    has("root/opt/bardo-test/lib/systemd/user/bardo-actiond.service.d/sandbox.conf");
    has("root/opt/bardo-test/share/dbus-1/services/org.bardo.Intents1.service");
    has("root/opt/bardo-test/share/dbus-1/interfaces/org.bardo.Intents1.xml");
    has("root/opt/bardo-test/share/bardo/settings/actions.settings.toml");
    has("root/etc/xdg/bardo/actions/actiond.toml");
    has("root/etc/bardo/actiond-callers.toml");
    let unit = jail.read("root/opt/bardo-test/lib/systemd/user/bardo-actiond.service");
    assert!(
        unit.contains("\nExecStart=/opt/bardo-test/libexec/bardo/bardo-actiond\n"),
        "{unit}"
    );
    let activation =
        jail.read("root/opt/bardo-test/share/dbus-1/services/org.bardo.Intents1.service");
    assert!(
        activation.contains("Exec=/opt/bardo-test/libexec/bardo/bardo-actiond\n"),
        "{activation}"
    );
    // Off: the same install again, with no drop-ins, and the ones the first left are removed.
    let bare = &jail;
    let out = bare.run_system(&["--sandbox", "off"]);
    assert!(out.status.success(), "{out:?}");
    assert!(
        bare.tree().iter().all(|p| !p.contains("sandbox.conf")),
        "{:#?}",
        bare.tree()
    );
    assert!(
        bare.tree()
            .contains("root/opt/bardo-test/lib/systemd/user/bardo-actiond.service")
    );
}

#[test]
fn the_system_installer_replaces_an_earlier_default_configuration() {
    let jail = Jail::new();
    let conf = jail.home().join("root/etc/xdg/bardo/actions/actiond.toml");
    std::fs::create_dir_all(conf.parent().expect("dir")).expect("dir");
    std::fs::write(&conf, "[roles]\nstale = [\"org.example.Gone1\"]\n").expect("stale");
    let out = jail.run_system(&["--sandbox", "off"]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(
        std::fs::read_to_string(&conf).expect("read"),
        include_str!("../../../dist/actiond.toml")
    );
}

#[test]
fn the_system_installer_dry_run_prints_every_action_and_writes_nothing() {
    let jail = Jail::new();
    let out = jail.run_system(&["--dry-run", "--sandbox", "on"]);
    assert!(out.status.success(), "{out:?}");
    let text = String::from_utf8_lossy(&out.stdout);
    for expect in [
        "install -m 755",
        "bardo-actiond.service.d/sandbox.conf",
        "/etc/bardo/actiond-callers.toml",
        "dry run",
    ] {
        assert!(text.contains(expect), "{expect}\n{text}");
    }
    assert!(jail.tree().is_empty(), "{:#?}", jail.tree());
}
