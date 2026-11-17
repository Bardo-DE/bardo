//! What the mouse gestures daemon asks of the session. The mouse gestures daemon is a system service with no session of its own, so
//! a gesture whose action needs one (the workspace steps, a line of the shell's own commands, a
//! program of the person's) arrives here as `HostEvent::MouseGestures` and the shell carries it out as
//! the person: the workspace steps and shell lines are requests, answered as `bardo-shell workspace next`
//! and the like are, and a program is started as a child of the shell. The decision is pure
//! ([`plan`]); only [`run`] has an effect.

use std::process::Stdio;

use layer_shell_host::{MouseGesturesAction, MouseGesturesWorkspace};
use shell_ipc::request::Request;

use crate::cli::request_of_words;

/// What an ask comes to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Plan {
    /// A request, carried out as a client's would be.
    Request(Request),
    /// A program of the person's, started without a shell.
    Program { program: String, args: Vec<String> },
    /// Not something the shell does for mouse_gestures.
    Refused(&'static str),
}

/// What `ask` comes to.
pub fn plan(ask: &MouseGesturesAction) -> Plan {
    match ask {
        MouseGesturesAction::Workspace(MouseGesturesWorkspace::Prev) => {
            line(&["workspace", "prev"])
        }
        MouseGesturesAction::Workspace(MouseGesturesWorkspace::Next) => {
            line(&["workspace", "next"])
        }
        MouseGesturesAction::Workspace(MouseGesturesWorkspace::Overview) => {
            line(&["workspace", "overview"])
        }
        MouseGesturesAction::Shell { words } => line(words),
        MouseGesturesAction::Command { argv } => match argv.split_first() {
            Some((program, args)) => Plan::Program {
                program: program.clone(),
                args: args.to_vec(),
            },
            None => Plan::Refused("a program with no name"),
        },
    }
}

/// The request a line of the shell's command language sends. Stopping the shell and the
/// development probes are not something a gesture does.
fn line<S: AsRef<str>>(words: &[S]) -> Plan {
    match request_of_words(words) {
        Some(Request::Quit | Request::Debug(_)) => Plan::Refused("a gesture does not do that"),
        Some(request) => Plan::Request(request),
        None => Plan::Refused("not a command of the shell"),
    }
}

/// Starts `program` with `args` as a child of the shell, and waits for it on a thread of its own,
/// so a finished program is not left a zombie.
pub fn run(program: &str, args: &[String]) {
    match std::process::Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .spawn()
    {
        Ok(mut child) => {
            let reaped = std::thread::Builder::new()
                .name("shell-mouse-gestures-reaper".to_owned())
                .spawn(move || {
                    let _ = child.wait();
                });
            if let Err(error) = reaped {
                eprintln!("bardo-shell: mouse_gestures: could not wait for {program}: {error}");
            }
        }
        Err(error) => eprintln!("bardo-shell: mouse_gestures: running {program}: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shell_launcher::actions::ActionId;
    use shell_model::actions::Ask;

    fn words(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn each_ask_comes_to_what_the_command_line_would_send() {
        let step = |action| Plan::Request(Request::Do(Ask::of(action)));
        let cases = [
            (
                "previous workspace",
                MouseGesturesAction::Workspace(MouseGesturesWorkspace::Prev),
                step(ActionId::PreviousSpace),
            ),
            (
                "next workspace",
                MouseGesturesAction::Workspace(MouseGesturesWorkspace::Next),
                step(ActionId::NextSpace),
            ),
            (
                "the overview",
                MouseGesturesAction::Workspace(MouseGesturesWorkspace::Overview),
                step(ActionId::MissionControl),
            ),
            (
                "a line of the shell's commands",
                MouseGesturesAction::Shell {
                    words: words("workspace next"),
                },
                step(ActionId::NextSpace),
            ),
            (
                "a program",
                MouseGesturesAction::Command {
                    argv: words("notify-send hi"),
                },
                Plan::Program {
                    program: "notify-send".into(),
                    args: vec!["hi".into()],
                },
            ),
        ];
        for (name, ask, want) in cases {
            assert_eq!(plan(&ask), want, "{name}");
        }
    }

    #[test]
    fn what_a_gesture_may_not_do_or_is_not_a_command_is_refused() {
        let cases = [
            (
                "stopping the shell",
                MouseGesturesAction::Shell {
                    words: words("quit"),
                },
            ),
            (
                "not a command",
                MouseGesturesAction::Shell {
                    words: words("no such verb"),
                },
            ),
            (
                "the shell's own start",
                MouseGesturesAction::Shell {
                    words: words("run"),
                },
            ),
            (
                "a help request",
                MouseGesturesAction::Shell {
                    words: words("--help"),
                },
            ),
            ("no program", MouseGesturesAction::Command { argv: vec![] }),
        ];
        for (name, ask) in cases {
            assert!(
                matches!(plan(&ask), Plan::Refused(_)),
                "{name}: {:?}",
                plan(&ask)
            );
        }
    }
}
