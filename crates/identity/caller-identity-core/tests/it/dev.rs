//! Developer rows: a stand-in the person built is a service at `Proof::Dev` and nothing more, and
//! a table without developer rows judges as it always did.

use crate::common::{facts, in_shell_unit, root_only, table, user_writable};
use caller_identity_core::{
    AppKey, Engine, Exe, NotHeldError, Pin, Proof, Requirement, RoleName, Row, SandboxFacts,
    Standing, Table, UnitName, WhyNot, judge,
};

const STANDIN: &str = "/home/p/proj/target/debug/accountd_standin";
const SCOPE: &str = "/user.slice/user-1000.slice/user@1000.service/app.slice/run-u7.scope";

fn dev_row(extra: &str) -> String {
    format!(
        "[[peer]]\napp = \"org.bardo.Accounts\"\nexe = [\"{STANDIN}\"]\nrole = \"accounts\"\n{extra}"
    )
}

fn writable_standing(path: &str, cgroup: &str, table: &Table) -> Standing {
    judge(&facts(cgroup, user_writable(path)), table)
}

fn role(name: &str) -> RoleName {
    RoleName::new(name).expect("a role")
}

fn dev_service(app: &str, role_name: &str) -> Standing {
    Standing::Service {
        app: AppKey::new(app).expect("an app"),
        role: role(role_name),
        proof: Proof::Dev,
        pin: Pin::AtConnect,
    }
}

fn merged(dev: &str) -> Table {
    table().with_dev_rows(Table::from_toml(dev).expect("dev rows read"))
}

#[test]
fn a_stand_in_matching_a_dev_row_is_a_dev_service_even_though_the_person_wrote_it() {
    let table = merged(&dev_row(""));
    let want = dev_service("org.bardo.Accounts", "accounts");
    assert_eq!(writable_standing(STANDIN, SCOPE, &table), want);
    // A root-owned image of the same path is the same.
    assert_eq!(judge(&facts(SCOPE, root_only(STANDIN)), &table), want);
}

#[test]
fn a_dev_row_that_names_a_unit_needs_that_unit_in_the_cgroup() {
    let table = merged(&dev_row("unit = \"standin.service\"\n"));
    let in_unit = "/user.slice/user-1000.slice/user@1000.service/app.slice/standin.service";
    assert_eq!(
        writable_standing(STANDIN, in_unit, &table),
        dev_service("org.bardo.Accounts", "accounts")
    );
    for cgroup in [SCOPE, in_shell_unit()] {
        assert_eq!(
            writable_standing(STANDIN, cgroup, &table),
            Standing::Unverified {
                exe_name: Some("accountd_standin".to_owned()),
                scope_app: None,
                why: WhyNot::ImageWritable,
            },
            "{cgroup}"
        );
    }
}

#[test]
fn a_dev_row_that_does_not_match_changes_nothing() {
    let table = merged(&dev_row(""));
    let other = "/home/p/proj/target/debug/something_else";
    assert_eq!(
        writable_standing(other, SCOPE, &table),
        Standing::Unverified {
            exe_name: Some("something_else".to_owned()),
            scope_app: None,
            why: WhyNot::ImageWritable,
        }
    );
    // A root-owned image in no row stays a program.
    assert!(matches!(
        judge(&facts(SCOPE, root_only("/usr/bin/other")), &table),
        Standing::Program { .. }
    ));
    // A hidden image has no path to match, and a dev row's unit gives no cgroup proof.
    let named = merged(&dev_row("unit = \"standin.service\"\n"));
    let in_unit = "/user.slice/user-1000.slice/user@1000.service/app.slice/standin.service";
    assert!(matches!(
        judge(&facts(in_unit, Exe::Hidden), &named),
        Standing::Unverified {
            why: WhyNot::ImageHidden,
            ..
        }
    ));
    assert!(matches!(
        judge(&facts(SCOPE, Exe::Hidden), &table),
        Standing::Unverified {
            why: WhyNot::ImageHidden,
            ..
        }
    ));
}

#[test]
fn a_dev_row_never_outranks_a_shipped_row() {
    // The dev row claims the shipped shell's image and unit.
    let evil = "[[peer]]\napp = \"org.evil.Shell\"\nunit = \"bardo-shell.service\"\nexe = [\"/usr/bin/bardo-shell\"]\nrole = \"shell\"\n\
                [[peer]]\napp = \"org.evil.Other\"\nexe = [\"/usr/bin/bardo-shell\"]\nrole = \"other\"\n";
    let table = merged(evil);
    // The unit stays the shipped one's, and the other dev row is after the shipped rows.
    let shipped_app = table
        .row_for_unit(&UnitName::new("bardo-shell.service").expect("a unit"))
        .map(|r| r.app.as_str());
    assert_eq!(shipped_app, Some("org.bardo.Shell"));
    let dev: Vec<bool> = table.rows().iter().map(Row::is_dev).collect();
    assert_eq!(dev, [false, false, true]);
    // A process running the shipped image is judged by the shipped row, not as Dev.
    let standing = judge(
        &facts(in_shell_unit(), root_only("/usr/bin/bardo-shell")),
        &table,
    );
    assert_eq!(
        standing,
        Standing::Service {
            app: AppKey::new("org.bardo.Shell").expect("an app"),
            role: role("shell"),
            proof: Proof::Exe,
            pin: Pin::AtConnect,
        }
    );
    // Outside its unit the shipped image is a plain program: the unit-less dev row that names
    // the same image does not turn it into a service at any proof.
    assert!(matches!(
        judge(&facts(SCOPE, root_only("/usr/bin/bardo-shell")), &table),
        Standing::Program { .. }
    ));
}

#[test]
fn merging_the_other_way_round_still_puts_shipped_rows_first_and_keeps_their_units() {
    let dev = Table::empty().with_dev_rows(
        Table::from_toml(
            "[[peer]]\napp = \"org.evil.Shell\"\nunit = \"bardo-shell.service\"\nexe = [\"/home/p/shell\"]\nrole = \"shell\"\n",
        )
        .expect("dev"),
    );
    let merged = dev.merge(table());
    let apps: Vec<&str> = merged.rows().iter().map(|r| r.app.as_str()).collect();
    assert_eq!(apps, ["org.bardo.Shell", "org.bardo.Updater"]);
    assert!(merged.rows().iter().all(|r| !r.is_dev()));
}

#[test]
fn a_dev_row_may_name_a_shipped_app_without_replacing_its_row() {
    let table = merged(&dev_row("")).with_dev_rows(
        Table::from_toml(
            "[[peer]]\napp = \"org.bardo.Shell\"\nexe = [\"/home/p/shell_standin\"]\nrole = \"shell\"\n",
        )
        .expect("dev"),
    );
    assert_eq!(table.rows().len(), 4);
    assert_eq!(
        writable_standing("/home/p/shell_standin", SCOPE, &table),
        dev_service("org.bardo.Shell", "shell")
    );
    // The shipped row is untouched.
    assert!(!table.rows()[0].is_dev());
}

#[test]
fn rows_know_where_they_came_from() {
    let shipped = Table::from_toml(&dev_row("")).expect("shipped");
    assert!(!shipped.rows()[0].is_dev());
    let dev = Table::empty().with_dev_rows(shipped);
    assert!(dev.rows()[0].is_dev());
}

#[test]
fn dev_proof_is_below_every_real_proof_and_meets_none_of_their_requirements() {
    assert!(Proof::Dev < Proof::Cgroup && Proof::Cgroup < Proof::Exe);
    let standing = dev_service("org.bardo.Accounts", "accounts");
    for at_least in [Proof::Cgroup, Proof::Exe, Proof::Full] {
        let need = [Requirement::new(role("accounts"), at_least)];
        assert_eq!(
            standing.holds(&need),
            Err(NotHeldError::NotAllowed),
            "{at_least:?}"
        );
    }
    // Asked for by name, in a debug build, it is let in; a different role still is not.
    standing
        .holds(&[Requirement::dev(role("accounts"))])
        .unwrap();
    standing
        .holds(&[Requirement::new(role("accounts"), Proof::Dev)])
        .unwrap();
    standing
        .holds(&[Requirement::dev(role("shell"))])
        .unwrap_err();
    // It lets in `Proof::Dev` and no other: the real service meets the real requirement beside
    // it and not this one.
    let real = Standing::Service {
        app: AppKey::new("org.bardo.Accounts").expect("an app"),
        role: role("accounts"),
        proof: Proof::Full,
        pin: Pin::AtConnect,
    };
    assert_eq!(
        real.holds(&[Requirement::dev(role("accounts"))]),
        Err(NotHeldError::NotAllowed)
    );
    let debug_build = [
        Requirement::new(role("accounts"), Proof::Full),
        Requirement::dev(role("accounts")),
    ];
    real.holds(&debug_build).unwrap();
    standing.holds(&debug_build).unwrap();
    // A proof between them is let in by neither.
    let exe = Standing::Service {
        app: AppKey::new("org.bardo.Accounts").expect("an app"),
        role: role("accounts"),
        proof: Proof::Exe,
        pin: Pin::AtConnect,
    };
    assert_eq!(exe.holds(&debug_build), Err(NotHeldError::NotAllowed));
}

#[test]
fn a_sandboxed_process_is_never_a_dev_service() {
    let table = merged(&dev_row(""));
    let mut f = facts(SCOPE, user_writable(STANDIN));
    f.sandbox = SandboxFacts::Snap {
        label: "snap.firefox.firefox".into(),
    };
    assert_eq!(
        judge(&f, &table),
        Standing::Sandboxed {
            app: AppKey::new("firefox").expect("an app"),
            engine: Engine::Snap
        }
    );
    // A sandbox that disagrees with its cgroup is unverified, not a dev service.
    f.sandbox = SandboxFacts::Flatpak {
        app: "org.example.App".into(),
    };
    assert_eq!(
        judge(&f, &table),
        Standing::Unverified {
            exe_name: None,
            scope_app: None,
            why: WhyNot::SandboxDisagrees,
        }
    );
}
