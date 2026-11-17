# The input broker

A small root service that creates only allowed virtual input devices for named desktop programs.

On Linux, a program that wants to move the pointer, scroll or press a shortcut by itself needs
`/dev/uinput`. Giving that node to the logged-in user (the usual `uaccess` rule) lets every program of
that user create a keyboard and type into whatever has focus, past any consent the desktop asks for.
The input broker closes the node to everyone but itself and makes devices for two known programs:

| Program | Devices | How |
|---|---|---|
| The mouse gestures daemon (Magic Mouse gestures) | a touchpad, a scroll wheel, a keyboard for shortcut combinations | proxied: the mouse gestures daemon sends events, the input broker checks each one against a ceiling and writes it |
| The key remap daemon (Mac keyboard mapping) | a full keyboard with its lights, the pointer half of a combo device | handed: the input broker makes the device and passes the key remap daemon its file descriptor |

Who is calling is decided by the kernel (the process behind the socket, held by a pidfd) and a
root-owned table of desktop programs read through [peephole](https://github.com/Bardo-DE/peephole),
not by anything the caller says. A kind of device is only made for the program it belongs to.

## Crates

| Crate | What |
|---|---|
| `input-broker-wire` | Pure. The datagrams, their codec, the fixed list of device kinds and the per-event ceilings. |
| `input-broker-client` | What the mouse gestures daemon and the key remap daemon link. Sync and small: connect, ask for a device, send events, get a descriptor. A `DeviceSource` trait with the real service and, under feature `fake`, a stand-in. Re-exports the wire types callers name. |
| `input_broker` | The root service: socket activation, caller identity, the device life cycle, exit after 60 s idle. |

```rust
use input_broker_client::{Broker, Create, DeviceSource, Made, Session};

let mut session = Broker::default().open()?;       // /run/bardo/input-broker/input.sock
let Made::Proxied(pad) = session.create(&Create::touchpad())? else { unreachable!() };
session.send(pad, &frame)?;                        // Sent::Queued, or Sent::Dropped when the service is behind
let Made::Handed { fd, .. } = session.create(&Create::keyboard())? else { unreachable!() };
```

## Files it ships (`dist/`)

| File | Placed at |
|---|---|
| `bardo-input-broker.socket`, `bardo-input-broker.service` | `/usr/lib/systemd/system/` |
| `72-bardo-input-broker.rules` | `/etc/udev/rules.d/`: `/dev/uinput` becomes `root:root 0600` and loses the `uaccess` tag |
| `bardo-input-broker-modules.conf` | `/etc/modules-load.d/input-broker.conf` |
| the table of desktop programs | `/etc/bardo/desktop-callers.toml`, from the desktop release |

The desktop installer places them; to try it by hand, copy them, then
`systemctl daemon-reload`, `udevadm control --reload-rules && udevadm trigger --subsystem-match=misc`,
`systemctl enable --now bardo-input-broker.socket`.

## Tests

Nothing touches the real system: a fake kernel behind a trait, sockets in scratch folders, tables for
every ceiling. The two tests that make a real device (`crates/input_broker/tests/it/device.rs`) are
`#[ignore]`d and run by hand on a machine with `/dev/uinput`.

See `ARCHITECTURE.md` for the design and `FINDINGS.md` for the decisions and what is unverified.

Licensed under either of MIT or Apache-2.0, at your option.
