//! Who may call what on `ComputerUse1`, and how a call is refused.

use serde::{Deserialize, Serialize};

/// Why a control call is refused. Each maps 1:1 to an error name
/// `org.bardo.ComputerUse1.Error.<Variant>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlRefusal {
    /// The caller is not on the allow-list for this member.
    NotPermitted,
    /// The run is not in a state where this applies.
    WrongState,
    /// No such question.
    UnknownQuestion,
    /// `computer_use.enabled` is off.
    Disabled,
    /// No such window.
    WindowUnknown,
    /// Another run holds the window.
    LeaseHeld,
    /// The backend cannot run this mode.
    ModeUnsupported,
    /// The task body is malformed or in a vocabulary this build does not read.
    BadTask,
    /// No computer-use model can serve a run: the chosen one is not installed or has no engine.
    ModelUnavailable,
}

impl ControlRefusal {
    /// Every refusal.
    pub const ALL: [ControlRefusal; 9] = [
        ControlRefusal::NotPermitted,
        ControlRefusal::WrongState,
        ControlRefusal::UnknownQuestion,
        ControlRefusal::Disabled,
        ControlRefusal::WindowUnknown,
        ControlRefusal::LeaseHeld,
        ControlRefusal::ModeUnsupported,
        ControlRefusal::BadTask,
        ControlRefusal::ModelUnavailable,
    ];
}

/// Who is calling, derived by the computer-use daemon from the connection (never sent in a body).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputerUseCaller {
    /// intentd: the router, acting for a `computeruse.run.start` call.
    Router,
    /// companiond: the planner.
    Companion,
    /// The shell: the launcher, the orb and the control centre.
    Shell,
    /// Anything else on the bus.
    Other,
}

/// A member of `ComputerUse1.Manager` or `ComputerUse1.Run` a caller may try. Reading a property is not a
/// member: properties and signals are open to any caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputerUseMember {
    /// `Manager.Start`.
    Start,
    /// `Manager.Runs`.
    Runs,
    /// `Manager.StopAll`.
    StopAll,
    /// `Run.Cancel`.
    Cancel,
    /// `Run.Pause`.
    Pause,
    /// `Run.Resume`.
    Resume,
    /// `Run.Answer`.
    Answer,
    /// `Run.TakeOver`.
    TakeOver,
    /// `Run.HandBack`.
    HandBack,
}

impl ComputerUseMember {
    /// Every member.
    pub const ALL: [ComputerUseMember; 9] = [
        ComputerUseMember::Start,
        ComputerUseMember::Runs,
        ComputerUseMember::StopAll,
        ComputerUseMember::Cancel,
        ComputerUseMember::Pause,
        ComputerUseMember::Resume,
        ComputerUseMember::Answer,
        ComputerUseMember::TakeOver,
        ComputerUseMember::HandBack,
    ];
}

/// Whether `caller` may call `member`.
///
/// `Start` is the router's and the planner's; stopping, pausing, resuming and taking over are the
/// router's, the planner's and the shell's; `Answer` is the shell's alone (a question is for the
/// person); `Runs` is open. A confirmation never comes through here: it is the router's, answered
/// through `Confirm1`.
#[must_use]
pub fn permits(caller: ComputerUseCaller, member: ComputerUseMember) -> bool {
    use ComputerUseCaller::{Companion, Router, Shell};
    match member {
        ComputerUseMember::Runs => true,
        ComputerUseMember::Start => matches!(caller, Router | Companion),
        ComputerUseMember::StopAll
        | ComputerUseMember::Cancel
        | ComputerUseMember::Pause
        | ComputerUseMember::Resume
        | ComputerUseMember::TakeOver
        | ComputerUseMember::HandBack => matches!(caller, Router | Companion | Shell),
        ComputerUseMember::Answer => matches!(caller, Shell),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn who_may_call_what() {
        use ComputerUseCaller::{Companion, Other, Router, Shell};
        use ComputerUseMember::*;
        struct Row {
            member: ComputerUseMember,
            allowed: &'static [ComputerUseCaller],
        }
        let table = [
            Row {
                member: Start,
                allowed: &[Router, Companion],
            },
            Row {
                member: Runs,
                allowed: &[Router, Companion, Shell, Other],
            },
            Row {
                member: StopAll,
                allowed: &[Router, Companion, Shell],
            },
            Row {
                member: Cancel,
                allowed: &[Router, Companion, Shell],
            },
            Row {
                member: Pause,
                allowed: &[Router, Companion, Shell],
            },
            Row {
                member: Resume,
                allowed: &[Router, Companion, Shell],
            },
            Row {
                member: Answer,
                allowed: &[Shell],
            },
            Row {
                member: TakeOver,
                allowed: &[Router, Companion, Shell],
            },
            Row {
                member: HandBack,
                allowed: &[Router, Companion, Shell],
            },
        ];
        assert_eq!(table.len(), ComputerUseMember::ALL.len());
        for row in &table {
            for caller in [Router, Companion, Shell, Other] {
                assert_eq!(
                    permits(caller, row.member),
                    row.allowed.contains(&caller),
                    "{caller:?} {:?}",
                    row.member
                );
            }
        }
    }

    #[test]
    fn only_the_shell_answers_a_question_and_nobody_unknown_controls_anything() {
        for caller in [
            ComputerUseCaller::Router,
            ComputerUseCaller::Companion,
            ComputerUseCaller::Other,
        ] {
            assert!(!permits(caller, ComputerUseMember::Answer), "{caller:?}");
        }
        for member in ComputerUseMember::ALL {
            if member != ComputerUseMember::Runs {
                assert!(!permits(ComputerUseCaller::Other, member), "{member:?}");
            }
        }
    }

    #[test]
    fn the_all_lists_cover_every_variant() {
        // A wildcard-free match: adding a variant breaks this until ALL is updated.
        for refusal in ControlRefusal::ALL {
            let _count = match refusal {
                ControlRefusal::NotPermitted
                | ControlRefusal::WrongState
                | ControlRefusal::UnknownQuestion
                | ControlRefusal::Disabled
                | ControlRefusal::WindowUnknown
                | ControlRefusal::LeaseHeld
                | ControlRefusal::ModeUnsupported
                | ControlRefusal::BadTask
                | ControlRefusal::ModelUnavailable => 1,
            };
        }
        for member in ComputerUseMember::ALL {
            let _count = match member {
                ComputerUseMember::Start
                | ComputerUseMember::Runs
                | ComputerUseMember::StopAll
                | ComputerUseMember::Cancel
                | ComputerUseMember::Pause
                | ComputerUseMember::Resume
                | ComputerUseMember::Answer
                | ComputerUseMember::TakeOver
                | ComputerUseMember::HandBack => 1,
            };
        }
    }
}
