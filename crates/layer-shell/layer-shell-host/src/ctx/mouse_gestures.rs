//! Phase 4 of the flush, the mouse gestures daemon part: when a connection attempt is due (at start, after a
//! pointer enter or a lock while unconnected, and when the wait after a miss has run out),
//! connect and register the socket's reading side with the event loop. Each readable wakeup
//! drains every datagram into `HostState::mouse_gestures_message`; the daemon's end closing removes the
//! source. A miss (no daemon, or one that went away) arms a timer, so a daemon that restarts is
//! found again without waiting for the pointer to enter a surface.

use calloop::PostAction;
use calloop::generic::Generic;
use calloop::timer::{TimeoutAction, Timer};
use calloop::{Interest, Mode};

use crate::ctx::HostCtx;
use crate::input::mouse_gestures::{MouseGestures, Stream, read_all};

/// Connect to the mouse gestures daemon if an attempt is due and the socket exists.
pub(super) fn connect_if_due<M: 'static>(ctx: &mut HostCtx<M>) {
    if !ctx.state.mouse_gestures.retry_due() {
        return;
    }
    let Some(reader) = ctx.state.mouse_gestures.connect() else {
        look_again_later(ctx);
        return;
    };
    cancel_retry(ctx);
    watch_settings(ctx);
    let source = Generic::new(reader, Interest::READ, Mode::Level);
    let inserted = ctx.handle.insert_source(source, |_, fd, ctx| {
        let (messages, stream) = read_all(&**fd);
        if !messages.is_empty() {
            ctx.state.mouse_gestures.settled();
        }
        for message in messages {
            ctx.state.mouse_gestures_message(message);
        }
        match stream {
            Stream::Open => Ok(PostAction::Continue),
            Stream::Closed => {
                ctx.state.mouse_gestures.disconnected();
                look_again_later(ctx);
                Ok(PostAction::Remove)
            }
            Stream::Incompatible { version } => {
                eprintln!(
                    "shell-host: mouse_gestures speaks protocol {version}, this host {}; disconnecting",
                    mouse_gestures_proto::PROTOCOL_VERSION
                );
                ctx.state.mouse_gestures.disconnected();
                look_again_later(ctx);
                Ok(PostAction::Remove)
            }
        }
    });
    if let Err(error) = inserted {
        eprintln!("shell-host: mouse_gestures socket source: {error}");
        ctx.state.mouse_gestures.disconnected();
        look_again_later(ctx);
    }
}

/// Look at the person's gesture settings file every couple of seconds, from the first connection
/// on, and tell the mouse gestures daemon when it changed (the host watches no files). The timer stays: a look while
/// the daemon is away does nothing, and the connect tells the daemon the file anyway.
fn watch_settings<M: 'static>(ctx: &mut HostCtx<M>) {
    if ctx.mouse_gestures_settings.is_some() || !ctx.state.mouse_gestures.watches_settings() {
        return;
    }
    let every = MouseGestures::look_every();
    let armed = ctx
        .handle
        .insert_source(Timer::from_duration(every), move |_, (), ctx| {
            ctx.state.mouse_gestures.settings_changed();
            TimeoutAction::ToDuration(every)
        });
    match armed {
        Ok(token) => ctx.mouse_gestures_settings = Some(token),
        Err(error) => eprintln!("shell-host: mouse_gestures settings timer: {error}"),
    }
}

/// Make an attempt due after the wait the client names, replacing a timer already armed (the wait
/// is shorter once the session locks).
fn look_again_later<M: 'static>(ctx: &mut HostCtx<M>) {
    cancel_retry(ctx);
    let Some(wait) = ctx.state.mouse_gestures.retry_in() else {
        return;
    };
    let armed = ctx
        .handle
        .insert_source(Timer::from_duration(wait), |_, (), ctx| {
            ctx.mouse_gestures_retry = None;
            ctx.state.mouse_gestures.retry_now();
            TimeoutAction::Drop
        });
    match armed {
        Ok(token) => ctx.mouse_gestures_retry = Some(token),
        Err(error) => eprintln!("shell-host: mouse_gestures retry timer: {error}"),
    }
}

/// Remove the pending retry timer, if any.
fn cancel_retry<M: 'static>(ctx: &mut HostCtx<M>) {
    if let Some(token) = ctx.mouse_gestures_retry.take() {
        ctx.handle.remove(token);
    }
}
