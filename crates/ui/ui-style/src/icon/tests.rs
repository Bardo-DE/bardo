use super::Icon;
use super::native::Native;
use super::render::{Glyph, GlyphProps, IconSize};
use super::style::GlyphStyle;
use crate::icon::shape::Shape;
use dioxus::prelude::*;

/// An icon's outline markup: the geometry these tests pin is Lucide's.
fn markup(icon: Icon) -> String {
    markup_in(icon, GlyphStyle::Outline)
}

fn markup_in(icon: Icon, style: GlyphStyle) -> String {
    let mut dom = VirtualDom::new_with_props(
        Glyph,
        GlyphProps {
            icon,
            size: IconSize::Base,
            style,
            cut: super::posed::Thousandths::default(),
        },
    );
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

/// Opening-tag names, in order. A close tag is not an element.
fn element_names(html: &str) -> Vec<&str> {
    let bytes = html.as_bytes();
    let mut names = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'<' {
            index += 1;
            continue;
        }
        index += 1;
        if index >= bytes.len() || matches!(bytes[index], b'/' | b'!' | b'?') {
            continue;
        }
        let start = index;
        while index < bytes.len()
            && !matches!(bytes[index], b' ' | b'\t' | b'\n' | b'\r' | b'>' | b'/')
        {
            index += 1;
        }
        names.push(&html[start..index]);
    }
    names
}

fn shape_tag(shape: &Shape) -> &'static str {
    match shape {
        Shape::Path(_) | Shape::Solid(_) => "path",
        Shape::Circle { .. } => "circle",
        Shape::Rect { .. } => "rect",
    }
}

#[test]
fn every_icon_is_an_svg_of_its_shapes() {
    let mut failures = Vec::new();
    for icon in Icon::MAILO.iter().chain(Icon::ACTIONS).chain(Icon::CONTROL) {
        let page = markup(*icon);
        let shapes = icon.shapes();
        let names = element_names(&page);
        let expect: Vec<&str> = std::iter::once("svg")
            .chain(shapes.iter().map(shape_tag))
            .collect();
        if shapes.is_empty()
            || names != expect
            || !page.contains("<svg")
            || !page.contains("class=\"ds-ic\"")
            || !page.contains("stroke=\"currentColor\"")
            || !page.contains("stroke-width=\"2\"")
            || !page.contains("fill=\"none\"")
            || !page.contains("viewBox=\"0 0 24 24\"")
            || !page.contains("aria-hidden=\"true\"")
        {
            failures.push(format!(
                "{icon:?}: {} shapes, names {names:?}, page {page}",
                shapes.len()
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_icon_draws_its_native_pick_by_default() {
    let mut failures = Vec::new();
    for &icon in Icon::ALL {
        let page = markup_in(icon, GlyphStyle::default());
        let (shapes, style) = match icon.native() {
            Native::Solid => (icon.solid_shapes(), "solid"),
            Native::Drawn(shapes) => (shapes, "solid"),
            Native::Outline | Native::Pair => (icon.shapes(), "outline"),
        };
        let expect: Vec<&str> = std::iter::once("svg")
            .chain(shapes.iter().map(shape_tag))
            .collect();
        if element_names(&page) != expect || !page.contains(&format!("data-style=\"{style}\"")) {
            failures.push(format!("{icon:?}: {page}"));
        }
        let filled = style == "solid";
        if filled != (page.contains("stroke=\"none\"") && !page.contains("stroke-width")) {
            failures.push(format!("{icon:?} paint: {page}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn an_explicit_style_beats_the_pick() {
    let page = markup_in(Icon::Bluetooth, GlyphStyle::Solid);
    assert!(page.contains("data-style=\"solid\""), "{page}");
    let page = markup_in(Icon::Play, GlyphStyle::Outline);
    assert!(page.contains("data-style=\"outline\"") && page.contains("stroke-width"));
}

#[test]
fn a_replacement_is_filled_paths_on_the_24_grid() {
    for icon in [Icon::Download, Icon::Upload, Icon::Sparkles] {
        let Native::Drawn(shapes) = icon.native() else {
            panic!("{icon:?}: not drawn");
        };
        for shape in shapes {
            let Shape::Solid(d) = shape else {
                panic!("{icon:?}: {shape:?}");
            };
            for number in d.split(|c: char| c.is_ascii_alphabetic() || c.is_whitespace()) {
                if let Ok(value) = number.parse::<f32>() {
                    assert!(value.abs() <= 24.0, "{icon:?}: {value} in {d}");
                }
            }
        }
    }
}

#[test]
fn an_outline_is_the_stroke_and_names_its_style() {
    let page = markup(Icon::Star);
    assert!(page.contains("data-style=\"outline\""), "{page}");
    assert!(page.contains("stroke-linecap=\"round\""), "{page}");
}
