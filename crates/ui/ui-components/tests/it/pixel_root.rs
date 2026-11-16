//! The root's device scale as markup (design/01-LAYOUT.md section 2.1): a `Ds` with a
//! fractional `scale` writes every pixel token's input inline, one given through the host's
//! the host's `HostSignals` scale does the same, a nested `Surface` writes nothing of its own (it inherits the
//! root's inputs), a root at 1x or with no scale writes nothing, and a `Glyph` under a 1.25 root
//! snaps its stroke.

use dioxus::core::VirtualDom;
use dioxus::prelude::*;
use ui_components::assembly::ds::Inject;
use ui_components::prelude::*;
use ui_style::icon::render::Glyph;

#[derive(Props, Clone, PartialEq)]
struct Setup {
    given: Option<Scale>,
    host: Option<Scale>,
}

#[allow(non_snake_case)]
fn Root(setup: Setup) -> Element {
    if let Some(scale) = setup.host {
        use_context_provider(|| HostSignals {
            modality: Signal::new(false),
            scale: Signal::new(scale),
            activity: Signal::new(true),
        });
    }
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Window, stylesheet: Inject::Host,
            scale: setup.given,
            Surface { material: Material::Popover,
                Glyph { icon: Icon::Plus, size: IconSize::Base, style: GlyphStyle::Outline }
            }
        }
    }
}

fn render(given: Option<Scale>, host: Option<Scale>) -> String {
    let mut dom = VirtualDom::new_with_props(Root, Setup { given, host });
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

#[test]
fn the_host_scale_reaches_a_root_without_one_and_the_prop_wins() {
    let hosted = render(None, Some(Scale(150)));
    assert!(hosted.contains("--scale-px:0.8px;"), "{hosted}");
    let both = render(Some(Scale(210)), Some(Scale(150)));
    assert!(both.contains("--scale-px:0.57143px;"), "{both}");
    assert!(!both.contains("--scale-px:0.8px;"), "{both}");
}

#[test]
fn at_one_or_with_no_scale_the_root_writes_nothing() {
    for html in [render(None, None), render(Some(Scale::ONE), None)] {
        assert!(!html.contains("--scale-"), "{html}");
        assert!(html.contains("stroke-width=\"2\""), "{html}");
    }
}

#[test]
fn a_glyph_under_a_fractional_root_snaps_its_stroke() {
    let html = render(Some(Scale(150)), None);
    assert!(html.contains("stroke-width=\"2.4\""), "{html}");
}
