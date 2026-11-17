//! What a component asks of the host: the requests a `SurfaceHandle` or a `LockHandle` queues, and
//! the queue the host drains.
//!
//! A queue, not a call, because a request is made while the host holds the document borrowed
//! (inside a render or an event handler): the host applies it after the documents are polled and
//! before it paints. It is the seam between these hooks and the host that renders the surfaces.

use std::any::Any;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt;
use std::rc::Rc;

use layer_shell_types::{DragOutcome, LockId, PopupConfig, PopupPlacement, SurfaceId, SurfaceOp};
use tokio::sync::oneshot;

use crate::drag::DragSpec;
use crate::root::HostRoot;

/// A request made from inside a document through its `SurfaceHandle`. Queued on the main
/// thread (a `HostRoot` is not `Send`), drained after the documents are polled.
#[derive(Debug)]
pub enum Request {
    Apply {
        id: SurfaceId,
        op: SurfaceOp,
    },
    OpenPopup {
        parent: SurfaceId,
        /// Allocated by the handle, so `open_popup` can return it before the popup exists.
        id: SurfaceId,
        config: PopupConfig,
        placement: PopupPlacement,
        root: HostRoot,
    },
    Close(SurfaceId),
    /// Render surface `id` on the next flush (a frame drive was taken on an idle surface, which
    /// has no callback coming to tick on).
    Redraw(SurfaceId),
    CopyText {
        text: String,
    },
    ActivationToken {
        from: SurfaceId,
        app_id: Option<String>,
        reply: oneshot::Sender<Option<String>>,
    },
    /// Start dragging out of surface `from` (`SurfaceHandle::start_drag`); `reply` gets the
    /// outcome when the drag ends, or at once when it cannot start.
    StartDrag {
        from: SurfaceId,
        spec: DragSpec,
        reply: oneshot::Sender<DragOutcome>,
    },
    /// Unlock session lock `LockId` (`LockHandle::unlock`, `HostCtx::unlock_session`).
    Unlock(LockId),
    /// A request only the host understands, sent with `SurfaceHandle::send_host`: the host
    /// downcasts it to the requests it defines (a programmatic scroll, for one).
    Host(Box<dyn Any>),
}

/// The main-thread queue a `SurfaceHandle` pushes into, and the wake that tells the loop to
/// drain it. Ops queue while a document is borrowed and apply after `poll()`, before paint.
#[derive(Clone)]
pub struct RequestQueue {
    queue: Rc<RefCell<VecDeque<Request>>>,
    wake: Rc<dyn Fn()>,
}

impl RequestQueue {
    /// A queue that calls `wake` on every push.
    pub fn new(wake: impl Fn() + 'static) -> RequestQueue {
        RequestQueue {
            queue: Rc::default(),
            wake: Rc::new(wake),
        }
    }

    /// Queue `request` and wake the loop.
    pub fn push(&self, request: Request) {
        self.queue.borrow_mut().push_back(request);
        (self.wake)();
    }

    /// Everything queued so far, oldest first.
    pub fn drain(&self) -> Vec<Request> {
        self.queue.borrow_mut().drain(..).collect()
    }
}

impl fmt::Debug for RequestQueue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RequestQueue")
            .field("queued", &self.queue.borrow().len())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    #[test]
    fn the_request_queue_is_first_in_first_out() {
        let woken = Rc::new(Cell::new(0_u32));
        let seen = Rc::clone(&woken);
        let queue = RequestQueue::new(move || seen.set(seen.get() + 1));
        let ids = [3, 1, 2].map(SurfaceId);
        for id in ids {
            queue.push(Request::Close(id));
        }
        assert_eq!(woken.get(), 3, "every push wakes the loop");
        let drained: Vec<SurfaceId> = queue
            .drain()
            .into_iter()
            .filter_map(|request| match request {
                Request::Close(id) => Some(id),
                _ => None,
            })
            .collect();
        assert_eq!(drained, ids);
        assert!(queue.drain().is_empty());
    }
}
