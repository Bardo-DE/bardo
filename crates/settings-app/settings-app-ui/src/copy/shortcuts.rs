//! The Shortcuts list's words: action names, group titles, who set a chord, what was wrong in the
//! files and the question before a chord is taken over.

use settings_app_model::external::{Action, StandardAction};
use settings_app_model::{
    KeymapLayer, Origin, ReplaceQuestion, ShortcutGroup, ShortcutProblem, ShortcutRow,
};

/// Names that read better than the action's slug spelled out.
const WORDED: &[(StandardAction, &str)] = &[
    (StandardAction::NextInputSource, "Next input language"),
    (
        StandardAction::MissionControl,
        "Show all windows and Spaces",
    ),
    (StandardAction::AppWindows, "Show the app's windows"),
    (StandardAction::SpaceLeft, "Move left one Space"),
    (StandardAction::SpaceRight, "Move right one Space"),
    (StandardAction::ScreenshotScreen, "Capture the screen"),
    (StandardAction::ScreenshotSelection, "Capture a selection"),
    (StandardAction::TileLeft, "Tile to the left half"),
    (StandardAction::TileRight, "Tile to the right half"),
    (StandardAction::TileTop, "Tile to the top half"),
    (StandardAction::TileBottom, "Tile to the bottom half"),
    (StandardAction::TileTopLeft, "Tile to the top left quarter"),
    (
        StandardAction::TileTopRight,
        "Tile to the top right quarter",
    ),
    (
        StandardAction::TileBottomLeft,
        "Tile to the bottom left quarter",
    ),
    (
        StandardAction::TileBottomRight,
        "Tile to the bottom right quarter",
    ),
    (StandardAction::Fill, "Fill the screen"),
    (StandardAction::Center, "Center the window"),
    (
        StandardAction::ReturnToPreviousSize,
        "Return to previous size",
    ),
    (StandardAction::LockScreen, "Lock the screen"),
    (StandardAction::Bigger, "Make bigger"),
    (StandardAction::Smaller, "Make smaller"),
    (StandardAction::ActualSize, "Actual size"),
    (StandardAction::ToggleToolbar, "Show or hide the toolbar"),
    (StandardAction::ToggleSidebar, "Show or hide the sidebar"),
    (StandardAction::ToggleDock, "Show or hide the dock"),
];

/// What the answer that takes a chord over is called.
pub(crate) const REPLACE: &str = "Replace";

fn capitalised(text: &str) -> String {
    let mut letters = text.chars();
    letters
        .next()
        .map(|first| first.to_uppercase().chain(letters).collect())
        .unwrap_or_default()
}

fn standard_name(action: StandardAction) -> String {
    if let StandardAction::SwitchToSpace(number) = action {
        return format!("Switch to Space {}", number.get());
    }
    WORDED
        .iter()
        .find(|(known, _)| *known == action)
        .map_or_else(
            || capitalised(&action.label()),
            |(_, name)| (*name).to_owned(),
        )
}

/// The name of `action` in plain words: "Select all", "Switch to Space 3"; an app's action by
/// its id.
pub(crate) fn action_name(action: &Action) -> String {
    match action {
        Action::Standard(standard) => standard_name(*standard),
        other => other.id(),
    }
}

/// The name of a standard action in plain words.
pub(crate) fn standard_action_name(action: StandardAction) -> String {
    action_name(&Action::Standard(action))
}

/// The group's title.
pub(crate) fn group_title(group: ShortcutGroup) -> &'static str {
    match group {
        ShortcutGroup::Launcher => "Launcher",
        ShortcutGroup::Windows => "Apps and windows",
        ShortcutGroup::Tiling => "Window tiling",
        ShortcutGroup::Spaces => "Spaces",
        ShortcutGroup::Screenshots => "Screenshots",
        ShortcutGroup::Input => "Input",
        ShortcutGroup::Text => "Text",
        ShortcutGroup::Documents => "Documents and views",
        ShortcutGroup::System => "System",
    }
}

/// How a row says who set its chord.
fn origin_label(origin: Origin) -> &'static str {
    match origin {
        Origin::Standard => "Standard",
        Origin::ThisComputer => "This computer",
        Origin::You => "You",
    }
}

/// The line under the name: who set the chord, and the other chords the action has.
pub(crate) fn shortcut_note(row: &ShortcutRow) -> String {
    match row.also.as_slice() {
        [] => origin_label(row.origin).to_owned(),
        also => format!("{}. Also {}.", origin_label(row.origin), also.join(", ")),
    }
}

/// A line of the files that could not be used, with whose file it is in.
pub(crate) fn problem_line(problem: &ShortcutProblem) -> String {
    let label = match problem.layer {
        KeymapLayer::Machine => "This computer's shortcuts",
        KeymapLayer::Person => "Your shortcuts",
    };
    format!("{label}: {}", problem.what)
}

/// The title and the message of the question asked before a chord is taken over.
pub(crate) fn replace_words(question: &ReplaceQuestion) -> (String, String) {
    let holder = action_name(&question.holder);
    let wanted = standard_action_name(question.wanted);
    (
        format!(
            "{} is already used by \u{201c}{holder}\u{201d}.",
            question.chord
        ),
        format!(
            "Use it for \u{201c}{wanted}\u{201d} instead? \u{201c}{holder}\u{201d} will lose it."
        ),
    )
}

/// What the person reads when their file could not be read, so nothing was written to it.
pub(crate) fn file_left_alone(trouble: &str) -> String {
    format!("Your shortcuts file was left alone. {trouble}")
}

/// What the person reads when the desktop keeps a chord for itself.
pub(crate) fn chord_reserved(chord: &str, reason: &str) -> String {
    format!("{chord} can't be used: {reason}")
}

/// The words "Reset all shortcuts to standard".
pub(crate) const RESET_ALL_SHORTCUTS_TO_STANDARD: &str = "Reset all shortcuts to standard";

/// The words "Reset All".
pub(crate) const RESET_ALL: &str = "Reset All";

/// The words "Your shortcuts could not be saved. Try again in a moment.".
pub(crate) const SHORTCUTS_NOT_SAVED: &str =
    "Your shortcuts could not be saved. Try again in a moment.";

/// The words "Your shortcuts were changed by another program just now, so nothing was saved. Try again.".
pub(crate) const SHORTCUTS_CHANGED_ELSEWHERE: &str =
    "Your shortcuts were changed by another program just now, so nothing was saved. Try again.";

/// The words "Reset {name}".
pub(crate) fn reset(name: impl std::fmt::Display) -> String {
    format!("Reset {name}")
}

/// The words "Reset".
pub(crate) const RESET: &str = "Reset";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_problem_says_whose_file_it_is_in() {
        let problem = |layer| ShortcutProblem {
            layer,
            what: "It is not plain text.".to_owned(),
        };
        assert_eq!(
            problem_line(&problem(KeymapLayer::Machine)),
            "This computer's shortcuts: It is not plain text."
        );
        assert_eq!(
            problem_line(&problem(KeymapLayer::Person)),
            "Your shortcuts: It is not plain text."
        );
    }
}
