//! The egress policy: host patterns and the verdicts by name, port and address, as table rows.
//! (The request parsing and the proxy itself are tested in the `sandbox` crate.)

use sandbox_core::egress::{
    EgressHostError, EgressPolicy, EgressRefusedError, Host, HostPattern, PolicyError, Ports,
};
use std::net::IpAddr;

fn pattern(text: &str) -> HostPattern {
    HostPattern::parse(text).expect("pattern")
}

fn ip(text: &str) -> IpAddr {
    text.parse().expect("ip")
}

fn host(text: &str) -> Host {
    Host::parse(text).expect("host")
}

#[test]
fn patterns_parse_or_name_the_fault() {
    let ok = |text: &str, want: HostPattern| (text.to_owned(), Ok(want));
    let bad = |text: &str, fault: EgressHostError| (text.to_owned(), Err(fault));
    let long = format!("{}.com", "a".repeat(64));
    let rows = [
        ok(
            "api.example.com",
            HostPattern::Exact("api.example.com".into()),
        ),
        ok(
            "API.Example.COM.",
            HostPattern::Exact("api.example.com".into()),
        ),
        ok(
            "*.example.com",
            HostPattern::Subdomains("example.com".into()),
        ),
        ok(
            "*.Example.com.",
            HostPattern::Subdomains("example.com".into()),
        ),
        ok("192.0.2.7", HostPattern::Addr(ip("192.0.2.7"))),
        ok("[2001:db8::1]", HostPattern::Addr(ip("2001:db8::1"))),
        ok("::ffff:192.0.2.7", HostPattern::Addr(ip("192.0.2.7"))),
        bad("", EgressHostError::Empty),
        bad(".", EgressHostError::Empty),
        bad("*", EgressHostError::BareStar),
        bad("*.", EgressHostError::BareStar),
        bad("a*.example.com", EgressHostError::StarNotLeading),
        bad("foo.*.com", EgressHostError::StarNotLeading),
        bad("*.*.com", EgressHostError::StarNotLeading),
        bad("example.*", EgressHostError::StarNotLeading),
        bad("exa mple.com", EgressHostError::Whitespace),
        bad(" example.com", EgressHostError::Whitespace),
        bad("example.com\n", EgressHostError::Whitespace),
        bad("b\u{fc}cher.example", EgressHostError::NonAscii),
        bad("*.b\u{fc}cher.example", EgressHostError::NonAscii),
        bad("a..b", EgressHostError::EmptyLabel),
        bad(".example.com", EgressHostError::EmptyLabel),
        bad("exa/mple.com", EgressHostError::BadChar),
        bad("host:80", EgressHostError::BadChar),
        bad("example.123", EgressHostError::Numeric),
        bad("127.1", EgressHostError::Numeric),
        bad(&long, EgressHostError::TooLong),
    ];
    for (text, want) in rows {
        assert_eq!(HostPattern::parse(&text), want, "{text:?}");
    }
}

fn policy(allow: &[&str], deny: &[&str], ports: Ports) -> EgressPolicy {
    let list = |l: &[&str]| l.iter().map(|t| pattern(t)).collect();
    EgressPolicy::new(list(allow), list(deny), ports).expect("policy")
}

#[test]
fn a_policy_needs_an_allow_list_and_real_ports() {
    assert_eq!(
        EgressPolicy::new(Vec::new(), vec![pattern("a.example")], Ports::default()),
        Err(PolicyError::NoAllow)
    );
    assert_eq!(Ports::new(Vec::new()), Err(PolicyError::BadPorts));
    assert_eq!(Ports::new(vec![443, 0]), Err(PolicyError::BadPorts));
    assert!(Ports::default().allows(443) && Ports::default().allows(80));
    assert!(!Ports::default().allows(22));
}

#[test]
fn verdicts_by_name_port_and_literal() {
    let p = policy(
        &[
            "api.example.com",
            "*.cdn.example.net",
            "*.evil.example.net",
            "203.0.113.9",
            "203.0.113.200",
        ],
        &["bad.cdn.example.net", "*.evil.example.net", "203.0.113.200"],
        Ports::default(),
    );
    let rows: Vec<(&str, u16, Result<(), EgressRefusedError>)> = vec![
        ("api.example.com", 443, Ok(())),
        ("api.example.com", 80, Ok(())),
        ("API.EXAMPLE.COM.", 443, Ok(())),
        ("api.example.com", 22, Err(EgressRefusedError::Port)),
        ("api.example.com", 8443, Err(EgressRefusedError::Port)),
        ("example.com", 443, Err(EgressRefusedError::NotListed)),
        (
            "www.api.example.com",
            443,
            Err(EgressRefusedError::NotListed),
        ),
        ("x.cdn.example.net", 443, Ok(())),
        ("a.b.cdn.example.net", 443, Ok(())),
        ("cdn.example.net", 443, Err(EgressRefusedError::NotListed)),
        ("xcdn.example.net", 443, Err(EgressRefusedError::NotListed)),
        ("bad.cdn.example.net", 443, Err(EgressRefusedError::Denied)),
        ("x.evil.example.net", 443, Err(EgressRefusedError::Denied)),
        ("203.0.113.9", 443, Ok(())),
        ("198.51.100.1", 443, Err(EgressRefusedError::NotListed)),
        ("[::1]", 443, Err(EgressRefusedError::NotListed)),
        ("127.0.0.1", 443, Err(EgressRefusedError::NotListed)),
        ("203.0.113.200", 443, Err(EgressRefusedError::Denied)),
        ("203.0.113.9", 22, Err(EgressRefusedError::Port)),
        ("::ffff:203.0.113.9", 443, Ok(())),
        ("localhost", 443, Err(EgressRefusedError::NotListed)),
        ("1.example.org", 443, Err(EgressRefusedError::NotListed)),
    ];
    for (name, port, want) in rows {
        assert_eq!(p.check(&host(name), port), want, "{name}:{port}");
    }
    let other = policy(&["a.example"], &[], Ports::new(vec![8443]).expect("ports"));
    assert_eq!(other.check(&host("a.example"), 8443), Ok(()));
    assert_eq!(
        other.check(&host("a.example"), 443),
        Err(EgressRefusedError::Port)
    );
}

#[test]
fn addresses_in_special_ranges_are_refused_unless_listed_exactly() {
    let p = policy(
        &["api.example.com", "10.0.0.5", "fd00::5", "203.0.113.200"],
        &["203.0.113.200"],
        Ports::default(),
    );
    let special = [
        "127.0.0.1",
        "127.9.9.9",
        "10.1.2.3",
        "172.16.0.1",
        "172.31.255.255",
        "192.168.1.1",
        "169.254.169.254",
        "0.0.0.0",
        "0.1.2.3",
        "224.0.0.1",
        "239.255.255.255",
        "255.255.255.255",
        "100.64.0.1",
        "100.127.255.255",
        "240.0.0.1",
        "::1",
        "::",
        "fe80::1",
        "febf::1",
        "fec0::1",
        "fc00::1",
        "fd12:3456::1",
        "ff02::1",
        "::ffff:10.0.0.1",
        "::ffff:127.0.0.1",
    ];
    for addr in special {
        assert_eq!(
            p.check_addr(ip(addr)),
            Err(EgressRefusedError::SpecialAddress),
            "{addr}"
        );
    }
    let fine = [
        "8.8.8.8",
        "1.1.1.1",
        "100.63.255.255",
        "100.128.0.1",
        "172.15.0.1",
        "172.32.0.1",
        "192.0.2.1",
        "2606:4700::1111",
        "::ffff:8.8.8.8",
        "10.0.0.5",
        "fd00::5",
        "::ffff:10.0.0.5",
    ];
    for addr in fine {
        assert_eq!(p.check_addr(ip(addr)), Ok(()), "{addr}");
    }
    assert_eq!(
        p.check_addr(ip("203.0.113.200")),
        Err(EgressRefusedError::Denied)
    );
}
