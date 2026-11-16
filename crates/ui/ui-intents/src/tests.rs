use crate::{HeardEndMark, HeardMark, InputLevel};

#[test]
fn what_a_field_hears_never_shows_the_words_in_debug() {
    let words = "lisbon receipts";
    let marks = [
        HeardMark::Tail(words.into()),
        HeardMark::Committed(words.into()),
        HeardMark::Ended(HeardEndMark::Send(words.into())),
    ];
    for mark in marks {
        let shown = format!("{mark:?}");
        assert!(!shown.contains("lisbon"), "{shown}");
    }
    assert_eq!(
        format!("{:?}", HeardMark::Level(InputLevel(420))),
        "Level(420)"
    );
    assert_eq!(
        format!("{:?}", HeardMark::Ended(HeardEndMark::Nothing)),
        "Ended(Nothing)"
    );
}
