//! Computer use over the bus: a begin, then a step whose frame rides as a memfd.

use super::*;
use computer_use_action::{Button, ClickCount, ComputerUseAction, Target};
use computer_use_geometry::{Coord, DeviceSize, PixelFormat, Point, Scale120, Size, WindowSpace};
use inference_core::{
    AttachIndex, ComputerUseBegin, ComputerUseStepRequest, FrameImage, FrameLayout, ImageSource,
    StepIndex, WindowGeometry,
};
use std::io::Write;
use std::os::fd::OwnedFd;

fn memfd(bytes: &[u8]) -> OwnedFd {
    let fd = rustix::fs::memfd_create("frame", rustix::fs::MemfdFlags::CLOEXEC).expect("memfd");
    std::fs::File::from(fd.try_clone().expect("dup"))
        .write_all(bytes)
        .expect("fill");
    fd
}

/// A window of 200 by 100 logical pixels at scale 1, whose frame is a flat grey.
fn step_request(step: u32) -> ComputerUseStepRequest {
    ComputerUseStepRequest::new(
        StepIndex(step),
        WindowGeometry {
            logical: Size::new(Coord(200), Coord(100)),
            scale: Scale120(120),
        },
        FrameImage {
            source: ImageSource::Attached(AttachIndex(0)),
            layout: FrameLayout::Raw {
                format: PixelFormat::Argb8888,
                size: DeviceSize { w: 200, h: 100 },
                stride: 800,
            },
        },
        None,
    )
}

fn begin() -> ComputerUseBegin {
    ComputerUseBegin::new("press the big button".into(), ComputerUseEnv::Desktop)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_computer_use_step_with_a_memfd_frame_round_trips_to_a_window_space_action() {
    let world = World::start(computer_use_world(CallerRole::ComputerUse)).await;
    let mut session = world
        .accounts
        .session(&computer_use_need(), DataClass::Screen, Tier::Best)
        .await
        .expect("open");

    // The begin is acknowledged with an empty step reply.
    session
        .send(ClientFrame::Request(
            InferRequest::ComputerUseBegin(begin()),
        ))
        .await
        .expect("begin");
    let events = until_finished(&mut session).await;
    assert!(matches!(events[0], InferEvent::Why(_)), "{events:?}");
    assert_eq!(
        events[1],
        InferEvent::Waiting(inference_core::Readiness::Loadable)
    );
    assert!(matches!(events[2], InferEvent::Routed(_)), "{events:?}");
    assert!(
        matches!(
            events.last(),
            Some(InferEvent::Finished(InferReply::ComputerUseStep(reply))) if reply.actions.is_empty()
        ),
        "{events:?}"
    );

    // Then a step, the frame as the one attached descriptor.
    let frame = vec![0x80_u8; 200 * 100 * 4];
    session
        .send_attached(
            ClientFrame::Request(InferRequest::ComputerUseStep(step_request(0))),
            vec![memfd(&frame)],
        )
        .await
        .expect("step");
    let events = until_finished(&mut session).await;
    let click = ComputerUseAction::Click {
        at: Target::Point(Point::<WindowSpace>::new(Coord(100), Coord(50))),
        button: Button::Left,
        count: ClickCount::One,
        mods: std::collections::BTreeSet::default(),
    };
    assert!(
        events.contains(&InferEvent::ActionProposed(click.clone())),
        "{events:?}"
    );
    let Some(InferEvent::Finished(InferReply::ComputerUseStep(reply))) = events.last() else {
        panic!("a step reply, got {events:?}");
    };
    // 500 of 1000 on the grid is the middle of a 200 by 100 window.
    assert_eq!(reply.actions, vec![click]);
    assert_eq!(reply.dropped, vec![]);

    // The engine saw the goal, the tool and the frame as an inline PNG.
    let bodies = world.engines["tiny-computer-use"].bodies("/v1/chat/completions");
    assert_eq!(bodies.len(), 1);
    let sent = bodies[0].to_string();
    assert!(sent.contains("press the big button"), "the goal");
    assert!(sent.contains("computer_use"), "the tool");
    assert!(
        sent.contains("data:image/png;base64,"),
        "the frame as a PNG"
    );

    // The audit says a frame was sent for the step and none for the begin; never the frame.
    let images: Vec<_> = world.audit.entries().iter().map(|e| e.images).collect();
    assert_eq!(images, vec![base_ids::Count(0), base_ids::Count(1)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn only_computer_use_daemon_may_open_a_computer_use_session() {
    let world = World::start(computer_use_world(CallerRole::App)).await;
    let mut session = world
        .accounts
        .session(&computer_use_need(), DataClass::Screen, Tier::Best)
        .await
        .expect("open");
    let events = until_finished(&mut session).await;
    assert_eq!(
        events,
        vec![InferEvent::Finished(InferReply::Refused(
            inference_core::InferRefusedError::Denied
        ))]
    );
    assert!(world.host.spawned.lock().expect("lock").is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_step_before_a_begin_and_a_wrong_class_are_refused() {
    let world = World::start(computer_use_world(CallerRole::ComputerUse)).await;
    let mut early = world
        .accounts
        .session(&computer_use_need(), DataClass::Screen, Tier::Best)
        .await
        .expect("open");
    early
        .send_attached(
            ClientFrame::Request(InferRequest::ComputerUseStep(step_request(0))),
            vec![memfd(&[0; 16])],
        )
        .await
        .expect("send");
    let events = until_finished(&mut early).await;
    assert!(
        matches!(
            events.last(),
            Some(InferEvent::Finished(InferReply::Refused(
                inference_core::InferRefusedError::Unsupported
            )))
        ),
        "{events:?}"
    );

    // Anything but screen data on a computer-use session never reaches a model.
    let mut mail = world
        .accounts
        .session(&computer_use_need(), DataClass::Mail, Tier::Best)
        .await
        .expect("open");
    let events = until_finished(&mut mail).await;
    assert_eq!(
        events,
        vec![InferEvent::Finished(InferReply::Refused(
            inference_core::InferRefusedError::Unsupported
        ))]
    );
}

fn click_at_middle() -> Chat {
    Chat::Call {
        name: "computer_use",
        arguments: r#"{"action":"left_click","coordinate":[500,500]}"#.into(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_reply_with_no_call_is_repaired_and_the_next_step_sees_the_earlier_frame() {
    let mut plan = computer_use_world(CallerRole::ComputerUse);
    plan.scripts = vec![(
        "tiny-computer-use",
        Script {
            chat: vec![
                Chat::Say(vec!["I cannot see a button"]),
                click_at_middle(),
                click_at_middle(),
            ],
            dims: 0,
        },
    )];
    let world = World::start(plan).await;
    let mut session = world
        .accounts
        .session(&computer_use_need(), DataClass::Screen, Tier::Best)
        .await
        .expect("open");
    session
        .send(ClientFrame::Request(
            InferRequest::ComputerUseBegin(begin()),
        ))
        .await
        .expect("begin");
    until_finished(&mut session).await;

    let frame = vec![0x80_u8; 200 * 100 * 4];
    for step in 0..2 {
        session
            .send_attached(
                ClientFrame::Request(InferRequest::ComputerUseStep(step_request(step))),
                vec![memfd(&frame)],
            )
            .await
            .expect("step");
        let events = until_finished(&mut session).await;
        let Some(InferEvent::Finished(InferReply::ComputerUseStep(reply))) = events.last() else {
            panic!("a step reply, got {events:?}");
        };
        assert_eq!(reply.actions.len(), 1, "step {step}: {events:?}");
    }

    // The first step took two turns (the second named what was wrong), the second step one.
    let bodies = world.engines["tiny-computer-use"].bodies("/v1/chat/completions");
    assert_eq!(bodies.len(), 3);
    let repair = bodies[1].to_string();
    assert!(
        repair.contains("did not hold an action I can run"),
        "{repair}"
    );
    assert!(
        !repair.contains("I cannot see a button"),
        "the reply is not repeated"
    );
    let frames = |body: &Value| body.to_string().matches("data:image/png;base64,").count();
    assert_eq!(
        (frames(&bodies[0]), frames(&bodies[1]), frames(&bodies[2])),
        (1, 1, 2),
        "a repair carries the same frame; the next step also shows the one before"
    );
}
