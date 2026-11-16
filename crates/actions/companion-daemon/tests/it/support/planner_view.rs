//! A planner over a scripted model, and the view and catalogue the planner tests read it with.
#![allow(
    dead_code,
    reason = "each test binary uses a different part of the shared support"
)]

use super::infer::{Say, ScriptedInfer};
use account_core::AppName;
use action_core::*;
use action_manifest::ValidManifest;
use agent_loop::{Sources, assemble};
use base_ids::Count;
use companion_daemon::*;
use provenance::Integrity;
use std::path::PathBuf;

pub fn manifest(file: &str) -> ValidManifest {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../manifests")
        .join(file);
    let text = std::fs::read_to_string(&path).expect("manifest");
    action_manifest::parse(&text).expect("valid")
}

pub fn catalogue() -> Catalogue {
    Catalogue::from_manifests(&[
        action_fake::mail_manifest().expect("mail"),
        manifest("org.bardo.Memory.toml"),
        manifest("org.bardo.Companion.toml"),
    ])
}

pub fn view(cards: Vec<ActionCard>) -> PlannerView {
    let config = AgentConfig::default();
    assemble(
        &config.assembler,
        &Sources {
            cards,
            profile: vec![ProfileLine("Prefers short answers".into())],
            primer: Some(PrimerText("Eve is the landlord".into())),
            rollup: None,
            roster: Roster::default(),
            episodes: vec![],
            recalled: vec![],
            context: ContextView {
                app: AppName::parse("org.bardo.Shell").expect("app"),
                window: Reveal::Plain(String::new()),
                here: HereView::Nowhere,
                selection: SelectionView::Nothing,
                visible: VisibleView {
                    kind: None,
                    items: vec![],
                    total: Count(0),
                },
                text_target: TextTargetView::None,
            },
            turns: vec![],
            history: vec![],
            handles: vec![],
            inbox: vec![],
            skills: vec![],
            skill_texts: vec![],
            shown: vec![],
            taint: Integrity::Trusted,
            task_policy: None,
            summary: None,
        },
    )
}

pub fn planner(script: Vec<Say>) -> (PlannerModel<ScriptedInfer>, ScriptedInfer) {
    let infer = ScriptedInfer::new(script);
    let c = catalogue();
    (PlannerModel::new(infer.clone()).with_catalogue(c), infer)
}
