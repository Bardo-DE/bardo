//! `bardo_desktop_manager_v1` (P2-Z; filled by P2-Q1): the wire glue lives in
//! `compositor_wayland::bardo::desktop`; this implements what it asks of the state.

use crate::state::State;
use compositor_wayland::bardo::desktop::{BardoDesktopHandler, DesktopParts, DesktopView};

impl BardoDesktopHandler for State {
    fn bardo_desktop_parts(&mut self) -> DesktopParts<'_> {
        DesktopParts {
            part: &mut self.wl.bardo_desktop,
            view: DesktopView {
                model: &self.core.model,
                outputs: &self.wl.output.outputs,
                rects: self
                    .core
                    .arrangement
                    .windows
                    .iter()
                    .map(|target| (target.window, target.rect))
                    .collect(),
            },
        }
    }
}
