//! The rows of a computer-use run: a step needs a begin.

use super::helpers::*;
use account_core::DataClass;
use inference_core::{ClientFrame, InferReply, RequestKind};

pub(super) fn rows() -> Vec<Row> {
    vec![
        (
            "a computer-use step before a begin is refused",
            computer_use_spec(DataClass::Screen),
            idle(RoutedNote::Sent),
            request(computer_use_step()),
            idle(RoutedNote::Sent),
            vec![unsupported()],
        ),
        (
            "a begin that is acknowledged begins the run",
            computer_use_spec(DataClass::Screen),
            in_turn(RequestKind::ComputerUseBegin, AudioCursor::NoAudio),
            SessionIn::TurnDone(begun_ack()),
            idle_computer_use(RoutedNote::Sent, ComputerUseProgress::Begun),
            vec![SessionOut::Audit(begun_ack()), done(begun_ack())],
        ),
        (
            "a begin that fails leaves the run unbegun",
            computer_use_spec(DataClass::Screen),
            in_turn(RequestKind::ComputerUseBegin, AudioCursor::NoAudio),
            SessionIn::TurnDone(lost_reply()),
            idle(RoutedNote::Sent),
            vec![SessionOut::Audit(lost_reply()), done(lost_reply())],
        ),
        (
            "a begin that is cancelled leaves the run unbegun",
            computer_use_spec(DataClass::Screen),
            in_turn(RequestKind::ComputerUseBegin, AudioCursor::NoAudio),
            frame(ClientFrame::Cancel),
            idle(RoutedNote::Sent),
            vec![
                SessionOut::DropTurn,
                SessionOut::Audit(InferReply::Cancelled),
                done(InferReply::Cancelled),
            ],
        ),
        (
            "a step after a begin starts",
            computer_use_spec(DataClass::Screen),
            idle_computer_use(RoutedNote::Sent, ComputerUseProgress::Begun),
            request(computer_use_step()),
            in_turn_with(
                RequestKind::ComputerUseStep,
                AudioCursor::NoAudio,
                ComputerUseProgress::Begun,
                None,
            ),
            vec![SessionOut::StartTurn(computer_use_step())],
        ),
        (
            "a step keeps the run begun, even when it fails",
            computer_use_spec(DataClass::Screen),
            in_turn_with(
                RequestKind::ComputerUseStep,
                AudioCursor::NoAudio,
                ComputerUseProgress::Begun,
                None,
            ),
            SessionIn::TurnDone(lost_reply()),
            idle_computer_use(RoutedNote::Sent, ComputerUseProgress::Begun),
            vec![SessionOut::Audit(lost_reply()), done(lost_reply())],
        ),
        (
            "a second begin starts another run",
            computer_use_spec(DataClass::Screen),
            idle_computer_use(RoutedNote::Sent, ComputerUseProgress::Begun),
            request(computer_use_begin()),
            in_turn_with(
                RequestKind::ComputerUseBegin,
                AudioCursor::NoAudio,
                ComputerUseProgress::Begun,
                None,
            ),
            vec![SessionOut::StartTurn(computer_use_begin())],
        ),
        (
            "a step queued behind a begin that fails is refused when it comes up",
            computer_use_spec(DataClass::Screen),
            in_turn_with(
                RequestKind::ComputerUseBegin,
                AudioCursor::NoAudio,
                ComputerUseProgress::NotBegun,
                Some(computer_use_step()),
            ),
            SessionIn::TurnDone(lost_reply()),
            idle(RoutedNote::Sent),
            vec![
                SessionOut::Audit(lost_reply()),
                done(lost_reply()),
                unsupported(),
            ],
        ),
    ]
}
