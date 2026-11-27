//! Parsed actions into window space: a point that falls outside the frame is dropped, never
//! clamped.

use computer_use_action::ComputerUseAction;
use computer_use_geometry::{CoordSpace, Length, Point, WindowSpace};
use computer_use_parse::{DropReason, Dropped, InSpace, VerbText};
use model_vision_prep::{FrameMap, MapError};

/// The verb a dropped action is reported under.
fn verb_of<S: CoordSpace>(action: &ComputerUseAction<S>) -> &'static str {
    match action {
        ComputerUseAction::Click { .. } => "click",
        ComputerUseAction::MoveTo { .. } => "move",
        ComputerUseAction::Drag { .. } => "drag",
        ComputerUseAction::Type { .. } => "type",
        ComputerUseAction::Key { .. } => "key",
        ComputerUseAction::Scroll { .. } => "scroll",
        ComputerUseAction::Wait { .. } => "wait",
        ComputerUseAction::Zoom { .. } => "zoom",
        ComputerUseAction::Observe => "observe",
        ComputerUseAction::Finish { .. } => "finish",
        ComputerUseAction::Ask { .. } => "ask",
    }
}

fn map_all<S: CoordSpace>(
    actions: Vec<ComputerUseAction<S>>,
    point: impl Fn(Point<S>) -> Result<Point<WindowSpace>, MapError>,
    length: impl Fn(Length<S>) -> Result<Length<WindowSpace>, MapError>,
) -> (Vec<ComputerUseAction<WindowSpace>>, Vec<Dropped>) {
    let mut kept = Vec::new();
    let mut dropped = Vec::new();
    for action in actions {
        let verb = verb_of(&action);
        match action.map_points(&point, &length) {
            Ok(mapped) => kept.push(mapped),
            Err(_) => dropped.push(Dropped {
                verb: VerbText::new(verb).unwrap_or_else(|_| unreachable!("a verb name is short")),
                reason: DropReason::OutOfFrame,
            }),
        }
    }
    (kept, dropped)
}

/// The actions in window space, and what mapping dropped.
pub(crate) fn to_window(
    actions: InSpace,
    map: &FrameMap,
) -> (Vec<ComputerUseAction<WindowSpace>>, Vec<Dropped>) {
    match actions {
        InSpace::Image(list) => map_all(
            list,
            |p| map.image_to_window(p),
            |l| map.length_to_window(l),
        ),
        InSpace::Grid(_, list) => {
            map_all(list, |p| map.grid_to_window(p), |l| map.length_to_window(l))
        }
    }
}
