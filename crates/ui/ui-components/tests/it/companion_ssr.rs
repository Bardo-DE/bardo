//! The companion's placeholder components as markup: each draws one root with its own `ds-` class,
//! lints clean under the design system's stylesheet, and that class is styled. Their looks are
//! the fill's; this pins that the lint passes and the sheets are registered, and that the
//! unseen-outcome mark comes and goes with what the caller passes.

use dioxus::prelude::*;
use ui_components::assembly::ds::Inject;
use ui_components::components::companion::activity::model::{ActivityEntry, ActivityState, UndoOffer};
use ui_components::components::companion::activity::view::ActivityStrip;
use ui_components::components::companion::answer::action::{ActionRole, CardAction, CardActionId};
use ui_components::components::companion::answer::footer::{
    CardFooter, ServedByView, ServedPlace, ServedPlaceKind,
};
use ui_components::components::companion::answer::model::{AnswerView, TextAnswer};
use ui_components::components::companion::answer::refused::{RefusedAnswer, RefusedView};
use ui_components::components::companion::answer::view::AnswerCard;
use ui_components::components::companion::chips::view::ContextChips;
use ui_components::components::companion::draft::model::{DraftReply, PersonLine};
use ui_components::components::companion::form::model::{
    CompactForm, FieldKeyId, FormField, FormInput, FormValue,
};
use ui_components::components::companion::memory::model::{ConsolidationDiff, MemoryDay};
use ui_components::components::companion::memory::view::{ConsolidationView, MemoryTimeline};
use ui_components::components::companion::orb::model::OrbSize;
use ui_components::components::companion::orb::view::CompanionOrb;
use ui_components::components::companion::outcome::model::Outcome;
use ui_components::components::companion::plan::model::{
    PlanGroup, PlanPhase, PlanStep, PlanStepState, PlanView, StepId,
};
use ui_components::components::companion::plan::view::PlanList;
use ui_components::components::companion::proposed_event::model::{
    BusyKind, DaySlot, MinuteSpan, ProposedEvent,
};
use ui_components::components::companion::replace::model::{ReplacePhase, ReplaceProposal};
use ui_components::components::companion::replace::view::ReplaceBar;
use ui_components::components::companion::run_row::model::{RunPlace, RunRowView, RunState};
use ui_components::components::companion::run_row::view::RunRow;
use ui_components::components::companion::served_by::view::ServedByChip;
use ui_components::components::companion::{ChipKind, ContextChip};
use ui_components::components::content::avatar::{AvatarFace, AvatarShape, AvatarSize, AvatarTone};
use ui_components::components::content::text_runs::{RunTone, TextLine, TextRun};
use ui_components::components::fields::text_field_model::Validity;
use ui_components::prelude::*;
use ui_core::vocab::{ActorMark, Availability, CompanionPresence, EffectMark, Tally};
use ui_lint::{LintConfig, markup};

fn footer() -> CardFooter {
    CardFooter {
        served_by: served_by(),
        sources: Vec::new(),
        scope: Vec::new(),
    }
}

fn served_by() -> ServedByView {
    ServedByView {
        model: "local".to_owned(),
        place: ServedPlace {
            kind: ServedPlaceKind::ThisComputer,
            provider: None,
        },
    }
}

fn app() -> ui_components::components::companion::mark::AppMark {
    ui_components::components::companion::mark::AppMark {
        icon: Icon::Search.into(),
        name: "Mail".to_owned(),
    }
}

fn root(body: Element) -> Element {
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Window, stylesheet: Inject::Host,
            {body}
        }
    }
}

/// A specimen: the root class it draws and how it is made.
type Specimen = (&'static str, fn() -> Element);

const SPECIMENS: &[Specimen] = &[
    ("ds-companion-orb", || {
        root(rsx! { CompanionOrb { presence: CompanionPresence::Working, size: OrbSize::Bar } })
    }),
    ("ds-context-chips", || {
        let chips = vec![ContextChip {
            kind: ChipKind::Results,
            label: "Results".to_owned(),
            count: Some(Tally(12)),
            removable: true,
        }];
        root(rsx! { ContextChips { chips } })
    }),
    ("ds-answer-card", || {
        let view = AnswerView::Text(TextAnswer {
            body: vec!["Done.".into()],
            streaming: false,
            actions: Vec::new(),
            footer: footer(),
        });
        root(rsx! { AnswerCard { view, on_action: |_| {} } })
    }),
    ("ds-plan-list", || {
        let view = PlanView {
            title: "Forward".to_owned(),
            groups: Vec::new(),
            phase: PlanPhase::Draft,
            footer: footer(),
        };
        root(rsx! { PlanList { view, on_out: |_| {} } })
    }),
    ("ds-replace-bar", || {
        let proposal = ReplaceProposal {
            original: "teh".to_owned(),
            proposed: "the".to_owned(),
            phase: ReplacePhase::Proposed,
            footer: footer(),
        };
        root(rsx! { ReplaceBar { proposal, on_in: |_| {} } })
    }),
    ("ds-run-row", || {
        let view = RunRowView {
            goal: "Book a table".to_owned(),
            app: app(),
            step: Tally(3),
            budget: Some(Tally(40)),
            thought: None,
            place: RunPlace::AgentWorkspace,
            state: RunState::Running,
            served_by: served_by(),
            actions: Vec::new(),
        };
        root(rsx! { RunRow { view, on_action: |_| {} } })
    }),
    ("ds-activity-strip", || {
        let entries = vec![ActivityEntry {
            key: ui_components::stack::toast_hub::UndoToken(1),
            title: "Archived 3 threads".to_owned(),
            actor: ActorMark::Companion,
            app: Some(app()),
            when: "now".to_owned(),
            state: ActivityState::Done,
            undo: UndoOffer::Undo,
        }];
        root(rsx! { ActivityStrip { entries, on_undo: |_| {}, on_open: |_| {} } })
    }),
    ("ds-memory-timeline", || {
        let days: Vec<MemoryDay> = Vec::new();
        root(rsx! { MemoryTimeline { days, on_verb: |_| {} } })
    }),
    ("ds-consolidation-view", || {
        let diff = ConsolidationDiff {
            night: "Last night".to_owned(),
            lines: Vec::new(),
        };
        root(rsx! { ConsolidationView { diff } })
    }),
    ("ds-served-by-chip", || {
        root(rsx! { ServedByChip { current: served_by(), options: Vec::new(), on_choose: |_| {} } })
    }),
];

fn render(make: fn() -> Element) -> String {
    let mut dom = VirtualDom::new(make);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

#[test]
fn every_placeholder_draws_its_root_class_and_it_is_styled() {
    let sheet = ui_components::stylesheet();
    for (class, make) in SPECIMENS {
        let html = render(*make);
        assert!(
            html.contains(&format!("class=\"{class}\"")),
            "{class}: {html}"
        );
        let needle = format!(".{class}");
        let styled = sheet.match_indices(&needle).any(|(at, _)| {
            !sheet[at + needle.len()..]
                .starts_with(|c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        });
        assert!(styled, "{class} is not styled");
    }
}

#[test]
fn every_placeholder_lints_clean() {
    let sheet = ui_components::stylesheet();
    let mut failures = Vec::new();
    for (class, make) in SPECIMENS {
        for offence in markup(&render(*make), sheet, &LintConfig::new(&ui_components::kits())) {
            failures.push(format!("{class}: {:?} {}", offence.rule, offence.text));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn the_orb_names_its_presence_and_size() {
    let html = render(SPECIMENS[0].1);
    assert!(html.contains("data-presence=\"working\""), "{html}");
    assert!(html.contains("data-size=\"bar\""), "{html}");
}

/// What carries the mark under test.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Carrier {
    Orb,
    Row,
}

#[derive(Clone, PartialEq)]
struct Marked {
    carrier: Carrier,
    outcome: Option<Outcome>,
}

#[allow(non_snake_case)]
fn MarkedStage(props: Marked) -> Element {
    let Marked { carrier, outcome } = props;
    match carrier {
        Carrier::Orb => root(rsx! { CompanionOrb { presence: CompanionPresence::Idle, outcome } }),
        Carrier::Row => {
            let view = RunRowView {
                goal: "Book a table".to_owned(),
                app: app(),
                step: Tally(3),
                budget: None,
                thought: None,
                place: RunPlace::InPlace,
                state: RunState::Done,
                served_by: served_by(),
                actions: Vec::new(),
            };
            root(rsx! { RunRow { view, outcome, on_action: |_| {} } })
        }
    }
}

fn render_marked(carrier: Carrier, outcome: Option<Outcome>) -> String {
    let mut dom = VirtualDom::new_with_props(MarkedStage, Marked { carrier, outcome });
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

/// An unseen outcome draws one still mark on the orb and on a run row, in its own tone word; the
/// caller passing `None` clears it, and the orb's presence is not touched either way.
#[test]
fn an_unseen_outcome_marks_the_orb_and_a_run_row_until_the_caller_clears_it() {
    for carrier in [Carrier::Orb, Carrier::Row] {
        for outcome in [Outcome::Done, Outcome::Failed] {
            let html = render_marked(carrier, Some(outcome));
            let word = format!("data-outcome=\"{}\"", outcome.slug());
            assert_eq!(
                html.matches("ds-outcome-mark").count(),
                1,
                "{carrier:?}: {html}"
            );
            assert!(html.contains(&word), "{carrier:?} {outcome:?}: {html}");
            assert!(
                !html.contains("animation"),
                "{carrier:?}: the mark is still"
            );
        }
        let cleared = render_marked(carrier, None);
        assert!(
            !cleared.contains("ds-outcome-mark"),
            "{carrier:?}: {cleared}"
        );
    }
    let marked = render_marked(Carrier::Orb, Some(Outcome::Failed));
    assert!(marked.contains("data-presence=\"idle\""), "{marked}");
}

/// What an answer card is drawn from.
#[derive(Clone, PartialEq)]
struct Card {
    view: AnswerView,
}

#[allow(non_snake_case)]
fn CardStage(props: Card) -> Element {
    let view = props.view;
    root(rsx! { AnswerCard { view, focused: Some(0), on_action: |_| {} } })
}

/// A card drawn, and checked against the design system's lint on the way.
fn render_card(view: AnswerView) -> String {
    let mut dom = VirtualDom::new_with_props(CardStage, Card { view });
    dom.rebuild_in_place();
    let html = dioxus_ssr::render(&dom);
    let offences = markup(&html, ui_components::stylesheet(), &LintConfig::new(&ui_components::kits()));
    assert!(offences.is_empty(), "{offences:?}\n{html}");
    html
}

fn action(id: &str, label: &str, role: ActionRole, effect: EffectMark) -> CardAction {
    CardAction {
        id: CardActionId(id.to_owned()),
        label: label.to_owned(),
        role,
        effect,
        keys: None,
        availability: Availability::Enabled,
    }
}

fn person() -> PersonLine {
    PersonLine {
        name: "Ada Lovelace".to_owned(),
        address: "ada@example.com".to_owned(),
        avatar: AvatarFace {
            initial: 'A',
            size: AvatarSize::Size22,
            tone: AvatarTone::Ink,
            shape: AvatarShape::Round,
        },
    }
}

fn has_all(html: &str, needles: &[&str]) {
    for needle in needles {
        assert!(html.contains(needle), "missing {needle}: {html}");
    }
}

#[test]
fn a_text_card_draws_each_line_its_runs_its_action_and_the_footer() {
    let html = render_card(AnswerView::Text(TextAnswer {
        body: vec![
            TextLine::Runs(vec![
                TextRun::new("Two", RunTone::Strong),
                TextRun::new(" threads need you.", RunTone::Plain),
            ]),
            "The rest can wait.".into(),
        ],
        streaming: true,
        actions: vec![action(
            "copy",
            "Copy",
            ActionRole::Secondary,
            EffectMark::Read,
        )],
        footer: footer(),
    }));
    has_all(
        &html,
        &[
            "data-kind=\"text\"",
            "data-tone=\"strong\"",
            "The rest can wait.",
        ],
    );
    has_all(
        &html,
        &[
            "data-streaming=\"arriving\"",
            ">Copy<",
            "ds-card-footer",
            "local, on this computer",
        ],
    );
}

#[test]
fn a_draft_card_draws_the_header_the_quote_the_editable_body_and_a_marked_send() {
    let html = render_card(AnswerView::DraftReply(DraftReply {
        to: vec![person()],
        subject: "Re: Lunch".to_owned(),
        quoted: Some("Are you free on Friday?".to_owned()),
        body: "Friday works.".to_owned(),
        actions: vec![
            action("send", "Send", ActionRole::Primary, EffectMark::Outbound),
            action(
                "save",
                "Save Draft",
                ActionRole::Secondary,
                EffectMark::UndoableWrite,
            ),
        ],
        footer: footer(),
    }));
    has_all(
        &html,
        &[
            "data-kind=\"draft-reply\"",
            "Ada Lovelace",
            "Re: Lunch",
            "Are you free on Friday?",
        ],
    );
    has_all(
        &html,
        &[
            "<textarea",
            "Friday works.",
            ">Send<",
            ">Save Draft<",
            "data-answers=\"return\"",
        ],
    );
    has_all(
        &html,
        &[
            "ds-effect-tag",
            "data-effect=\"outbound\"",
            "data-focus=\"keyboard\"",
        ],
    );
}

#[test]
fn an_event_card_draws_its_facts_and_a_slot_per_part_of_the_day() {
    let slot = |from, to, kind, label: &str| DaySlot {
        span: MinuteSpan { from, to },
        kind,
        label: label.to_owned(),
    };
    let html = render_card(AnswerView::ProposedEvent(ProposedEvent {
        title: "Lunch with Ada".to_owned(),
        when: "Friday at 12:30".to_owned(),
        place: Some("Cafe Lumen".to_owned()),
        people: vec![person()],
        day: vec![
            slot(540, 600, BusyKind::Existing, "Standup"),
            slot(750, 810, BusyKind::Proposed, "Lunch"),
        ],
        actions: vec![action(
            "add",
            "Add to Calendar",
            ActionRole::Primary,
            EffectMark::UndoableWrite,
        )],
        footer: footer(),
    }));
    has_all(
        &html,
        &[
            "data-kind=\"proposed-event\"",
            "Lunch with Ada",
            "Friday at 12:30",
            "Cafe Lumen",
        ],
    );
    has_all(
        &html,
        &[
            "data-kind=\"existing\"",
            "data-kind=\"proposed\"",
            "--from:",
            ">Add to Calendar<",
        ],
    );
}

#[test]
fn a_plan_card_draws_groups_steps_states_and_the_buttons_of_its_phase() {
    let step = |id, label: &str, state| PlanStep {
        id: StepId(id),
        label: label.to_owned(),
        detail: None,
        app: None,
        effect: EffectMark::Outbound,
        included: true,
        state,
    };
    let plan = |phase| {
        AnswerView::Plan(PlanView {
            title: "Forward the invoices".to_owned(),
            groups: vec![PlanGroup {
                label: "Send".to_owned(),
                effect: EffectMark::Outbound,
                steps: vec![
                    step(1, "Find the invoices", PlanStepState::Done),
                    step(2, "Mail them to Ada", PlanStepState::Pending),
                ],
            }],
            phase,
            footer: footer(),
        })
    };
    let draft = render_card(plan(PlanPhase::Draft));
    has_all(
        &draft,
        &[
            "data-kind=\"plan\"",
            "Forward the invoices",
            "Mail them to Ada",
            ">Run<",
            ">Edit<",
        ],
    );
    assert!(!draft.contains("Undo All"), "{draft}");
    let running = render_card(plan(PlanPhase::Running { at: StepId(2) }));
    has_all(&running, &[">Stop<", "data-state=\"done\""]);
}

#[test]
fn a_replace_card_draws_both_texts_and_waits_for_apply() {
    let html = render_card(AnswerView::Replace(ReplaceProposal {
        original: "teh".to_owned(),
        proposed: "the".to_owned(),
        phase: ReplacePhase::Proposed,
        footer: footer(),
    }));
    has_all(
        &html,
        &[
            "data-kind=\"replace\"",
            "data-side=\"original\"",
            ">teh<",
            ">the<",
            ">Apply<",
            ">Discard<",
        ],
    );
}

#[test]
fn a_form_card_draws_a_labelled_input_per_field_and_its_submit() {
    let field = |key: &str, label: &str, input, required| FormField {
        key: FieldKeyId(key.to_owned()),
        label: label.to_owned(),
        input,
        required,
        value: FormValue::Empty,
        validity: Validity::Valid,
    };
    let html = render_card(AnswerView::Form(CompactForm {
        title: "Who is it for?".to_owned(),
        fields: vec![
            field("to", "Recipient", FormInput::Text, true),
            field(
                "kind",
                "Kind",
                FormInput::Choice(vec![("a".to_owned(), "Invoice".to_owned())]),
                false,
            ),
        ],
        submit: action("go", "Continue", ActionRole::Primary, EffectMark::Read),
        footer: footer(),
    }));
    has_all(
        &html,
        &[
            "data-kind=\"form\"",
            "Who is it for?",
            "Recipient",
            "(optional)",
            ">Invoice<",
            ">Continue<",
            "class=\"ds-chip-press\" aria-pressed=\"false\"",
        ],
    );
}

#[test]
fn a_refused_card_says_why_in_plain_words_and_offers_its_action() {
    let html = render_card(AnswerView::Refused(RefusedAnswer {
        why: RefusedView::NeedsCloud {
            class: "mail".to_owned(),
        },
        actions: vec![action(
            "allow",
            "Allow Once",
            ActionRole::Primary,
            EffectMark::Outbound,
        )],
        footer: footer(),
    }));
    has_all(
        &html,
        &[
            "data-kind=\"refused\"",
            "data-why=\"needs-cloud\"",
            "This needs a cloud model",
            ">Allow Once<",
        ],
    );
}
