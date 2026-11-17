//! A text answer in the preview pane: the question as the heading, then every line of the
//! answer in order, its emphasis set in the UI kit's text runs (`ds-run`), never as markup.

use dioxus::prelude::*;
use shell_model::companion::{
    AnswerFacts, Emphasis, ProgressShown, Span, body_lines, preview_progress, spans,
};
use ui_components::components::content::text_runs::{RunTone, TextLine, TextRun, text};
use ui_components::components::controls::progress::model::{Progress, ProgressStyle};
use ui_components::components::controls::progress::view::ProgressIndicator;
use ui_components::motion::detail::operation::{Operation, PendingToken};
use ui_components::style::tokens::control_size::ControlSize;

/// The answer's lines as paragraphs under the question, and while it is being made, what the
/// companion said as it went.
pub(super) fn answer_text(answer: &AnswerFacts) -> Element {
    let lines: Vec<TextLine> = body_lines(answer)
        .iter()
        .map(|line| runs_of(line))
        .collect();
    let progress = preview_progress(answer);
    rsx! {
        div { class: "shell-answer-text",
            h2 { class: "shell-answer-title", "{answer.prompt}" }
            for (at , line) in lines.iter().enumerate() {
                p { key: "{at}", class: "shell-answer-line", {text(line)} }
            }
            ProgressList { shown: progress }
        }
    }
}

/// What the companion said next to its calls, newest last, the older ones folded into a count,
/// with the activity indicator under them. Nothing when it has said nothing.
#[component]
fn ProgressList(shown: ProgressShown) -> Element {
    // The work began when this list first appeared; the loop restarts only for a new list.
    let working = use_hook(|| Operation::Running(PendingToken::start()));
    if shown.lines.is_empty() {
        return rsx! {};
    }
    let first = shown.earlier;
    rsx! {
        div { class: "shell-answer-progress",
            if shown.earlier > 0 {
                p { class: "shell-answer-progress-line", "+{shown.earlier} earlier" }
            }
            for (at , line) in shown.lines.iter().enumerate() {
                p {
                    key: "{first + at}",
                    class: "shell-answer-progress-line",
                    {text(&runs_of(line))}
                }
            }
            ProgressIndicator {
                style: ProgressStyle::Spinner,
                progress: Progress::Unknown(working),
                size: ControlSize::Small,
            }
        }
    }
}

fn runs_of(line: &str) -> TextLine {
    TextLine::Runs(spans(line).into_iter().map(run_of).collect())
}

fn run_of(span: Span) -> TextRun {
    let tone = match span.emphasis {
        Emphasis::Plain => RunTone::Plain,
        Emphasis::Strong => RunTone::Strong,
        Emphasis::Italic => RunTone::Italic,
        Emphasis::Code => RunTone::Code,
    };
    TextRun::new(span.text, tone)
}
