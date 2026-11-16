//! Test support: a fake keycap daemon on a private socket that speaks
//! the keymap part of the protocol. It needs no devices, so these tests run everywhere.

use key_remap_proto::{
    self as protocol, KeymapCall, KeymapState, Registration, Status, ToClient, ToDaemon,
};
use rustix::fd::OwnedFd;
use rustix::net::{AddressFamily, RecvFlags, SendFlags, SocketAddrUnix, SocketType};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Shared {
    keymap: KeymapState,
    watchers: Vec<Arc<OwnedFd>>,
    registered: Vec<(usize, Registration)>,
    next_id: usize,
}

/// A daemon that serves one keymap, watchers, and registrations by connection.
pub(crate) struct FakeDaemon {
    pub(crate) socket: PathBuf,
    shared: Arc<Mutex<Shared>>,
    _dir: tempfile::TempDir,
}

fn send(fd: &OwnedFd, message: &ToClient) {
    let _ = rustix::net::send(fd, &protocol::encode(message), SendFlags::NOSIGNAL);
}

impl FakeDaemon {
    /// A daemon whose `Hello` says `version`, holding `keymap`.
    pub(crate) fn start(version: u16, keymap: KeymapState) -> FakeDaemon {
        let dir = tempfile::tempdir().expect("scratch dir");
        let socket = dir.path().join("keycap.sock");
        let listener =
            rustix::net::socket(AddressFamily::UNIX, SocketType::SEQPACKET, None).expect("socket");
        rustix::net::bind(&listener, &SocketAddrUnix::new(&socket).expect("path")).expect("bind");
        rustix::net::listen(&listener, 16).expect("listen");
        let shared = Arc::new(Mutex::new(Shared {
            keymap,
            ..Shared::default()
        }));
        let accepting = Arc::clone(&shared);
        std::thread::spawn(move || {
            while let Ok(fd) = rustix::net::accept(&listener) {
                let shared = Arc::clone(&accepting);
                std::thread::spawn(move || serve(Arc::new(fd), version, &shared));
            }
        });
        FakeDaemon {
            socket,
            shared,
            _dir: dir,
        }
    }

    /// Replaces the keymap and tells every watcher.
    pub(crate) fn change(&self, keymap: KeymapState) {
        let mut shared = self.shared.lock().expect("lock");
        shared.keymap = keymap.clone();
        for watcher in &shared.watchers {
            send(watcher, &ToClient::Keymap(keymap.clone()));
        }
    }

    /// How many connections have asked to hear the keymap.
    pub(crate) fn watchers(&self) -> usize {
        self.shared.lock().expect("lock").watchers.len()
    }
}

fn serve(fd: Arc<OwnedFd>, version: u16, shared: &Mutex<Shared>) {
    let me = {
        let mut shared = shared.lock().expect("lock");
        shared.next_id += 1;
        shared.next_id
    };
    send(
        &fd,
        &ToClient::Hello {
            version,
            status: Status {
                paused: false,
                keyboards: vec![],
            },
        },
    );
    let mut buf = vec![0u8; protocol::MAX_PACKET];
    while let Ok((n, _)) = rustix::net::recv(&*fd, &mut buf, RecvFlags::empty()) {
        if n == 0 {
            break;
        }
        let Ok(message) = protocol::decode::<ToDaemon>(&buf[..n]) else {
            break;
        };
        let mut shared = shared.lock().expect("lock");
        match message {
            ToDaemon::Keymap(KeymapCall::Watch) => {
                shared.watchers.push(Arc::clone(&fd));
                send(&fd, &ToClient::Keymap(shared.keymap.clone()));
            }
            ToDaemon::Keymap(KeymapCall::Register(registration)) => {
                shared.registered.push((me, registration));
            }
            ToDaemon::Keymap(KeymapCall::List) => {
                let all = shared.registered.iter().map(|(_, r)| r.clone()).collect();
                send(&fd, &ToClient::Registered(all));
            }
            ToDaemon::Steer(_) => {}
        }
    }
    let mut shared = shared.lock().expect("lock");
    shared.registered.retain(|(owner, _)| *owner != me);
    shared.watchers.retain(|w| !Arc::ptr_eq(w, &fd));
}
