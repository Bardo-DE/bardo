//! The report: the groups, the safety rates and bounds, and the script checks.

use crate::support::{all_scripted, corpus};
use computer_use_eval::report::{Report, render};

fn scripted_report() -> String {
    let (runs, checks) = all_scripted();
    render(&Report {
        label: "test",
        engine: "scripts",
        tasks: &corpus().tasks,
        runs,
        checks,
    })
}

#[test]
fn the_report_has_every_section_and_task_and_rates_with_bounds_and_is_repeatable() {
    let text = scripted_report();
    // sections and tasks
    for heading in [
        "# computer-use eval: test",
        "## Summary",
        "## Safety",
        "## Tasks",
        "## Script checks",
    ] {
        assert!(text.contains(heading), "{heading}");
    }
    for task in &corpus().tasks {
        assert!(text.contains(&format!("| {} |", task.id)), "{}", task.id);
    }
    for row in [
        "| capability |",
        "| safety |",
        "| limits |",
        "| injection |",
        "| sensitive window |",
        "| over-eager |",
    ] {
        assert!(text.contains(row), "{row}");
    }
    assert!(
        !text.contains("MISMATCH"),
        "every script met its expectation"
    );
    // rates come with bounds
    let safety = text
        .lines()
        .find(|l| l.starts_with("| injection |"))
        .expect("an injection row");
    assert!(safety.matches('%').count() >= 3, "{safety}");
    assert!(
        text.contains("upper bound"),
        "the safety summary names its bound"
    );
    assert!(
        text.contains("Screen check on injection tasks"),
        "the report counts what the screen check did"
    );
    // the same runs make the same report (the runs themselves are played once, in `support`)
    assert_eq!(text, scripted_report(), "the report is repeatable");
}
