//! A platform object of the surface, as its document sees it: the host puts it in, a component
//! takes it out by type. The host rebuilds a hidden surface with a new object, so the handle reads
//! it through a cell the host refreshes, and never keeps a stale one.
//!
//! What the object is (a Wayland `wl_surface`, for the Wayland host) is the host's business: this
//! crate names no platform type, so the cell holds it type-erased.
//!
//! See FINDINGS F34.

use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// One surface's current platform object, shared by the host and its `SurfaceHandle`.
#[derive(Clone, Default)]
pub struct NativeCell(Rc<RefCell<Option<Rc<dyn Any>>>>);

impl NativeCell {
    /// A cell holding `object`.
    pub fn new<T: 'static>(object: T) -> NativeCell {
        NativeCell(Rc::new(RefCell::new(Some(Rc::new(object)))))
    }

    /// A cell with no object: handles built without a platform connection (tests).
    #[must_use]
    pub fn detached() -> NativeCell {
        NativeCell::default()
    }

    /// Replace the object (the host rebuilt the role).
    pub fn set<T: 'static>(&self, object: T) {
        *self.0.borrow_mut() = Some(Rc::new(object));
    }

    /// The object now, when it is a `T`; `None` for a detached cell or another type.
    #[must_use]
    pub fn get<T: Clone + 'static>(&self) -> Option<T> {
        self.0
            .borrow()
            .as_ref()
            .and_then(|object| object.downcast_ref::<T>())
            .cloned()
    }
}

impl std::fmt::Debug for NativeCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeCell")
            .field("held", &self.0.borrow().is_some())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cell_hands_out_the_latest_object_of_the_asked_type() {
        let cell = NativeCell::new(7_u32);
        assert_eq!(cell.get::<u32>(), Some(7));
        assert_eq!(cell.get::<u64>(), None, "another type is not it");
        cell.set(9_u32);
        assert_eq!(cell.get::<u32>(), Some(9));
        assert_eq!(NativeCell::detached().get::<u32>(), None);
    }
}
