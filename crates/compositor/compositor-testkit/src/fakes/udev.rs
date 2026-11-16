//! The fake udev: a scripted stream of device events (hotplug reconciliation, P1-D1), as
//! `compositor_backend::tty::DeviceSource`.

use compositor_backend::tty::{DeviceEvent, DevicePath, DeviceSource};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

/// A scripted udev.
#[derive(Debug, Default)]
pub struct FakeUdev {
    present: Vec<DevicePath>,
    script: VecDeque<DeviceEvent>,
}

impl FakeUdev {
    /// A udev that will report `script` in order.
    #[must_use]
    pub fn scripted(script: Vec<DeviceEvent>) -> FakeUdev {
        FakeUdev {
            present: Vec::new(),
            script: script.into(),
        }
    }

    /// The same, with `present` devices there from the start.
    #[must_use]
    pub fn with_present(self, present: Vec<DevicePath>) -> FakeUdev {
        FakeUdev { present, ..self }
    }

    /// Queues one more event.
    pub fn push(&mut self, event: DeviceEvent) {
        self.script.push_back(event);
    }
}

impl DeviceSource for FakeUdev {
    fn present(&self) -> Vec<DevicePath> {
        self.present.clone()
    }

    fn next_event(&mut self) -> Option<DeviceEvent> {
        self.script.pop_front()
    }
}

/// A [`FakeUdev`] a backend and a test both hold: the backend takes one clone as its
/// `DeviceSource`, the test scripts hotplug through another.
///
/// ```
/// use compositor_backend::tty::{DeviceEvent, DevicePath, DeviceSource};
/// use compositor_testkit::fakes::{FakeUdev, SharedUdev};
/// use std::path::PathBuf;
///
/// let udev = SharedUdev::new(FakeUdev::default());
/// let mut for_the_backend = udev.clone();
/// let card = DevicePath(PathBuf::from("/dev/dri/card0"));
/// udev.push(DeviceEvent::Added(card.clone()));
/// assert_eq!(for_the_backend.next_event(), Some(DeviceEvent::Added(card)));
/// ```
#[derive(Debug, Clone)]
pub struct SharedUdev(Rc<RefCell<FakeUdev>>);

impl SharedUdev {
    /// Shares `udev`.
    #[must_use]
    pub fn new(udev: FakeUdev) -> SharedUdev {
        SharedUdev(Rc::new(RefCell::new(udev)))
    }

    /// Queues one more event.
    pub fn push(&self, event: DeviceEvent) {
        self.0.borrow_mut().push(event);
    }
}

impl DeviceSource for SharedUdev {
    fn present(&self) -> Vec<DevicePath> {
        self.0.borrow().present()
    }

    fn next_event(&mut self) -> Option<DeviceEvent> {
        self.0.borrow_mut().next_event()
    }
}
