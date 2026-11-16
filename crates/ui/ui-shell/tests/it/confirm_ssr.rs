//! The confirmation card as markup: one root with its own `ds-` class, the arm state and the
//! offer written on it, lint clean, and the class styled. Its look is the fill's.

use dioxus::prelude::*;
use ui_components::assembly::ds::Inject;
use ui_components::components::companion::mark::AppMark;
use ui_components::prelude::*;
use ui_core::vocab::EffectMark;
use ui_lint::{LintConfig, markup};
use ui_shell::confirm::model::{ConfirmView, GestureMark, TaintLine};
use ui_shell::confirm::view::ConfirmCard;

fn mark(name: &str) -> AppMark {
    AppMark {
        icon: Icon::Search.into(),
        name: name.to_owned(),
    }
}

fn view(armed: bool) -> ConfirmView {
    ConfirmView {
        asker: mark("Assistant"),
        target: mark("Mail"),
        title: "Send this email?".to_owned(),
        facts: Vec::new(),
        verb: "Send".to_owned(),
        effect: EffectMark::Outbound,
        taint: TaintLine::Clean,
        offer_always: false,
        gesture: GestureMark::Press,
        armed,
    }
}

fn card(armed: bool) -> String {
    let mut dom = VirtualDom::new_with_props(
        |armed: bool| {
            rsx! {
                Ds { appearance: Appearance::default(), material: Material::Window, stylesheet: Inject::Host,
                    ConfirmCard { view: view(armed), on_answer: |_| {} }
                }
            }
        },
        armed,
    );
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

#[test]
fn the_card_names_its_arm_state_and_offer() {
    let arming = card(false);
    assert!(arming.contains("data-arm=\"arming\""), "{arming}");
    assert!(arming.contains("data-offer=\"once-only\""), "{arming}");
    let armed = card(true);
    assert!(armed.contains("data-arm=\"armed\""), "{armed}");
}

#[test]
fn the_card_lints_clean_and_its_class_is_styled() {
    let sheet = ui_shell::stylesheet();
    let html = card(true);
    let offences: Vec<String> = markup(&html, sheet, &LintConfig::new(&ui_shell::kits()))
        .into_iter()
        .map(|offence| format!("{:?} {}", offence.rule, offence.text))
        .collect();
    assert!(offences.is_empty(), "{}", offences.join("\n"));
    assert!(html.contains("class=\"ds-confirm-card\""), "{html}");
    assert!(sheet.contains(".ds-confirm-card"));
}
