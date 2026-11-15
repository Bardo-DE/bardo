//! The modes to try when a display refuses the one asked for: the mode it showed before, then
//! the one it prefers, then the smallest it lists.
//!
//! A mode a monitor lists can still be refused by the kernel (the cable cannot carry it, a
//! dock's link is short of bandwidth). Without a way back the output stays dark.

use casement_core::output::Mode;

/// The modes to try for a display that lists `listed`, in order and each once (by size and
/// refresh rate): `wanted`, then `previous` (what it was showing), then the preferred mode,
/// then the smallest listed mode, which is the likeliest to fit any link. A mode in `avoid`
/// (one that was lit and then showed nothing) is left out unless nothing else is left.
///
/// ```
/// use casement_backend::tty::mode_ladder;
/// use casement_core::geometry::PhysicalSize;
/// use casement_core::output::Mode;
/// use casement_core::time::Millihertz;
///
/// let mode = |w, h, preferred| Mode {
///     size: PhysicalSize { w, h },
///     refresh: Millihertz(60_000),
///     preferred,
/// };
/// let big = mode(3840, 2160, true);
/// let small = mode(1024, 768, false);
/// let ladder = mode_ladder(big, None, &[big, small], None);
/// assert_eq!(ladder, [big, small]);
/// assert_eq!(mode_ladder(big, None, &[big, small], Some(big)), [small]);
/// ```
pub fn mode_ladder(
    wanted: Mode,
    previous: Option<Mode>,
    listed: &[Mode],
    avoid: Option<Mode>,
) -> Vec<Mode> {
    let preferred = listed
        .iter()
        .find(|mode| mode.preferred)
        .copied();
    let smallest = listed
        .iter()
        .min_by_key(|mode| {
            (
                u64::from(mode.size.w) * u64::from(mode.size.h),
                mode.refresh,
            )
        })
        .copied();
    let same = |a: &Mode, b: &Mode| a.size == b.size && a.refresh == b.refresh;
    let ladder = std::iter::once(wanted)
        .chain(previous)
        .chain(preferred)
        .chain(smallest)
        .fold(Vec::new(), |mut ladder, mode| {
            if !ladder.iter().any(|seen| same(seen, &mode)) {
                ladder.push(mode);
            }
            ladder
        });
    let kept: Vec<Mode> = ladder
        .iter()
        .filter(|mode| !avoid.is_some_and(|avoided| same(&avoided, mode)))
        .copied()
        .collect();
    match kept.is_empty() {
        true => ladder,
        false => kept,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::geometry::PhysicalSize;
    use casement_core::time::Millihertz;

    fn mode(w: u32, h: u32, hz: u32, preferred: bool) -> Mode {
        Mode {
            size: PhysicalSize { w, h },
            refresh: Millihertz(hz * 1000),
            preferred,
        }
    }

    #[test]
    fn the_ladder_goes_wanted_previous_preferred_smallest_each_once() {
        let native = mode(3840, 2160, 60, true);
        let fast = mode(3840, 2160, 144, false);
        let hd = mode(1920, 1080, 60, false);
        let vga = mode(640, 480, 60, false);
        let listed = [native, fast, hd, vga];
        // name, wanted, previous, ladder
        let cases = [
            ("a fresh output", fast, None, vec![fast, native, vga]),
            (
                "it was showing hd",
                fast,
                Some(hd),
                vec![fast, hd, native, vga],
            ),
            (
                "wanted is the preferred one",
                native,
                Some(native),
                vec![native, vga],
            ),
            ("wanted is the smallest", vga, None, vec![vga, native]),
        ];
        for (name, wanted, previous, want) in cases {
            assert_eq!(mode_ladder(wanted, previous, &listed, None), want, "{name}");
        }
    }

    #[test]
    fn a_mode_that_showed_nothing_is_left_out_unless_it_is_all_there_is() {
        let only = mode(1024, 768, 60, true);
        let big = mode(2560, 1440, 60, false);
        assert_eq!(mode_ladder(big, None, &[only, big], Some(big)), [only]);
        assert_eq!(mode_ladder(only, None, &[only], Some(only)), [only]);
    }

    #[test]
    fn a_display_that_lists_no_mode_gets_the_one_asked_for() {
        let wanted = mode(800, 600, 60, false);
        assert_eq!(mode_ladder(wanted, None, &[], None), [wanted]);
    }
}
