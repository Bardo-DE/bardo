//! Now Playing: every MPRIS player on the session bus (`org.mpris.MediaPlayer2.*`) and which one
//! the control center shows (design/20 §1.5, design/13 §13.3.7: the Now Playing module).
//!
//! Spoken through hand-written `zbus` proxies for `org.mpris.MediaPlayer2` and
//! `org.mpris.MediaPlayer2.Player`, found by `ListNames` and followed by `NameOwnerChanged`
//! (the `mpris` crate runs on the `dbus` crate and libdbus). Album art reaches the surface only as
//! a file in `$XDG_CACHE_HOME/bardo/shell/art/`, the one folder of covers the shell may draw from: an
//! `http(s)://` one is downloaded once into it and a `file://` one is copied into it, each only
//! within a size (`shell_model::now_playing::art_source`, `backend::art`).

mod backend;

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::Run;
use shell_model::now_playing::NowPlayingService;
use shell_model::service::ServiceIo;

impl Run for NowPlayingService {
    type Kind = Self;
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        backend::run(
            env.buses.session.clone(),
            env.dirs.shell_cache().join("art"),
            io.state,
            io.commands,
        )
    }
}
