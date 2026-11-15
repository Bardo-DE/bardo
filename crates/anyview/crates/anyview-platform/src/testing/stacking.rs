use super::locked;
use crate::stacking::WindowStacking;
use std::sync::{Arc, Mutex};

/// A [`WindowStacking`] that records the requests and answers by its support.
#[derive(Debug, Clone)]
pub struct FakeStacking {
    supported: bool,
    requested: Arc<Mutex<Vec<bool>>>,
}

impl FakeStacking {
    /// A desktop that can keep a window above when `supported`.
    pub fn with(supported: bool) -> Self {
        FakeStacking {
            supported,
            requested: Arc::default(),
        }
    }

    /// Every request (`keep_above`), oldest first.
    pub fn requested(&self) -> Vec<bool> {
        locked(&self.requested).clone()
    }
}

impl WindowStacking for FakeStacking {
    fn request(&self, keep_above: bool) -> bool {
        locked(&self.requested).push(keep_above);
        !keep_above || self.supported
    }
}
