use input_broker_wire::Maker;

/// casement keeps its own copy of the parser and pins these same strings (its `origin.rs` test).
/// Change one side and the other must change with it.
#[test]
fn the_labels_are_these_exact_strings_and_parse_back() {
    const WANT: &[(Maker, &str)] = &[
        (Maker::PadDaemon, "turnstile/pad_daemon"),
        (Maker::KeymapDaemon, "turnstile/keymap_daemon"),
    ];
    for &(maker, label) in WANT {
        assert_eq!(maker.to_string(), label);
        assert_eq!(Maker::parse(label), Some(maker));
        assert_eq!(
            Maker::parse(&format!("{label}\n")),
            Some(maker),
            "sysfs adds a newline"
        );
    }
}

#[test]
fn any_other_text_is_no_maker() {
    for text in [
        "",
        "turnstile/",
        "turnstile/pad_daemon/extra",
        "turnstile/pad_daemon ",
        "turnstile/Pad_daemon",
        "turnstile/settings",
        "turnstile",
        "xturnstile/pad_daemon",
        "ydotool virtual device",
        "usb-0000:00:14.0-2/input0",
    ] {
        assert_eq!(Maker::parse(text), None, "{text:?}");
    }
}
