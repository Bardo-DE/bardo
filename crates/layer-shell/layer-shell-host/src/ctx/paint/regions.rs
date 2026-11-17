//! The input and blur regions sent to the compositor: the only place they are sent from.

use smithay_client_toolkit::compositor::{CompositorState, Region};
use wayland_client::QueueHandle;

use crate::csd::{Inset, band, to_surface};
use crate::handlers::background_effect::BackgroundEffect;
use crate::resolve::resolve;
use crate::state::HostState;
use crate::surface::HostSurface;
use layer_shell_types::{BlurSpec, Rect, Size};

/// Resolve the input and blur regions against the laid-out document and send whichever
/// changed since they were last sent. Both are the document's, moved to surface coordinates;
/// the input region adds the resize band (`csd::band`), the blur region never covers it.
pub(super) fn send_regions(
    compositor: &CompositorState,
    effect: Option<&mut BackgroundEffect>,
    qh: &QueueHandle<HostState>,
    surface: &mut HostSurface,
    blur: bool,
) {
    let size = surface.document_size();
    let inset = surface.inset;
    let document = &surface.document;
    let input = in_surface(
        resolve(&surface.input, size, |id| document.element_rect(&id)),
        inset,
        band(surface.logical, inset),
    );
    let blur = match (&surface.blur, blur) {
        (BlurSpec::Region(spec), true) => in_surface(
            resolve(spec, size, |id| document.element_rect(&id)),
            inset,
            Vec::new(),
        ),
        (BlurSpec::None, _) | (BlurSpec::Region(_), false) => Some(Vec::new()),
    };
    let wl = surface.role.wl_surface();
    if surface.regions.input.update(input.as_deref())
        && let Some(rects) = &input
    {
        let region = region_of(compositor, rects);
        wl.set_input_region(region.as_ref().map(Region::wl_region));
    }
    let opaque = opaque_region(surface.role.opaque(), surface.logical);
    if surface.regions.opaque.update(opaque.as_deref())
        && let Some(rects) = &opaque
    {
        let region = region_of(compositor, rects);
        wl.set_opaque_region(region.as_ref().map(Region::wl_region));
    }
    if surface.regions.blur.update(blur.as_deref())
        && let Some(rects) = &blur
        && let Some(effect) = effect
    {
        let region = (!rects.is_empty())
            .then(|| region_of(compositor, rects))
            .flatten();
        effect.set_blur_region(qh, wl, region.as_ref().map(Region::wl_region));
    }
}

/// A document-coordinate region in surface coordinates, plus `extra` surface strips. A
/// pending region stays pending: the surface keeps the one it has.
fn in_surface(resolved: Option<Vec<Rect>>, inset: Inset, extra: Vec<Rect>) -> Option<Vec<Rect>> {
    resolved.map(|rects| {
        let mut strips = to_surface(rects, inset);
        strips.extend(extra);
        strips
    })
}

fn region_of(compositor: &CompositorState, rects: &[Rect]) -> Option<Region> {
    let region = Region::new(compositor).ok()?;
    for rect in rects {
        region.add(rect.x, rect.y, rect.width, rect.height);
    }
    Some(region)
}

/// The opaque region of a surface of `logical` size, in surface-local logical coordinates:
/// all of it when the surface is `opaque` and has an area, else nothing to send. The region
/// follows the logical size, so a resize resends it and a scale change (same logical size)
/// does not.
fn opaque_region(opaque: bool, logical: Size) -> Option<Vec<Rect>> {
    (opaque && logical.has_area()).then(|| {
        vec![Rect {
            x: 0,
            y: 0,
            width: logical.width,
            height: logical.height,
        }]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size(width: i32, height: i32) -> Size {
        Size { width, height }
    }

    #[test]
    fn an_opaque_surface_declares_all_of_itself_opaque() {
        let whole = Rect {
            x: 0,
            y: 0,
            width: 3840,
            height: 2160,
        };
        assert_eq!(opaque_region(true, size(3840, 2160)), Some(vec![whole]));
    }

    #[test]
    fn a_translucent_or_unsized_surface_sends_no_opaque_region() {
        assert_eq!(opaque_region(false, size(3840, 2160)), None);
        assert_eq!(opaque_region(true, size(0, 0)), None);
    }

    #[test]
    fn a_resize_changes_the_opaque_region_and_a_repeat_does_not() {
        let mut tracked = crate::resolve::Tracked::holding(Vec::new());
        assert!(tracked.update(opaque_region(true, size(100, 50)).as_deref()));
        assert!(!tracked.update(opaque_region(true, size(100, 50)).as_deref()));
        assert!(tracked.update(opaque_region(true, size(120, 50)).as_deref()));
    }
}
