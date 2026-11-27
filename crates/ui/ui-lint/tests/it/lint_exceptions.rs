//! `LintConfig.exceptions`: an exception silences exactly its (rule, selector) pair, in a
//! stylesheet and in markup, and `assert_clean` says how much each one silenced and fails on
//! one that silenced nothing.

use ui_lint::{Exception, LintConfig, Rule, assert_clean, markup, stylesheet};

const TRUNCATE_MASK: &[Exception] = &[Exception {
    rule: Rule::HexColour,
    selector: ".fade",
    reason: "mask alpha only",
}];

#[test]
fn an_exception_suppresses_exactly_its_rule_and_selector() {
    let css = ".fade { mask-image: linear-gradient(#000, transparent); }\n\
               .fade .x { color: #000; }\n\
               .fade { color: red; }";
    let config = LintConfig {
        exceptions: TRUNCATE_MASK,
        ..LintConfig::new(&ui_components::kits())
    };
    let kept: Vec<(Rule, String)> = stylesheet(css, &config)
        .into_iter()
        .map(|offence| (offence.rule, offence.selector))
        .collect();
    assert_eq!(
        kept,
        vec![
            (Rule::HexColour, ".fade .x".to_owned()),
            (Rule::NamedColour, ".fade".to_owned()),
        ],
        "the other selector and the other rule are not suppressed"
    );
    let bare = stylesheet(css, &LintConfig::new(&ui_components::kits()));
    assert_eq!(bare.len(), kept.len() + 1, "without it, it fires");
}

#[test]
fn a_markup_exception_names_the_element() {
    const INLINE: &[Exception] = &[Exception {
        rule: Rule::HexColour,
        selector: "span.ds-space-swatch",
        reason: "the editor's swatch paints its picked colour inline",
    }];
    let html = "<span class=\"ds-space-swatch\" style=\"background:#944242\"></span>";
    let bare = markup(
        html,
        ".ds-space-swatch{}",
        &LintConfig::new(&ui_components::kits()),
    );
    assert_eq!(bare.len(), 1, "{bare:?}");
    assert_eq!(bare[0].selector, "span.ds-space-swatch");
    let config = LintConfig {
        exceptions: INLINE,
        ..LintConfig::new(&ui_components::kits())
    };
    assert_eq!(
        markup(html, ".ds-space-swatch{}", &config),
        [] as [ui_lint::Offence; 0]
    );
}

#[test]
fn assert_clean_passes_when_exceptions_cover_everything() {
    let config = LintConfig {
        exceptions: TRUNCATE_MASK,
        ..LintConfig::new(&ui_components::kits())
    };
    assert_clean(
        ".fade { mask-image: linear-gradient(#000, transparent); }",
        &config,
    );
}

#[test]
#[should_panic(expected = "1 x HexColour on .fade (mask alpha only)")]
fn assert_clean_counts_what_each_exception_suppressed() {
    let config = LintConfig {
        exceptions: TRUNCATE_MASK,
        ..LintConfig::new(&ui_components::kits())
    };
    assert_clean(
        ".fade { mask-image: linear-gradient(#000, transparent); color: red; }",
        &config,
    );
}

/// An exception for an offence that has gone: the `.fade` rule no longer writes `#000`.
const CLEAN_FADE: &str = ".fade { mask-image: linear-gradient(var(--ink), transparent); }";

#[test]
#[should_panic(
    expected = "1 stale exception(s), which suppressed nothing:\n  HexColour on .fade (mask alpha only)"
)]
fn a_stale_exception_fails() {
    let config = LintConfig {
        exceptions: TRUNCATE_MASK,
        ..LintConfig::new(&ui_components::kits())
    };
    assert_clean(CLEAN_FADE, &config);
}

#[test]
fn a_two_megabyte_single_line_page_reports_the_late_offence_position() {
    let filler = "<p class=\"ds-ok\">é</p>";
    let mut html = filler.repeat(2_000_000 / filler.len());
    let late_column =
        u32::try_from(html.chars().count()).expect("a page of two million characters") + 1;
    html.push_str("<p class=\"nope\">x</p>");
    let offences = markup(&html, ".ds-ok{}", &LintConfig::new(&ui_components::kits()));
    assert_eq!(offences.len(), 1, "{offences:?}");
    assert_eq!((offences[0].line, offences[0].column), (1, late_column));
    assert_eq!(offences[0].rule, Rule::UnstyledClass);
}
