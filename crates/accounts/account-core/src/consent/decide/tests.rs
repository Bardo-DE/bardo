use super::*;
use base_time::UnixSeconds;

fn granted(id: &str, scope: GrantScope) -> Verdict {
    Verdict::Granted {
        grant: GrantId::parse(id).expect("grant id"),
        scope,
    }
}

#[test]
fn decide_is_generic_over_the_key() {
    let grants = vec![Grant {
        id: GrantId::parse("g1").expect("grant id"),
        key: "mail.thread.archive",
        decision: Decision::Allow,
        scope: GrantScope::Always,
        at: UnixSeconds(1),
    }];
    assert_eq!(
        decide(&grants, &"mail.thread.archive"),
        granted("g1", GrantScope::Always)
    );
    assert_eq!(decide(&grants, &"mail.thread.delete"), Verdict::Ask);
}

#[test]
fn availability_reveals_only_the_best_state() {
    let yes = granted("g1", GrantScope::Always);
    let cases: Vec<(&str, Vec<Verdict>, bool, Availability)> = vec![
        (
            "a granted account",
            vec![Verdict::Ask, yes.clone()],
            true,
            Availability::Granted,
        ),
        (
            "an undecided account",
            vec![Verdict::Denied, Verdict::Ask],
            true,
            Availability::AvailableNeedsConsent,
        ),
        (
            "only refused accounts",
            vec![Verdict::Denied],
            true,
            Availability::Denied,
        ),
        (
            "no account, a provider could",
            vec![],
            true,
            Availability::NeedsAccount,
        ),
        (
            "no account, no provider",
            vec![],
            false,
            Availability::Unsupported,
        ),
    ];
    for (name, fitting, catalog, expected) in cases {
        assert_eq!(availability(&fitting, catalog), expected, "{name}");
    }
}
