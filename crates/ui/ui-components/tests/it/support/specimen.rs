//! Rendered specimens, linted: a component state rendered through dioxus-ssr is held to the
//! markup rules (only classes the stylesheet styles, no hand-written SVG or form control, no
//! literal paint outside the custom properties a component computes). Nothing here compares
//! output with a stored copy; a test that wants a property of the markup asserts it.

use std::sync::LazyLock;
use ui_lint::{Exception, LintConfig, Profile, Rule, markup, markup_warnings};

/// The consumer classes the mail-app specimens put on a button through `extra_class`, and the
/// edit surface 2 specimen on the surface (mailo's `.c-body`, positioned so its text stacks over
/// a selection layer): a consumer's own sheet styles them, as it would in the app, so they are in
/// scope here and nowhere in the UI kit's sheet. `.ds-space-dot` is the picker's Space dot, styled
/// by a sheet above `ds`.
const STAND_INS: &str = ".row-reveal{opacity:0}\n.quiet-until-hover{opacity:0}\n.fold-more{opacity:0}\n\
     .c-body{position:relative}\n.ds-space-dot{}";

/// What the specimens write inline that a consumer may not: colours computed per face, account,
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

/// The stylesheet the specimens are linted against: the kit's, and the consumer stand-ins.
static SHEET: LazyLock<String> =
    LazyLock::new(|| format!("{}\n{STAND_INS}", ui_components::stylesheet()));

/// Specimens whose one reviewed warning is left standing: an `EditSurface` rendered without its
/// optional `label` (making it required is H2's, design/27 section 7).
const REVIEWED: &[(&str, Rule)] = &[("controls/edit_surface/empty", Rule::UnnamedControl)];

/// Lints the rendered markup `html` of the specimen `name` against the kit's stylesheet. Returns
/// the offences, and the Strict profile's warnings beyond the reviewed ones, as text instead of
/// panicking, so a table-driven test reports every case at once.
pub fn lint(name: &str, html: &str) -> Result<(), String> {
    let config = LintConfig {
        exceptions: EXCEPTIONS,
        ..LintConfig::new(&ui_components::kits())
    };
    let mut offences: Vec<String> = markup(html, &SHEET, &config)
        .into_iter()
        .map(|offence| format!("{name}: {:?} {}", offence.rule, offence.text))
        .collect();
    let strict = LintConfig {
        profile: Profile::Strict,
        ..LintConfig::new(&ui_components::kits())
    };
    offences.extend(
        markup_warnings(html, &strict)
            .into_iter()
            .filter(|warning| !REVIEWED.contains(&(name, warning.rule)))
            .map(|warning| {
                format!(
                    "{name}:{} {:?} {}",
                    warning.line, warning.rule, warning.text
                )
            }),
    );
    if offences.is_empty() {
        Ok(())
    } else {
        Err(offences.join("\n"))
    }
}
