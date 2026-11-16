//! Chord text: spellings, refusals, taps, per-platform display and round trips.

use super::*;

fn chord(text: &str) -> Chord {
    text.parse().unwrap_or_else(|e| panic!("{text}: {e}"))
}

#[test]
fn spellings_of_one_chord_agree() {
    let redo = DefaultChord::from_str("Primary+Shift+Z");
    assert_eq!(redo, DefaultChord::from_str("primary+shift+z"));
    assert_eq!(chord("ctrl+shift+z"), chord("Control+Shift+Z"));
    assert_eq!(chord("⇧⌘Z"), chord("Cmd+Shift+Z"));
    assert_eq!(chord("⇧⌘Z"), chord("Super+Shift+z"));
    assert_eq!(chord("⌘+"), chord("Super++"));
    assert_eq!(chord("Ctrl+-"), chord("Ctrl+Minus"));
}

type Bad = (&'static str, ChordError);

#[test]
fn bad_text_is_refused_in_words() {
    let cases: &[Bad] = &[
        ("", ChordError::Empty),
        ("Ctrl+", ChordError::NoKey),
        ("Ctrl+Shift", ChordError::NoKey),
        ("Ctrl+Nope", ChordError::UnknownKey("Nope".into())),
        ("Zz+A", ChordError::NotAModifier("Zz".into())),
        ("Primary", ChordError::NoKey),
    ];
    for (text, expected) in cases {
        assert_eq!(
            text.parse::<DefaultChord>(),
            Err(expected.clone()),
            "{text:?}"
        );
    }
    assert_eq!(
        "Primary+C".parse::<Chord>(),
        Err(ChordError::PrimaryNotAllowed)
    );
}

#[test]
fn a_lone_modifier_is_a_tap() {
    let tap = Chord::new(Modifiers::empty(), Key::Named(NamedKey::Meta));
    assert_eq!(chord("Super"), tap);
    assert_eq!(chord("⌘"), tap);
    assert_eq!(tap.display(Platform::MacOs), "⌘");
    assert_eq!(tap.display(Platform::Windows), "Win");
}

type Shown = (Platform, &'static str, &'static str);

#[test]
fn display_follows_each_platforms_order_and_words() {
    let ours = Platform::Linux {
        desktop: Desktop::Ours,
    };
    let kde = Platform::Linux {
        desktop: Desktop::Kde,
    };
    let gnome = Platform::Linux {
        desktop: Desktop::Gnome,
    };
    let cases: &[Shown] = &[
        (Platform::MacOs, "Ctrl+Alt+Shift+Super+Z", "⌃⌥⇧⌘Z"),
        (Platform::MacOs, "Shift+Super+Z", "⇧⌘Z"),
        (ours, "Super+Space", "⌘Space"),
        (Platform::Windows, "Shift+Ctrl+Z", "Ctrl+Shift+Z"),
        (Platform::Windows, "Super+Shift+S", "Shift+Win+S"),
        (kde, "Shift+Ctrl+Z", "Ctrl+Shift+Z"),
        (kde, "Super+Alt+K", "Meta+Alt+K"),
        (gnome, "Super+Shift+Tab", "Shift+Super+Tab"),
        (gnome, "Ctrl+F5", "Ctrl+F5"),
    ];
    for (platform, input, shown) in cases {
        let c = chord(input);
        assert_eq!(c.display(*platform), *shown, "{platform:?} {input}");
    }
}

#[test]
fn display_reads_back_on_every_platform() {
    let samples = [
        "Ctrl+Alt+Shift+Super+Z",
        "Super+Space",
        "Ctrl+Up",
        "Alt+F4",
        "Shift+Tab",
        "Ctrl++",
        "Ctrl+-",
        "Ctrl+,",
        "Super+`",
        "Super",
        "Ctrl+Alt+Delete",
        "Shift+PgUp",
        "Alt+Enter",
        "Ctrl+Backspace",
        "PrtSc",
        "Shift+Super+3",
        "Ctrl+Esc",
        "Super+Home",
        "Ctrl+End",
        "AudioVolumeUp",
        "Ctrl+F24",
    ];
    for platform in Platform::ALL {
        for text in samples {
            let original = chord(text);
            let shown = original.display(platform);
            assert_eq!(
                shown.parse::<Chord>(),
                Ok(original),
                "{platform:?}: {shown}"
            );
        }
    }
}

#[test]
fn canonical_text_keeps_primary_and_round_trips() {
    for text in [
        "Primary+Shift+Z",
        "Ctrl+Alt+Shift+Super+A",
        "Primary++",
        "Super",
        "F11",
    ] {
        let parsed: DefaultChord = text.parse().unwrap_or_else(|e| panic!("{text}: {e}"));
        assert_eq!(
            parsed.canonical().parse::<DefaultChord>(),
            Ok(parsed),
            "{text}"
        );
    }
    assert_eq!(chord("shift+ctrl+z").canonical(), "Ctrl+Shift+Z");
}
