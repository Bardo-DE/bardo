//! When an animation has finished, computed rather than observed: Blitz dispatches no
//! `animationend` (design/05-MOTION.md section 7.1).

use super::anim::Anim;
use base_machine::time::FRAME_SLACK;
use std::time::Duration;
use ui_style::appearance::motion::MotionLevel;

/// `duration(anim, level) + FRAME_SLACK`. Worked values, Standard: `settle(RowOut)` 184 ms,
/// `settle(Heal)` 284 ms; Reduced: every settle is 184 ms.
#[must_use]
pub fn settle(anim: Anim, level: MotionLevel) -> Duration {
    anim.recipe().duration.duration(level) + FRAME_SLACK
}
