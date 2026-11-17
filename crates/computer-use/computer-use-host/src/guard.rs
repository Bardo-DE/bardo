//! The checks every control call passes before it reaches a run.

use computer_use_bus::{ComputerUseCaller, ComputerUseMember, ControlRefusal, permits};

/// `Ok` when `caller` may call `member` now: it is on the allow-list for the member (`NotPermitted`
/// otherwise), and computer use is on for `Start` (`Disabled` otherwise).
///
/// The caller check comes first, so a stranger learns nothing about the setting.
///
/// # Errors
///
/// Returns [`ControlRefusal::NotPermitted`] when `caller` may not call `member`, and
/// [`ControlRefusal::Disabled`] when `member` is `Start` and computer use is off.
pub fn guard(
    caller: ComputerUseCaller,
    member: ComputerUseMember,
    enabled: bool,
) -> Result<(), ControlRefusal> {
    if !permits(caller, member) {
        Err(ControlRefusal::NotPermitted)
    } else if member == ComputerUseMember::Start && !enabled {
        Err(ControlRefusal::Disabled)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_refused_for_non_planner_caller_and_while_disabled() {
        struct Row {
            name: &'static str,
            caller: ComputerUseCaller,
            member: ComputerUseMember,
            enabled: bool,
            want: Result<(), ControlRefusal>,
        }
        let rows = [
            Row {
                name: "router starts",
                caller: ComputerUseCaller::Router,
                member: ComputerUseMember::Start,
                enabled: true,
                want: Ok(()),
            },
            Row {
                name: "planner starts",
                caller: ComputerUseCaller::Companion,
                member: ComputerUseMember::Start,
                enabled: true,
                want: Ok(()),
            },
            Row {
                name: "shell may not start",
                caller: ComputerUseCaller::Shell,
                member: ComputerUseMember::Start,
                enabled: true,
                want: Err(ControlRefusal::NotPermitted),
            },
            Row {
                name: "a stranger may not start",
                caller: ComputerUseCaller::Other,
                member: ComputerUseMember::Start,
                enabled: true,
                want: Err(ControlRefusal::NotPermitted),
            },
            Row {
                name: "off refuses the router",
                caller: ComputerUseCaller::Router,
                member: ComputerUseMember::Start,
                enabled: false,
                want: Err(ControlRefusal::Disabled),
            },
            Row {
                name: "off tells a stranger nothing",
                caller: ComputerUseCaller::Other,
                member: ComputerUseMember::Start,
                enabled: false,
                want: Err(ControlRefusal::NotPermitted),
            },
            Row {
                name: "stop-all works while off",
                caller: ComputerUseCaller::Shell,
                member: ComputerUseMember::StopAll,
                enabled: false,
                want: Ok(()),
            },
            Row {
                name: "only the shell answers",
                caller: ComputerUseCaller::Companion,
                member: ComputerUseMember::Answer,
                enabled: true,
                want: Err(ControlRefusal::NotPermitted),
            },
        ];
        for row in rows {
            assert_eq!(
                guard(row.caller, row.member, row.enabled),
                row.want,
                "{}",
                row.name
            );
        }
    }
}
