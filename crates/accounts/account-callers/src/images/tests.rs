use super::*;

#[test]
fn a_daemon_is_installed_under_each_root_and_a_program_in_bin() {
    assert_eq!(
        installed(Install::Libexec, "bardo-inferd"),
        [
            "/usr/libexec/bardo/bardo-inferd",
            "/usr/local/libexec/bardo/bardo-inferd",
            "/opt/bardo/releases/*/libexec/bardo/bardo-inferd",
        ]
    );
    assert_eq!(
        installed(Install::Bin, "bardo-shell")[0],
        "/usr/bin/bardo-shell"
    );
}

#[test]
fn only_a_plain_service_name_is_a_daemon() {
    let cases = [
        ("bardo-inferd.service", Some("bardo-inferd")),
        ("bardo-agent-acp.service", Some("bardo-agent-acp")),
        ("org.bardo.Settings.service", None),
        ("bardo-shell.scope", None),
        (".service", None),
        ("Inferd.service", None),
        ("a/b.service", None),
    ];
    for (unit, want) in cases {
        assert_eq!(daemon_name(unit), want, "{unit}");
    }
}
