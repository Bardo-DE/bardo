//! Reading what a surface really paints: the contrast of an element's ink against the ground
//! behind it, off the rendered pixels. Computed colours cannot see a glyph the renderer kept in
//! a stale colour (a dark glyph on a dark bar after the scheme changed), so a legibility test
//! asks the picture.

use std::collections::HashMap;

use std::time::Duration;

use ds::prelude::Theme;
use ds_harness::{Backdrop, Harness, Srgba};
use ds_settings::Environment;

use super::SurfaceHarness;
use crate::surface::Surface;

/// WCAG's floor for a graphic (and large text): a glyph or a pill must reach it.
pub const GRAPHIC_CONTRAST: f32 = 3.0;

impl<S: Surface> SurfaceHarness<S> {
    /// [`ink_contrast`] of this surface's document.
    pub fn ink_contrast(&mut self, selector: &str) -> Option<f32> {
        ink_contrast(&mut self.harness, self.viewport.scale_percent, selector)
    }

    /// The `(name, selector)` inks that paint below [`GRAPHIC_CONTRAST`] (or not at all), as
    /// readable lines; empty when every one reads.
    pub fn weak_inks(&mut self, inks: &[(&str, &str)]) -> Vec<String> {
        inks.iter()
            .filter_map(|(name, selector)| {
                let contrast = self.ink_contrast(selector);
                contrast
                    .is_none_or(|got| got < GRAPHIC_CONTRAST)
                    .then(|| format!("{name} ({selector}): {contrast:?}"))
            })
            .collect()
    }
}

/// The environment pinned to `theme`.
fn appearance(theme: Theme) -> Environment {
    let mut environment = Environment::default();
    environment.settings.appearance.theme = theme;
    environment
}

/// A surface's `inks` read in both schemes: when it starts in either one, and after a live
/// switch to the other (the regression: a live change to Dark left glyphs in their light-scheme
/// ink, dark on a dark surface). `start` mounts the surface pinned to the theme it is given.
pub fn assert_inks_read_in_each_scheme<S: Surface>(
    inks: &[(&str, &str)],
    start: impl Fn(Environment) -> SurfaceHarness<S>,
) {
    for theme in [Theme::Light, Theme::Dark] {
        let mut run = start(appearance(theme));
        assert_eq!(
            run.weak_inks(inks),
            Vec::<String>::new(),
            "starting in {theme:?}"
        );
    }
    for (from, to) in [(Theme::Light, Theme::Dark), (Theme::Dark, Theme::Light)] {
        let mut run = start(appearance(from));
        run.set_appearance(appearance(to));
        run.harness.advance(Duration::from_secs(1));
        assert_eq!(
            run.weak_inks(inks),
            Vec::<String>::new(),
            "{from:?} to {to:?}"
        );
    }
}

/// The WCAG contrast between the ground behind the first element matching `selector` (the colour
/// most of its box shows) and the pixel that differs most from it: how well its glyph or text
/// reads as painted, in a document drawn at `scale_percent`. `None` when nothing matches or the
/// render fails; about 1 for an element that paints nothing visible against its ground.
pub fn ink_contrast(harness: &mut Harness, scale_percent: u16, selector: &str) -> Option<f32> {
    let rect = harness.rect(selector)?;
    let image = harness.render_over(Backdrop::Scheme).ok()?;
    let scale = f32::from(scale_percent) / 100.0;
    let (left, top) = (
        (rect.origin.x.0 * scale) as u32,
        (rect.origin.y.0 * scale) as u32,
    );
    let (right, bottom) = (
        (((rect.origin.x.0 + rect.size.width.0) * scale).ceil() as u32).min(image.width()),
        (((rect.origin.y.0 + rect.size.height.0) * scale).ceil() as u32).min(image.height()),
    );
    let pixels: Vec<[u8; 3]> = (top..bottom)
        .flat_map(|y| (left..right).map(move |x| (x, y)))
        .map(|(x, y)| {
            let [r, g, b, _] = image.get_pixel(x, y).0;
            [r, g, b]
        })
        .collect();
    let ground = most_common(&pixels)?;
    pixels
        .iter()
        .map(|pixel| srgb(*pixel).contrast(srgb(ground)))
        .reduce(f32::max)
}

fn most_common(pixels: &[[u8; 3]]) -> Option<[u8; 3]> {
    let mut seen: HashMap<[u8; 3], usize> = HashMap::new();
    for pixel in pixels {
        *seen.entry(*pixel).or_default() += 1;
    }
    seen.into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(pixel, _)| pixel)
}

fn srgb([r, g, b]: [u8; 3]) -> Srgba {
    Srgba([
        f32::from(r) / 255.0,
        f32::from(g) / 255.0,
        f32::from(b) / 255.0,
        1.0,
    ])
}
