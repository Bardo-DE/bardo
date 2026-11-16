//! The window frame casement draws for a client that asked for a server-side one, read from
//! quire's tokens: the chrome lengths, the colour table, `light_state`'s reveal rule, the window
//! radius (`Radius::px`), the shadow (`Shadow::layers`), the three light hues (`light_hue`).
//! Pure; the renderer draws the [`FrameStyle`] this makes. Nothing here is a copy of a number
//! quire owns.

use compositor_core::window::{
    FrameColour, FrameLook, FrameMetrics, FrameStyle, Light, LightDraw, ShadowLayer,
};

use ui_style::appearance::theme::{Scheme, Theme};
use ui_style::tokens::chrome::light_hue::{Light as HueLight, light_hue};
use ui_style::tokens::chrome::{ChromeToken, GroupHover, LightFill, LightState, light_state};
use ui_style::tokens::colour::ColourToken;
use ui_style::tokens::elevation::Shadow;
use ui_style::tokens::hex::{Colour, Hex};
use ui_style::tokens::shape::Radius;

/// The frame's lengths from the chrome tokens.
#[must_use]
pub fn metrics() -> FrameMetrics {
    let of = |token: ChromeToken| u32::from(token.length().0);
    FrameMetrics::new(
        of(ChromeToken::TitlebarHeight),
        of(ChromeToken::LightSize),
        of(ChromeToken::LightGap),
        of(ChromeToken::LightInset),
        of(ChromeToken::Edge),
        of(ChromeToken::EdgeCorner),
    )
}

/// The frame as it looks in `scheme` for a window in `activity` with the pointer as `hover`
/// says.
#[must_use]
pub fn frame_style(scheme: Scheme, activity: bool, hover: GroupHover) -> FrameStyle {
    let state = light_state(activity, hover);
    let token = |token: ColourToken| colour(token.value(scheme));
    let ground = if activity {
        ColourToken::Paper
    } else {
        ColourToken::Surface2
    };
    FrameStyle {
        metrics: metrics(),
        look: FrameLook {
            radius: u32::from(Radius::Window.px()),
            titlebar: token(ground),
            rule: token(ColourToken::Line),
            lights: Light::ALL.map(|which| light(state, which, scheme)),
            shadow: shadow(scheme),
        },
    }
}

/// The scheme a person's `appearance.theme` paints before the desktop is asked.
///
/// `System` follows the desktop's preference, which `compositor_app::appearance` reads over the
/// settings portal; until it has answered (and on a desktop without a portal) it paints Light, the
/// scheme the desktop starts in.
///
/// ```
/// use compositor_layout::{Scheme, Theme, scheme_of};
///
/// assert_eq!(scheme_of(Theme::Dark), Scheme::Dark);
/// assert_eq!(scheme_of(Theme::System), Scheme::Light);
/// ```
#[must_use]
pub fn scheme_of(theme: Theme) -> Scheme {
    match theme {
        Theme::Dark => Scheme::Dark,
        _ => Scheme::Light,
    }
}

/// `style` (drawn in `scheme`) with `light` drawn pressed: its hue a shade darker, its mark
/// showing, as the Mac draws a light under the button.
///
/// ```
/// use compositor_core::window::Light;
/// use compositor_layout::{GroupHover, Scheme, frame_style, pressed};
///
/// let rest = frame_style(Scheme::Light, true, GroupHover::Away);
/// let down = pressed(rest, Light::Close, Scheme::Light);
/// assert!(down.look.light(Light::Close).mark.is_some());
/// assert_eq!(down.look.light(Light::Zoom), rest.look.light(Light::Zoom));
/// ```
#[must_use]
pub fn pressed(style: FrameStyle, light: Light, scheme: Scheme) -> FrameStyle {
    let (hue, ink) = light_hue(hue_light(light), scheme);
    let mut look = style.look;
    look.lights[slot(light)] = LightDraw {
        fill: darker(rgb(hue)),
        ring: darker(darker(rgb(hue))),
        mark: Some(rgb(ink)),
    };
    FrameStyle { look, ..style }
}

/// Where `light` is in a look's `lights`: left to right.
fn slot(light: Light) -> usize {
    match light {
        Light::Close => 0,
        Light::Minimise => 1,
        Light::Zoom => 2,
    }
}

/// quire's name for casement's `light`.
fn hue_light(light: Light) -> HueLight {
    match light {
        Light::Close => HueLight::Close,
        Light::Minimise => HueLight::Minimise,
        Light::Zoom => HueLight::Zoom,
    }
}

/// One light: its hue or the grey pill, and its mark while the group shows marks.
fn light(state: LightState, which: Light, scheme: Scheme) -> LightDraw {
    let (hue, ink) = light_hue(hue_light(which), scheme);
    let (fill, ring) = match state.fill {
        LightFill::Colour => (rgb(hue), darker(rgb(hue))),
        LightFill::Grey => (
            colour(ColourToken::Line.value(scheme)),
            colour(ColourToken::InkFaint.value(scheme)),
        ),
    };
    let mark = state.marks.then(|| rgb(ink));
    LightDraw { fill, ring, mark }
}

/// `--shadow-window`, from quire's numeric layers (contact shadow, then the wide one). A third
/// layer would not be drawn: `FrameLook::shadow` holds two, and a test pins quire's count.
fn shadow(scheme: Scheme) -> [ShadowLayer; 2] {
    let layers = Shadow::Window.layers(scheme);
    [0, 1].map(|at| {
        layers.get(at).map_or_else(
            || ShadowLayer {
                dx: 0,
                dy: 0,
                blur: 0,
                spread: 0,
                colour: FrameColour {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 0,
                },
            },
            |layer| {
                let [r, g, b, a] = layer.rgba();
                ShadowLayer {
                    dx: i32::from(layer.dx),
                    dy: i32::from(layer.dy),
                    blur: u32::from(layer.blur),
                    spread: i32::from(layer.spread),
                    colour: FrameColour { r, g, b, a },
                }
            },
        )
    })
}

/// A token's value as a straight-alpha colour.
pub(crate) fn colour(value: Colour) -> FrameColour {
    match value {
        Colour::Solid(hex) => FrameColour {
            r: hex.0[0],
            g: hex.0[1],
            b: hex.0[2],
            a: 255,
        },
        Colour::Alpha(hex, alpha) => FrameColour {
            r: hex.0[0],
            g: hex.0[1],
            b: hex.0[2],
            a: u8::try_from((u32::from(alpha.0.min(1000)) * 255 + 500) / 1000).unwrap_or(255),
        },
    }
}

/// An opaque colour from quire's hex.
fn rgb(value: Hex) -> FrameColour {
    FrameColour {
        r: value.0[0],
        g: value.0[1],
        b: value.0[2],
        a: 255,
    }
}

/// A hue's hairline: the hue at about four fifths.
fn darker(of: FrameColour) -> FrameColour {
    let shade = |channel: u8| (u16::from(channel) * 4 / 5) as u8;
    FrameColour {
        r: shade(of.r),
        g: shade(of.g),
        b: shade(of.b),
        a: of.a,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_metrics_are_the_chrome_tokens() {
        let m = metrics();
        assert_eq!(
            (m.titlebar, m.light, m.gap, m.inset, m.edge, m.corner),
            (28, 12, 8, 13, 4, 12)
        );
    }

    #[test]
    fn the_lights_follow_activity_and_hover() {
        const CASES: &[(&str, bool, GroupHover, bool, bool)] = &[
            ("key at rest", true, GroupHover::Away, true, false),
            ("key hovered", true, GroupHover::Over, true, true),
            ("inactive at rest", false, GroupHover::Away, false, false),
            ("inactive hovered", false, GroupHover::Over, true, true),
        ];
        for (name, activity, hover, coloured, marked) in CASES {
            let draw = frame_style(Scheme::Light, *activity, *hover);
            for light in Light::ALL {
                let look = draw.look.light(light);
                assert_eq!(look.mark.is_some(), *marked, "{name} mark");
            }
            let red = draw.look.light(Light::Close).fill;
            assert_eq!(
                red == rgb(light_hue(HueLight::Close, Scheme::Light).0),
                *coloured,
                "{name} fill"
            );
        }
    }

    #[test]
    fn a_pressed_light_is_darker_with_its_mark_and_the_others_stay() {
        let rest = frame_style(Scheme::Light, false, GroupHover::Away);
        for light in Light::ALL {
            let down = pressed(rest, light, Scheme::Light);
            let (was, now) = (rest.look.light(light), down.look.light(light));
            assert!(now.mark.is_some(), "{light:?} mark");
            assert_ne!(now.fill, was.fill, "{light:?} fill");
            for other in Light::ALL.into_iter().filter(|other| *other != light) {
                assert_eq!(down.look.light(other), rest.look.light(other), "{other:?}");
            }
        }
    }

    #[test]
    fn the_theme_picks_the_scheme() {
        const CASES: &[(&str, Theme, Scheme)] = &[
            ("dark", Theme::Dark, Scheme::Dark),
            ("light", Theme::Light, Scheme::Light),
            ("system is light", Theme::System, Scheme::Light),
        ];
        for (name, theme, want) in CASES {
            assert_eq!(scheme_of(*theme), *want, "{name}");
        }
    }

    #[test]
    fn a_dark_frame_has_a_dark_titlebar() {
        let light = frame_style(Scheme::Light, true, GroupHover::Away);
        let dark = frame_style(Scheme::Dark, true, GroupHover::Away);
        assert!(dark.look.titlebar.r < light.look.titlebar.r);
    }

    #[test]
    fn an_inactive_titlebar_is_lighter() {
        let key = frame_style(Scheme::Light, true, GroupHover::Away);
        let idle = frame_style(Scheme::Light, false, GroupHover::Away);
        assert!(idle.look.titlebar.r > key.look.titlebar.r);
    }

    const SCHEMES: [Scheme; 2] = [Scheme::Light, Scheme::Dark];

    #[test]
    fn the_radius_is_quires_window_radius() {
        for scheme in SCHEMES {
            let look = frame_style(scheme, true, GroupHover::Away).look;
            assert_eq!(look.radius, u32::from(Radius::Window.px()), "{scheme:?}");
        }
    }

    #[test]
    fn the_shadow_is_quires_window_layers_in_both_schemes() {
        for scheme in SCHEMES {
            let quire = Shadow::Window.layers(scheme);
            assert_eq!(quire.len(), 2, "{scheme:?}: FrameLook holds two layers");
            let look = frame_style(scheme, true, GroupHover::Away).look;
            for (ours, theirs) in look.shadow.iter().zip(quire) {
                let [r, g, b, a] = theirs.rgba();
                assert_eq!(
                    (ours.dx, ours.dy, ours.blur, ours.spread),
                    (
                        i32::from(theirs.dx),
                        i32::from(theirs.dy),
                        u32::from(theirs.blur),
                        i32::from(theirs.spread)
                    ),
                    "{scheme:?}"
                );
                assert_eq!(ours.colour, FrameColour { r, g, b, a }, "{scheme:?}");
            }
        }
    }

    #[test]
    fn the_lights_are_quires_hues_and_inks_in_both_schemes() {
        for scheme in SCHEMES {
            let marked = frame_style(scheme, true, GroupHover::Over).look;
            for light in Light::ALL {
                let (disc, ink) = light_hue(hue_light(light), scheme);
                let draw = marked.light(light);
                assert_eq!(draw.fill, rgb(disc), "{scheme:?} {light:?} disc");
                assert_eq!(draw.mark, Some(rgb(ink)), "{scheme:?} {light:?} mark");
                let rest = frame_style(scheme, true, GroupHover::Away);
                let held = pressed(rest, light, scheme).look.light(light);
                assert_eq!(held.mark, Some(rgb(ink)), "{scheme:?} {light:?} held mark");
                assert_eq!(
                    held.fill,
                    darker(rgb(disc)),
                    "{scheme:?} {light:?} held disc"
                );
            }
        }
    }

    #[test]
    fn the_lights_have_a_slot_each() {
        let slots: Vec<usize> = Light::ALL.iter().map(|light| slot(*light)).collect();
        assert_eq!(slots, [0, 1, 2]);
    }
}
