//! Coherence rules 1 and 2 for the launcher's panel over results (the empty panel and the
//! catcher are rows of `crates/shell/shell-app/tests/lint.rs`): the real `Launcher` (open over results, with
//! its preview pane, in companion mode with an answer card) through the markup lint, rendered by
//! the harness (its palette embedded, measured and focused as live) and by an SSR pass. All of it
//! runs under `Profile::Details` (design/26-DETAILS.md): `Strict` plus timing only by the
//! grammar's tokens and no `infinite` loop.

use crate::launcher_harness;

use launcher_harness::launcher_support;

use dioxus::prelude::*;
use shell_launcher_ui::Launcher;
use shell_settings::ShellFile;
use shell_ui_kit::links::LauncherLink;
use shell_ui_kit::settings::SettingsFeed;
use shell_ui_kit::surface::{MountProps, mount};
use ui_lint::{LintConfig, Profile, assert_clean, markup};

const LAUNCHER_CSS: &str = include_str!("../../src/launcher/style.css");

fn details() -> LintConfig {
    LintConfig {
        profile: Profile::Details,
        ..LintConfig::new(&shell_kit::kits())
    }
}

/// `root` on the fake services with the launcher open on the test output over "fi"'s results,
/// rendered to HTML inside a runtime (the appearance hook watches files and the portal on it).
fn render_ssr(root: VirtualDom) -> String {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("a runtime builds"); // test-only
    let _entered = rt.enter();
    let (services, feeds) = launcher_support::services();
    feeds.results.send_replace(shell_model::results::Results {
        generation: shell_launcher::Generation(1),
        items: launcher_support::search("fi"),
        sections: Vec::new(),
    });
    let link = LauncherLink::new();
    link.open(launcher_support::fake::output().id, "fi".to_owned());
    let mut dom = root;
    dom.provide_root_context(services);
    dom.provide_root_context(SettingsFeed::fixed(ShellFile::default()));
    dom.provide_root_context(link);
    dom.provide_root_context(launcher_support::providers());
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

fn panel_html() -> String {
    launcher_harness::open("fi").harness.html()
}

/// The SSR pass of the panel: its root and the Ds, the palette not yet mounted.
fn panel_ssr() -> String {
    render_ssr(VirtualDom::new_with_props(
        mount::<Launcher>,
        MountProps::<Launcher>::new(launcher_support::fake::output(), ()),
    ))
}

fn full_css() -> String {
    format!("{}\n{}", shell_kit::stylesheet(), LAUNCHER_CSS)
}

/// The panel over results, with the report's preview pane up, and in companion mode: named
/// documents, each drawn by the harness.
fn panel_states() -> Vec<(&'static str, String)> {
    use launcher_harness::launcher_support::companion::{companion, draft, fresh};
    // The report's look is held (until `_held` drops), so the pane is the pending one however fast
    // the page would raster.
    let (look, _held) = launcher_harness::launcher_support::m9::gated(
        launcher_harness::launcher_support::m9::folder().join("m9-report.pdf"),
    );
    let mut previewing = launcher_harness::open_looking("m9", Some(look));
    previewing.press(launcher_harness::Press::QuickLook);
    previewing
        .harness
        .advance(std::time::Duration::from_millis(60));
    let pane = previewing.harness.html();
    assert!(pane.contains("ds-preview"), "the pane is up");
    let answering = launcher_harness::open_in("", shell_launcher::LauncherMode::Companion, |bar| {
        bar.companion
            .send_replace(companion(vec![draft()], fresh()));
    })
    .harness
    .html();
    assert!(
        answering.contains("ds-answer-card"),
        "the card is up: {answering}"
    );
    vec![
        ("results", panel_html()),
        ("results over SSR", panel_ssr()),
        ("preview pane", pane),
        ("companion answer card", answering),
    ]
}

#[test]
fn the_panel_renders_the_palette() {
    let html = panel_html();
    assert!(html.contains("class=\"ds-palette\""), "{html}");
    assert!(html.contains("id=\"launcher-card\""), "the blur element");
    let ssr = panel_ssr();
    assert!(ssr.contains("id=\"panel\""), "{ssr}");
}

#[test]
fn the_launcher_stylesheet_is_clean() {
    assert_clean(LAUNCHER_CSS, &details());
}

#[test]
fn the_panel_is_clean_in_each_state() {
    let css = full_css();
    for (state, html) in panel_states() {
        let offences = markup(&html, &css, &details());
        assert!(offences.is_empty(), "{state}: {offences:#?}");
    }
}
