use super::*;

fn options(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

fn control(scores: pi::Knob<pi::ScoreOptions>) -> pi::ChatControl {
    pi::ChatControl::new()
        .with_reasoning(pi::Reasoning::Off)
        .with_scores(scores)
}

#[test]
fn only_a_choice_of_two_or_more_with_the_knob_set_asks_the_engine() {
    let two = sp::OutputShape::Choice(options(&["a", "b"]));
    let one = sp::OutputShape::Choice(options(&["a"]));
    let on = pi::Knob::Set(pi::ScoreOptions {
        top_k: account_ids::Count(5),
    });
    assert_eq!(
        choice_scores(&control(on), &two),
        sp::ChoiceScores::FirstToken {
            top_k: sp::Count(5)
        }
    );
    assert_eq!(
        choice_scores(&control(pi::Knob::Off), &two),
        sp::ChoiceScores::Off
    );
    assert_eq!(choice_scores(&control(on), &one), sp::ChoiceScores::Off);
    assert_eq!(
        choice_scores(&control(on), &sp::OutputShape::Free),
        sp::ChoiceScores::Off
    );
}

#[test]
fn the_number_of_candidates_asked_for_is_held_to_what_the_engines_take() {
    let two = sp::OutputShape::Choice(options(&["a", "b"]));
    let ask = |k| {
        choice_scores(
            &control(pi::Knob::Set(pi::ScoreOptions {
                top_k: account_ids::Count(k),
            })),
            &two,
        )
    };
    assert_eq!(
        ask(0),
        sp::ChoiceScores::FirstToken {
            top_k: sp::Count(1)
        }
    );
    assert_eq!(
        ask(500),
        sp::ChoiceScores::FirstToken {
            top_k: sp::Count(20)
        }
    );
}
