//! The root chrome as markup (bar gaps): what each material's root stamps
//! and draws (the window's loose layers and grain; shell chrome's `.ds-frame` group; nothing on
//! a transparent root), the explicit overrides, and a `Surface` on the frame ground.

use dioxus::prelude::*;
use ui_components::assembly::ds::Inject;
use ui_components::prelude::*;
use ui_components::root::chrome::Ground;
use ui_style::space::presets::PRESETS;
use ui_style::tokens::shape::Corner;

#[derive(Props, Clone, PartialEq)]
struct Setup {
    material: Material,
    blur: bool,
    chrome: Option<RootChrome>,
    ground: Option<Ground>,
    surface: Option<Ground>,
    radius: Option<Corner>,
}

#[expect(
    non_snake_case,
    reason = "a Dioxus component is a function named in CamelCase"
)]
#[expect(
    clippy::needless_pass_by_value,
    reason = "a Dioxus component takes its props by value"
)]
fn Root(setup: Setup) -> Element {
    let look = SpaceLook {
        dots: PRESETS[0].dots.to_vec(),
        grain: Grain(35),
        ..SpaceLook::default()
    };
    rsx! {
        Ds {
            appearance: Appearance { theme: Theme::Light, ..Appearance::default() },
            look,
            material: setup.material,
            blur: setup.blur,
            stylesheet: Inject::Host,
            chrome: setup.chrome,
            ground: setup.ground,
            radius: setup.radius,
            if let Some(on) = setup.surface {
                Surface { material: Material::Widget, on: Some(on), p { "inside" } }
            } else {
                p { "inside" }
            }
        }
    }
}

fn render(setup: Setup) -> String {
    let mut dom = VirtualDom::new_with_props(Root, setup);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

fn setup(material: Material) -> Setup {
    Setup {
        material,
        blur: true,
        chrome: None,
        ground: None,
        surface: None,
        radius: None,
    }
}

#[test]
fn each_materials_root_stamps_its_material_and_keeps_its_content() {
    for material in Material::ALL.iter().copied() {
        let html = render(setup(material));
        let stamp = format!("data-material=\"{}\"", material.slug());
        assert!(html.contains(&stamp), "{stamp} missing: {html}");
        assert!(html.contains("<p>inside</p>"), "{html}");
    }
}

#[test]
fn the_overrides_change_what_the_root_draws() {
    let painted = render(Setup {
        chrome: Some(RootChrome::Painted),
        ..setup(Material::Popover)
    });
    assert!(
        painted.contains("ds-frame"),
        "a painted popover draws the frame: {painted}"
    );
    let plain = render(setup(Material::Popover));
    assert!(
        !plain.contains("ds-frame"),
        "a popover is transparent by default: {plain}"
    );

    let transparent = render(Setup {
        chrome: Some(RootChrome::Transparent),
        ..setup(Material::Bar)
    });
    assert!(!transparent.contains("ds-frame"), "{transparent}");
    assert!(
        transparent.contains("data-chrome=\"transparent\""),
        "{transparent}"
    );

    let paper = render(Setup {
        ground: Some(Ground::Paper),
        ..setup(Material::Bar)
    });
    assert!(!paper.contains("data-ground"), "{paper}");

    let frame_surface = render(Setup {
        surface: Some(Ground::Frame),
        ..setup(Material::Window)
    });
    assert!(
        frame_surface.contains("data-ground=\"frame\""),
        "{frame_surface}"
    );

    let radius = render(Setup {
        radius: Some(Corner::Px(Px(12.0))),
        ..setup(Material::Dock)
    });
    assert!(radius.contains("--m-radius:12px"), "{radius}");
    assert!(!render(setup(Material::Dock)).contains("--m-radius"));
}
