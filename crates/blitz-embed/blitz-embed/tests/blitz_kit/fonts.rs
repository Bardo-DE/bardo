//! A face registered on `SharedFonts` actually reaches a document (without it a document falls
//! back to fontconfig's faces): the family is reported, and a document naming that family in
//! `font-family` measures differently from one naming only `system-ui`.
//!
//! Uses a real font's bytes rather than a synthetic one: Space Mono, kept in this crate's own
//! `fixtures/` (its licence beside it). A machine that has the family installed fails the
//! comparison test, saying so, rather than passing without testing anything.

use crate::support;

use blitz_dom::FontContext;
use blitz_embed::fonts::{FontFaces, SharedFonts};

/// The family the fixture face names.
const FAMILY: &str = "Space Mono";

/// The fixture face's bytes.
const FACE: &[u8] = include_bytes!("fixtures/space-mono-normal-400-latin.ttf");

/// The width `text` lays out at in `stack` under `fonts`.
fn width(fonts: FontContext, stack: &str, text: &str) -> f64 {
    let (mut doc, body) = support::page_with(400, 100, 1.0, Some(fonts));
    let style = format!("display:inline-block;white-space:pre;font-size:16px;font-family:{stack}");
    let line = support::div(&mut doc, body, "line", &style);
    support::text(&mut doc, line, text);
    doc.resolve(0.0);
    doc.get_client_bounding_rect(line)
        .expect("the line is laid out")
        .width
}

#[test]
fn a_registered_face_is_a_family() {
    let families = SharedFonts::system().register(FACE);
    assert!(
        families.contains(&FAMILY.to_string()),
        "{FAMILY} is not in {families:?}"
    );
}

#[test]
fn a_registered_faces_stack_measures_differently_from_system_ui() {
    // The bug this pins: with only `SharedFonts::system()`, a font-family naming a bundled face
    // that is not also a system font fell straight through to `system-ui`, indistinguishable
    // from not naming it at all.
    let mut machine = SharedFonts::system().for_document();
    assert!(
        !machine.collection.family_names().any(|name| name == FAMILY),
        "{FAMILY} is installed on this machine, so it resolves without being registered; this test needs a machine without it"
    );
    let family = format!("\"{FAMILY}\"");
    let system = width(
        SharedFonts::system().for_document(),
        "system-ui,sans-serif",
        "shell-host",
    );

    let registered = width(
        SharedFonts::system_with(&FontFaces(vec![FACE])).for_document(),
        &family,
        "shell-host",
    );
    assert!(
        (registered - system).abs() >= 1.0,
        "registering {FAMILY} did not change how it lays out ({registered} vs system-ui's {system})"
    );
}

#[test]
fn bundled_fonts_hold_the_bullet_face_and_no_system_family() {
    let mut context = SharedFonts::bundled(&FontFaces::default()).for_document();
    assert_eq!(context.collection.family_names().count(), 1);
}
