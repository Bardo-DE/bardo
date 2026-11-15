use super::super::*;

/// One row: kinds bits, multiple, cursor modes, and what they parse to.
type Row = (
    u32,
    u32,
    &'static [u32],
    Option<(Kinds, Plurality, Vec<Pointer>)>,
);

#[test]
fn kinds_and_cursors_parse_from_the_wire() {
    let rows: [Row; 7] = [
        (
            1,
            0,
            &[1],
            Some((Kinds::Screens, Plurality::One, vec![Pointer::Hidden])),
        ),
        (
            2,
            1,
            &[2, 1],
            Some((
                Kinds::Windows,
                Plurality::Many,
                vec![Pointer::Hidden, Pointer::Embedded],
            )),
        ),
        (
            3,
            0,
            &[4, 4, 2],
            Some((
                Kinds::Both,
                Plurality::One,
                vec![Pointer::Embedded, Pointer::Metadata],
            )),
        ),
        (
            3 | 8,
            7,
            &[],
            Some((Kinds::Both, Plurality::Many, vec![Pointer::Hidden])),
        ),
        (
            1,
            0,
            &[3, 8],
            Some((Kinds::Screens, Plurality::One, vec![Pointer::Hidden])),
        ),
        (0, 0, &[1], None),
        (4, 0, &[1], None),
    ];
    for (kinds, multiple, cursors, want) in rows {
        let got = Ask::from_wire("Meet", kinds, multiple, cursors)
            .map(|a| (a.kinds, a.plurality, a.pointers));
        assert_eq!(
            got, want,
            "kinds {kinds} multiple {multiple} cursors {cursors:?}"
        );
    }
}

#[test]
fn the_pointer_wire_values_round_trip() {
    for pointer in [Pointer::Hidden, Pointer::Embedded, Pointer::Metadata] {
        assert_eq!(Pointer::from_wire(pointer.wire()), Some(pointer));
    }
    assert_eq!(Pointer::from_wire(0), None);
    assert_eq!(Pointer::from_wire(3), None);
}

#[test]
fn remembered_parses_from_the_wire() {
    let rows = [
        (0, Remembered::EveryTime),
        (1, Remembered::WhileRunning),
        (2, Remembered::UntilTakenBack),
        (3, Remembered::EveryTime),
        (u32::MAX, Remembered::EveryTime),
    ];
    for (value, want) in rows {
        assert_eq!(Remembered::from_wire(value), want, "{value}");
    }
}
