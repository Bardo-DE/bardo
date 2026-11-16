//! Builders shared by the modules.

use caller_identity_core::{
    CgroupPath, DropIns, Exe, ExePath, Facts, FileTrust, Fragment, Loader, Pid, Pin, SandboxFacts,
    Table, Transient, Uid, UnitFacts, UnitName, Writer,
};

pub const PID: u32 = 4242;

/// A small table: one service with a unit, one program without.
pub const TABLE: &str = r#"
[[peer]]
app  = "org.bardo.Shell"
unit = "bardo-shell.service"
exe  = ["/opt/bardo/releases/*/bin/bardo-shell", "/usr/bin/bardo-shell"]
role = "shell"

[[peer]]
app  = "org.bardo.Updater"
exe  = ["/usr/bin/bardo-updater"]
role = "updater_cli"
"#;

pub fn table() -> Table {
    Table::from_toml(TABLE).expect("the sample table reads")
}

pub fn pid() -> Pid {
    Pid::new(PID).expect("a pid")
}

pub fn exe_path(text: &str) -> ExePath {
    ExePath::new(text).expect("an image path")
}

pub fn read(path: &str, trust: FileTrust) -> Exe {
    Exe::Read {
        path: exe_path(path),
        trust,
    }
}

pub fn root_only(path: &str) -> Exe {
    read(path, FileTrust::RootOnly)
}

pub fn user_writable(path: &str) -> Exe {
    read(path, FileTrust::Writable(Writer::Owner(Uid::new(1000))))
}

pub fn in_sill_unit() -> &'static str {
    "/user.slice/user-1000.slice/user@1000.service/app.slice/bardo-shell.service"
}

pub fn facts(cgroup: &str, exe: Exe) -> Facts {
    Facts {
        pid: pid(),
        uid: Uid::new(1000),
        cgroup: CgroupPath::new(cgroup).expect("a cgroup path"),
        exe,
        sandbox: SandboxFacts::Native,
        unit: None,
        pin: Pin::AtConnect,
    }
}

/// What the manager says of a healthy bardo-shell.service.
pub fn sill_unit() -> UnitFacts {
    UnitFacts {
        name: UnitName::new("bardo-shell.service").expect("a unit"),
        main_pid: Some(pid()),
        fragment: Fragment::RootOwned,
        drop_ins: DropIns::None,
        transient: Transient::No,
        exec: Some(exe_path("/usr/bin/bardo-shell")),
        exec_trust: Some(FileTrust::RootOnly),
        loader: Loader::Cleared,
    }
}
