//! Every case and every flow has its test row in the accept suite (the rows themselves run in
//! `cargo xtask accept agent`; this check needs no daemon, so the gate runs it).

use crate::accept::eval::FLOW_SLUGS;
use crate::accept::hostile::PLANNER_CASES;
use action_eval::load_planner_cases;
use action_live::flows::Flow;
use std::path::PathBuf;

#[test]
fn every_planner_case_has_its_test() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../eval");
    let cases = load_planner_cases(&root.join("hostile-model/planner")).expect("cases");
    assert!(cases.len() >= 25);
    let mut ids: Vec<String> = cases.into_iter().map(|case| case.id).collect();
    let mut tested: Vec<String> = PLANNER_CASES
        .iter()
        .map(|stem| format!("hostile-planner-{stem}"))
        .collect();
    ids.sort();
    tested.sort();
    assert_eq!(
        ids, tested,
        "a planner case has no test row above, or a row has no case"
    );
}

#[test]
fn every_flow_has_its_test() {
    let all: Vec<&str> = Flow::ALL.iter().map(|flow| flow.slug()).collect();
    assert_eq!(
        all, FLOW_SLUGS,
        "a flow has no test row above, or a row has no flow"
    );
}
