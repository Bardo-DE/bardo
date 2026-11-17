//! The goldens' hygiene, in one place: every golden under `tests/snapshots` lints clean as
//! markup against the stylesheet (only classes it styles, no hand-written SVG or form control,
//! no literal paint outside the custom properties a component computes), and every specimen of
//! the suites that keep specimens lints clean and has every `ds-` class styled. A suite with
//! specimens adds a `hygiene_failures` and a line here.

use crate::support::golden;
use ui_lint::{Exception, LintConfig, Rule, markup};

/// The consumer classes the mail-app goldens put on a button through `extra_class`, and the edit
/// surface 2 golden on the surface (mailo's `.c-body`, positioned so its text stacks over a
/// selection layer): a consumer's own sheet styles them, as it would in the app, so they are in
/// scope here and nowhere in Bardo's sheet. `.ds-space-dot` is the picker's Space dot, styled by
/// a sheet above `ds`.
const STAND_INS: &str = ".row-reveal{opacity:0}\n.quiet-until-hover{opacity:0}\n.fold-more{opacity:0}\n\
     .c-body{position:relative}\n.ds-space-dot{}";

/// What the goldens write inline that a consumer may not: colours computed per face, account,
/// provider, tile or Space, and the send ring's two circles. Each is data the palette or the app
/// computes, not a theme colour.
const EXCEPTIONS: &[Exception] = &[
    Exception {
        rule: Rule::HexColour,
        selector: "span.ds-avatar",
        reason: "the person hue and account colour are computed per face (O-7); the letter is #fff (O-3)",
    },
    Exception {
        rule: Rule::HexColour,
        selector: "span.ds-provider",
        reason: "a provider's identity colour enters as --pc (section 28, O-3)",
    },
    Exception {
        rule: Rule::HexColour,
        selector: "button.ds-space-dot",
        reason: "a Space dot is painted with its FrameVars gradient",
    },
    Exception {
        rule: Rule::HexColour,
        selector: "span.ds-icon-tile",
        reason: "a tile's ground is the caller's identity colour, entering as --tile-bg",
    },
    Exception {
        rule: Rule::RawMarkup,
        selector: "svg.ds-send-ring",
        reason: "the send ring is two circles Rust redraws per tick, not a glyph (O-20, spike S6)",
    },
];

/// The golden directories whose files this test lints, with the least number each must hold.
const DIRS: &[(&str, usize)] = &[
    ("controls", 60),
    ("forms", 1),
    ("lists", 1),
    ("overlays", 1),
    ("structure", 1),
];

/// Coherence rule 2 on Bardo's own output: every golden of every directory above.
#[test]
fn every_golden_lints_clean() {
    let config = LintConfig {
        exceptions: EXCEPTIONS,
        ..LintConfig::new(&ui_components::kits())
    };
    let css = format!("{}\n{STAND_INS}", ui_components::stylesheet());
    let mut failures = Vec::new();
    for &(dir, least) in DIRS {
        let goldens = golden::all_in(dir);
        assert!(
            goldens.len() >= least,
            "{dir}: only {} goldens",
            goldens.len()
        );
        for (name, html) in &goldens {
            failures.extend(
                markup(html, &css, &config)
                    .into_iter()
                    .map(|offence| format!("{name}: {:?} {}", offence.rule, offence.text)),
            );
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_specimen_lints_clean_and_every_class_is_styled() {
    let failures: Vec<String> = [
        crate::alert_ssr::hygiene_failures(),
        crate::launcher_parts_ssr::hygiene_failures(),
        crate::pdf_thumb_ssr::hygiene_failures(),
        crate::status_bar_ssr::hygiene_failures(),
        crate::voice_orb_ssr::hygiene_failures(),
    ]
    .concat();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
