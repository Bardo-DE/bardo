//! A compositor in a thread, over a socket pair: `wl_compositor` and `xdg_wm_base` for a client to
//! make a toplevel with, and, unless left out, `bardo_window_manager_v1`, which records what its
//! windows are sent.

use super::proto::server::{
    bardo_window_manager_v1::{self, BardoWindowManagerV1},
    bardo_window_v1::{self, BardoWindowV1, FrameMode},
};
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;
use wayland_protocols::xdg::shell::server::{
    xdg_surface::{self, XdgSurface},
    xdg_toplevel::{self, XdgToplevel},
    xdg_wm_base::{self, XdgWmBase},
};
use wayland_server::backend::{ClientData, ClientId, DisconnectReason};
use wayland_server::protocol::{
    wl_compositor::{self, WlCompositor},
    wl_surface::{self, WlSurface},
};
use wayland_server::{
    Client, DataInit, Dispatch, Display, DisplayHandle, GlobalDispatch, New, WEnum,
};

/// Who draws the frame effects, as a window said.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    Client,
    CompositorShadow,
    Unknown,
}

/// One `set_frame` the compositor was sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Frame {
    pub(super) mode: Mode,
    pub(super) corner_radius: u32,
}

/// What the compositor has been sent.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct Log {
    /// The `set_frame` requests, oldest first.
    pub(super) frames: Vec<Frame>,
    /// The `get_window` requests.
    pub(super) windows: u32,
    /// The window objects destroyed.
    pub(super) destroyed: u32,
}

/// What the compositor offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Offer {
    /// `wl_compositor`, `xdg_wm_base` and `bardo_window_manager_v1`.
    Quire,
    /// `wl_compositor` and `xdg_wm_base` alone, as GNOME and KDE are.
    NoQuire,
}

/// The running compositor: stopped and joined when dropped.
pub(super) struct Fake {
    seen: Arc<Mutex<Log>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Fake {
    /// Start a compositor offering `offer`, and the client's end of its socket.
    pub(super) fn start(offer: Offer) -> (Fake, UnixStream) {
        let (client, server) = UnixStream::pair().expect("a socket pair");
        let seen = Arc::new(Mutex::new(Log::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let (seen, stop) = (Arc::clone(&seen), Arc::clone(&stop));
            std::thread::spawn(move || serve(offer, server, seen, &stop))
        };
        let fake = Fake {
            seen,
            stop,
            thread: Some(thread),
        };
        (fake, client)
    }

    /// Everything it has been sent so far.
    pub(super) fn log(&self) -> Log {
        self.seen.lock().expect("the compositor's log").clone()
    }
}

impl Drop for Fake {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn serve(offer: Offer, stream: UnixStream, seen: Arc<Mutex<Log>>, stop: &AtomicBool) {
    let mut display = Display::<State>::new().expect("a display");
    let mut handle = display.handle();
    handle.create_global::<State, WlCompositor, ()>(1, ());
    handle.create_global::<State, XdgWmBase, ()>(1, ());
    if offer == Offer::Quire {
        handle.create_global::<State, BardoWindowManagerV1, ()>(1, ());
    }
    handle
        .insert_client(stream, Arc::new(Gone))
        .expect("the client's socket");
    let mut state = State { seen };
    while !stop.load(Ordering::Acquire) {
        display.dispatch_clients(&mut state).expect("dispatching");
        display.flush_clients().expect("flushing");
        std::thread::sleep(Duration::from_millis(1));
    }
}

struct State {
    seen: Arc<Mutex<Log>>,
}

impl State {
    fn log(&self) -> std::sync::MutexGuard<'_, Log> {
        self.seen.lock().expect("the compositor's log")
    }
}

struct Gone;

impl ClientData for Gone {
    fn initialized(&self, _: ClientId) {}
    fn disconnected(&self, _: ClientId, _: DisconnectReason) {}
}

/// A global that is bound with no data, and a resource that only needs creating.
macro_rules! plain_global {
    ($($interface:ty),*) => {$(
        impl GlobalDispatch<$interface, ()> for State {
            fn bind(
                _: &mut Self,
                _: &DisplayHandle,
                _: &Client,
                resource: New<$interface>,
                _: &(),
                data_init: &mut DataInit<'_, Self>,
            ) {
                data_init.init(resource, ());
            }
        }
    )*};
}

plain_global!(WlCompositor, XdgWmBase, BardoWindowManagerV1);

impl Dispatch<WlCompositor, ()> for State {
    fn request(
        _: &mut Self,
        _: &Client,
        _: &WlCompositor,
        request: wl_compositor::Request,
        (): &(),
        _: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        if let wl_compositor::Request::CreateSurface { id } = request {
            data_init.init(id, ());
        }
    }
}

impl Dispatch<WlSurface, ()> for State {
    fn request(
        _: &mut Self,
        _: &Client,
        _: &WlSurface,
        _: wl_surface::Request,
        (): &(),
        _: &DisplayHandle,
        _: &mut DataInit<'_, Self>,
    ) {
    }
}

impl Dispatch<XdgWmBase, ()> for State {
    fn request(
        _: &mut Self,
        _: &Client,
        _: &XdgWmBase,
        request: xdg_wm_base::Request,
        (): &(),
        _: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        if let xdg_wm_base::Request::GetXdgSurface { id, .. } = request {
            data_init.init(id, ());
        }
    }
}

impl Dispatch<XdgSurface, ()> for State {
    fn request(
        _: &mut Self,
        _: &Client,
        _: &XdgSurface,
        request: xdg_surface::Request,
        (): &(),
        _: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        if let xdg_surface::Request::GetToplevel { id } = request {
            data_init.init(id, ());
        }
    }
}

impl Dispatch<XdgToplevel, ()> for State {
    fn request(
        _: &mut Self,
        _: &Client,
        _: &XdgToplevel,
        _: xdg_toplevel::Request,
        (): &(),
        _: &DisplayHandle,
        _: &mut DataInit<'_, Self>,
    ) {
    }
}

impl Dispatch<BardoWindowManagerV1, ()> for State {
    fn request(
        state: &mut Self,
        _: &Client,
        _: &BardoWindowManagerV1,
        request: bardo_window_manager_v1::Request,
        (): &(),
        _: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        if let bardo_window_manager_v1::Request::GetWindow { id, .. } = request {
            state.log().windows += 1;
            data_init.init(id, ());
        }
    }
}

impl Dispatch<BardoWindowV1, ()> for State {
    fn request(
        state: &mut Self,
        _: &Client,
        _: &BardoWindowV1,
        request: bardo_window_v1::Request,
        (): &(),
        _: &DisplayHandle,
        _: &mut DataInit<'_, Self>,
    ) {
        match request {
            bardo_window_v1::Request::SetFrame {
                mode,
                corner_radius,
            } => {
                let mode = match mode {
                    WEnum::Value(FrameMode::Client) => Mode::Client,
                    WEnum::Value(FrameMode::CompositorShadow) => Mode::CompositorShadow,
                    _ => Mode::Unknown,
                };
                state.log().frames.push(Frame {
                    mode,
                    corner_radius,
                });
            }
            bardo_window_v1::Request::Destroy => state.log().destroyed += 1,
            _ => {}
        }
    }
}
