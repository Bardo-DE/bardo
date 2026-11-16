use super::{Control, control_of};
use ui_settings::schema::KeyKind;

fn words(list: &[&str]) -> Vec<String> {
    list.iter().map(|word| (*word).to_owned()).collect()
}

#[test]
fn each_kind_gets_the_control_detent_gives_it() {
    let cases = [
        (KeyKind::Toggle, Control::Switch),
        (
            KeyKind::Segmented {
                variants: words(&["a", "b", "c"]),
            },
            Control::PopUp(words(&["a", "b", "c"])),
        ),
        (
            KeyKind::Segmented {
                variants: words(&["a", "b"]),
            },
            Control::Segments(words(&["a", "b"])),
        ),
        (
            KeyKind::Menu {
                variants: words(&["a", "b"]),
            },
            Control::PopUp(words(&["a", "b"])),
        ),
        (KeyKind::Text, Control::Shown),
        (KeyKind::Colour, Control::Shown),
        (KeyKind::Shortcut, Control::Shown),
        (
            KeyKind::Provided {
                capability: "look".to_owned(),
            },
            Control::Shown,
        ),
    ];
    for (kind, expect) in cases {
        assert_eq!(control_of(&kind), expect, "{kind:?}");
    }
}
