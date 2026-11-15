//! What the traffic lights show (design/04-COMPONENTS.md "Window frame"), as a pure function of
//! two facts: whether the window is the one the person works in, and whether the pointer is over
//! the group. The window frame's stylesheet draws it; a compositor that draws the lights itself
//! asks here.

/// Whether the pointer is over any of the three lights.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GroupHover {
    /// Over the group: any light under the pointer colours all three.
    Over,
    /// Elsewhere.
    #[default]
    Away,
}

/// How a light's disc is filled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LightFill {
    /// Its own colour: red, amber or green.
    Colour,
    /// The frame's pill grey with a hairline.
    Grey,
}

/// What one light shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LightState {
    /// The disc.
    pub fill: LightFill,
    /// Whether the mark (x, minus, arrows) is on the disc.
    pub marks: bool,
}

/// The lights in the window `activity` describes, with the pointer as `hover` says: coloured in
/// the key window and whenever the pointer is over the group, grey otherwise; the marks show only
/// over the group.
pub fn light_state(activity: bool, hover: GroupHover) -> LightState {
    let fill = match (activity, hover) {
        (true, _) | (false, GroupHover::Over) => LightFill::Colour,
        (false, GroupHover::Away) => LightFill::Grey,
    };
    let marks = match hover {
        GroupHover::Over => true,
        GroupHover::Away => false,
    };
    LightState { fill, marks }
}

#[cfg(test)]
mod tests {
    use super::{GroupHover, LightFill, LightState, light_state};

    const CASES: &[(&str, bool, GroupHover, LightFill, bool)] = &[
        (
            "key window at rest",
            true,
            GroupHover::Away,
            LightFill::Colour,
            false,
        ),
        (
            "key window under the pointer",
            true,
            GroupHover::Over,
            LightFill::Colour,
            true,
        ),
        (
            "inactive window at rest",
            false,
            GroupHover::Away,
            LightFill::Grey,
            false,
        ),
        (
            "inactive window under the pointer",
            false,
            GroupHover::Over,
            LightFill::Colour,
            true,
        ),
    ];

    #[test]
    fn the_lights_follow_activity_and_group_hover() {
        for (name, activity, hover, fill, marks) in CASES {
            assert_eq!(
                light_state(*activity, *hover),
                LightState {
                    fill: *fill,
                    marks: *marks
                },
                "{name}"
            );
        }
    }
}
