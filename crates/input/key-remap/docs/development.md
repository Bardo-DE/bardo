# Development notes

Internals for contributors. User documentation is in `README.md`; the crate map and layering rules are in `ARCHITECTURE.md`.

## Architecture

```
evdev (grabbed) ──► check_gate ──► Remapper::step ──► Vec<RemapOut> ──► dispatch ──► uinput keyboard
   EVIOCGRAB       (no key held)   (pure state)                  │
   └─ touchpad axes of a combo device ──────────────────────────┴─────────────────► "<name> (the key remap daemon pointer)"
socket ── Focus ───────────────────► Remapper::step (profile)    └─ ReleaseGrabs ─► ungrab, Paused
socket ── UserConfig ──► config merge ──► Profiles
/usr/lib/bardo/key-remap/key-remap.toml, /etc/bardo/key-remap/key-remap.toml ── inotify ──► config merge
```

Two threads and no async runtime: the **reader** (poll over every grabbed fd, an eventfd and
the timeout; pets the systemd watchdog, drains the socket channel, then feeds the core) and the
**socket** thread (accept, peer classification, fan-out of `Status`), woken by an eventfd so
the reader never waits on a client.

The pure core takes events and instants and returns values; nothing in it reads a clock or
touches a device, so a script replays to exactly the outputs the live daemon produced.

| Module | What it is |
| --- | --- |
| `key` | `Key(u16)` from the name table, `Modifier`, `Side`, `Mods` |
| `event` | `Edge`, `KeyEvent`, the `Physical` and `Logical` key newtypes |
| `layout` | `Layout { Apple, Pc }` from the vendor; `role` by key position |
| `profile` | `Trigger`, `Stroke`, `RuleAction`, `Profile`, `Profiles` (`default` always exists) |
| `config` | lenient TOML into `ConfigLayer`s (built-in defaults, system file, the seat owner's `UserConfig`), folded by `resolve` |
| `remap` | `Remapper::step(self, RemapInput, &Profiles) -> (Self, Vec<RemapOut>)` |
| `held` | what each physical press emitted, so its release sends exactly that |
| `gate` | `check_gate`: a keyboard is grabbed only once no key is down |
| `escape` | the emergency chord, both Shifts + Esc |
| `backoff` | the pure reconnect backoff |
| `key-remap-proto` (crate) | the socket messages and their codec |
| `keymap` | the person's keymap file (`keymap`, chordkit's overrides line format): read, judged against the last good copy, watched |
| `key-remap-client` (crate) | a program's end of the socket, and `KeyRemapSource`, this desktop's chordkit keymap source |
| `daemon` | threads, socket, discovery, the uinput keyboard, sd_notify, sd_notify; the ioctls are `key-remap-evdev`, through `input-linux`'s safe handles (the workspace forbids `unsafe`) |

## The socket protocol

`/run/bardo/key-remap/key-remap.sock`, `SOCK_SEQPACKET`, mode 0666, one JSON message per packet, adjacently
tagged (`{"kind": "...", "v": {...}}`). The first packet on a connection is `Hello`; a client
checks `version` (`PROTOCOL_VERSION = 3`, `MIN_PROTOCOL_VERSION = 3`: nothing older is read) before reading on. `MAX_PACKET` is 64 KiB, so a
config fits in one packet.

```rust
pub enum ToDaemon {
    Steer(Steer),                     // honoured only from the seat owner's programs
    Keymap(KeymapCall),               // open to every client
}
pub enum Steer {
    Focus { profile: ProfileName },   // the whole desktop-facing contract
    UserConfig { toml: String },      // the seat owner's settings, parsed leniently
    Resume,                           // after the escape chord or a pause
}
pub enum KeymapCall {
    Watch,                            // send the keymap now, and on each change
    Register(Registration),           // an app's actions, until the connection closes
    List,                             // every app's actions
}
pub enum ToClient {
    Hello { version: u16, status: Status },
    Status(Status),
    ConfigWarnings(Vec<Warning>),
    Keymap(KeymapState),              // only to a client that sent KeymapCall::Watch
    Registered(Vec<Registration>),    // answering KeymapCall::List
}
pub struct KeymapState { overrides: String, problems: Vec<KeymapProblem> }
// overrides: chordkit's line format, only lines that read; problems: plain words with a line
pub struct Status { paused: bool, keyboards: Vec<Keyboard> }  // empty = no keyboards; paused = escape chord
pub struct Keyboard { info: KeyboardInfo, grab: Grab }
pub enum Grab {
    Active,
    WaitingForRelease,                // grab gate: a key is held
    Conflict,                         // EVIOCGRAB -> EBUSY: another remapper holds it
}
// ProfileName: validated [a-z0-9_-]+, parsed once at the boundary
```

`SO_PEERCRED` and the process's control group sort each client into `Peer { Steering, Plain, Observer }`,
using `ACTIVE_UID` from `/run/systemd/seats/seat0`: `Steering` is the shell (a process in `bardo-shell.service`, or
for the Plasma feed `bardo-key-remap-focus.service`, under `user@<active uid>.service`), `Plain` any other process of
that user, `Observer` anyone else. Only `Steering`'s `Focus` and `UserConfig` are honoured, and `Resume` also from `Plain` (any other request
from the others ends their connection); every client receives `Status`. The user layer the shell sends is
cut down on arrival: no rule on a key with no modifier held, no list or empty `then`, at most 256 rules. The
machine file lifts the list ban with `[user_layer] sequences = true`. The keymap messages are open to every client (they
only read, or inform), and a client that never sends `KeymapCall::Watch` is never sent a `Keymap`. Registrations are kept per connection, at most 16 apps
each, and dropped when the connection closes. A connection that has sent only keymap messages is
an observer for the seat (it is not the owner being present, so its leaving resets nothing) and
counts against its own per-uid cap of 64; the cap on other connections stays 8.

## Lifecycle

The key remap daemon's lifetime is the machine's, not the session's.

| Moment | What the key remap daemon does |
| --- | --- |
| Boot | Starts from `multi-user.target`, before the display manager, and grabs keyboards once no keys are held. The login screen and text consoles already have Mac keys, with the `default` profile. |
| Login | Nothing restarts. The desktop connects and starts sending `Focus`. The seat owner's `UserConfig` applies within one message. |
| Logout or desktop crash | The client's socket closes, and the key remap daemon drops back to `default` and the system config. The keyboards stay grabbed throughout. |
| User switch | `ACTIVE_UID` in `/run/systemd/seats/seat0` changes. The key remap daemon re-reads it when that file changes and only listens to the new owner. |
| Suspend/resume, keyboard reconnect | Each device is re-grabbed through the grab gate. The daemon itself keeps running. |

Only per-app profiles and the user's own settings wait for a session.

## Failure behaviour

| Failure | What the key remap daemon does | Proven by |
| --- | --- | --- |
| Crash or panic (`panic = "abort"`) | The kernel releases the grab, systemd restarts the key remap daemon within 200 ms, and the grab gate waits for keys to be released before grabbing again | gate table tests, `Held::drain` tests, device test (`kill -9`, restart) |
| Hang | Watchdog kill after 2 s (the reader wakes at least every second), then the crash path | device test (ping cadence), systemd tier (`kill -STOP`) |
| Crash loop | The unit becomes failed after 5 restarts; the desktop notices | systemd tier (six `kill -9`s, `reset-failed`) |
| Killed mid-chord | The kernel drops the grab and closes the virtual keyboard with the process, so nothing stays down; the restarted daemon starts with a fresh `Remapper` | gate table tests, device test |
| Keyboard unplugged mid-chord | `DeviceGone` releases exactly what that keyboard's keys wrote | core tests, device test |
| Profile changed mid-chord | `Focus` applies from the next press; held keys keep what they wrote | core tests |
| Combo device (keyboard and touchpad in one node) | Its keys are remapped as usual; its absolute axes, buttons and motion go to a second virtual device, `<name> (key_remap pointer)`, cloned from the node's axes, ranges and properties, made when the device is grabbed and destroyed when it is let go, unplugged or paused, after its held buttons are released | table tests of the capability value, recording twins in the reader tests; hardware unverified |
| Focus on a profile the config does not define | The default profile applies and the log says so once per name until the config changes | `Unknowns` table test |
| Another remapper holds the keyboard | `Grab::Conflict`; the grab is retried after 1 s, doubling to 30 s, never in a loop | devices tests with a fake grabber, device test with a second grabber |
| Emergency chord | `RemapOut::ReleaseGrabs` and `Status::Paused` | core test, device test (`Resume` included) |

`RemapInput::ReleaseAll` is the core's drain entry point, used by the tests and by `key_remap replay`
(`release-all`); the daemon never sends it, because a process that dies takes its grab with it.

## Safety rules

The key remap daemon grabs the user's only keyboard, so:

- **Tests create no input device unless they are the gated device tests.** The ordinary suite
  drives the core with scripts and a recording sink and never constructs the uinput device. The
  device tests (`crates/key_remap/tests/devices`) are the one exception: each is `#[ignore]`d and
  asserts `BARDO_KEY_REMAP_DEVICE_TESTS=1`, so neither `cargo test` nor `cargo test -- --ignored` alone
  creates a device. CI runs them (see Tests).
- **Virtual devices are never grabbed**, so another remapper's output cannot loop back through
  key_remap. The one exception is `BARDO_KEY_REMAP_GRAB_VIRTUAL=<exact device name>`, which makes the virtual
  device with that name eligible like a hardware keyboard, for tests whose keyboard is itself a
  uinput device. Only root, or the unit's `Environment=`, can set it; it admits
  exactly one named device and makes it the only device grabbed (no hardware keyboard is touched),
  and the key remap daemon's own keyboard and palmrest's stay excluded even when named. Leave it unset on a
  real machine.
- **No `record` command.** A tool that writes every keystroke to a file is a keylogger. Test
  scripts are synthesised by hand; `key_remap replay FILE` feeds one through the core.
- `key_remap --check-uinput` opens and closes `/dev/uinput` without `UI_DEV_CREATE`, and only when
  `BARDO_KEY_REMAP_ALLOW_UINPUT_PROBE=1` is set; when the device service's socket is there it only connects to it.
- The emergency chord (both Shifts + Esc) releases every grab at once; `Resume` grabs again (the shell
  or any program of the seat's user may send it).
- The key remap daemon never changes kernel module parameters or `/etc` files; `install.sh` does, with sudo,
  and only when asked (`--dry-run` prints the same commands and runs none).
- The device tests make "the key remap daemon keyboard" write real key events to the desktop session. Run them
  on a machine you are not using (CI), not in your own session: a mapped Cmd+Q is Alt+F4.

## Environment and replay

`--check-uinput` opens and closes `/dev/uinput` and creates nothing; it refuses to run unless
`BARDO_KEY_REMAP_ALLOW_UINPUT_PROBE=1` is set.

The daemon reads its environment once, at start:

| Variable | Meaning | Default |
| --- | --- | --- |
| `NOTIFY_SOCKET`, `WATCHDOG_USEC` | systemd's `sd_notify` socket and watchdog interval | none |
| `RUNTIME_DIRECTORY` | where `key-remap.sock` lives | `/run/key_remap` |
| `BARDO_KEY_REMAP_CONFIG_DIR` | where `key-remap.toml` is read, only if root-only (a file in a test's scratch folder is not used) | `/etc/key_remap` |
| `BARDO_KEY_REMAP_VENDOR_DIR` | where the desktop's shipped `key-remap.toml` is read, under the one above; tests point it at a folder that is not there | `/usr/lib/key_remap` |
| `BARDO_KEY_REMAP_SEAT_FILE` | logind's seat state file, for the active uid | `/run/systemd/seats/seat0` |
| `BARDO_KEY_REMAP_SYS_INPUT`, `BARDO_KEY_REMAP_DEV_INPUT` | where nodes are listed and opened | `/sys/class/input`, `/dev/input` |
| `BARDO_KEY_REMAP_UDEV_DATA` | where udev's database says which seat a node is on | `/run/udev/data` |
| `BARDO_KEY_REMAP_GRAB_VIRTUAL` | one virtual device, by exact name, that may be grabbed (see Safety rules) | none |
| `BARDO_KEY_REMAP_DEVICE_SERVICE` | the socket of the service that makes the virtual devices; when it is absent the key remap daemon opens `/dev/uinput` | `/run/bardo/input-broker/input.sock` |
| `BARDO_KEY_REMAP_STEERING` | `seat-user` makes every program of the seat's user the shell; for tests whose client is in no unit, like `BARDO_KEY_REMAP_GRAB_VIRTUAL` | the shell's unit only |
| `BARDO_KEY_REMAP_ALLOW_UINPUT_PROBE` | lets `--check-uinput` open the node | none |

The path variables exist so a test can run the real binary against scratch directories.

`key_remap replay FILE` feeds a script through the remapper with the built-in profiles (and
`--config FILE` laid over them as the user layer) and prints each input with what it writes.
It never touches a device. A script has one input per line; `#` starts a comment:

```
<ms> press|release|repeat <dev> <apple|pc> <keyname>
<ms> focus <profile>
<ms> gone <dev>
<ms> release-all
<ms> resume
<ms> led caps|num|scroll on|off
```

`<dev>` is a number naming a keyboard, `apple` or `pc` its layout, `<keyname>` a name from the
key table (`leftmeta`, `c`, `left`). `led` is the desktop setting a LED of the virtual keyboard;
with the Caps Lock key remapped, a `led caps on` nothing in the script asked for is answered by
a `+capslock SYN -capslock SYN` tap before the next typed key, as the daemon does. Output is `+key` for a press, `-key` for a release, `SYN`
for the sync, then `release-grabs` and `status paused` for the escape chord:

```
$ cat copy.txt
0 press 1 apple leftmeta
10 press 1 apple c
20 release 1 apple c
30 release 1 apple leftmeta
$ the key remap daemon replay copy.txt
0 press 1 apple leftmeta -> +leftctrl SYN
10 press 1 apple c -> +c SYN
20 release 1 apple c -> -c SYN
30 release 1 apple leftmeta -> -leftctrl SYN
```

## Manual checks

What CI cannot reach, waiting for the user and real hardware:

- A real Apple keyboard, and a Bluetooth keyboard reconnecting (a uhid device, not a fake).
- Install the unit and check the mapping at the login screen, in a real compositor session
  (sill's `Focus` messages, Cmd+C in a terminal and in a GUI app).
- That compositor gives the virtual keyboard the session's layout.
- A user switch under logind: `ACTIVE_UID` changing, the old owner no longer obeyed.
- Run keyd at the same time: the `Conflict` notification appears.
- The escape chord, then `Resume`.
- Bluetooth keyboard reconnect.
- Grab a keyboard (or restart the key remap daemon) while Caps Lock is on: its light matches the desktop's.
  With the Caps Lock key remapped, a lock nothing typed through the key remap daemon asked for is tapped off
  before the next key typed (the journal says `turned Caps Lock off`), and `key_remap status` shows
  `Caps Lock: ON` until then. The key remap daemon follows the lock from the LEDs the desktop sets on its
  virtual keyboard, so a lock the desktop reports a moment after the grab is caught too; one set
  with Shift+Caps Lock stays, also across a VT switch. The tap waits for a key because the
  desktop may not have opened the new virtual keyboard yet. `key_remap replay` plays it with
  `<ms> led caps on|off` lines.
- A receiver with a touchpad: the pointer keeps working while it is grabbed, and stops with
  the escape chord only because the device is given back.

CI covers the rest: the mapping through the kernel, the grab gate, unplug and `kill -9` mid-chord,
the escape chord and `Resume`, LED forwarding, a second grabber, the seat check, the watchdog and
the unit's restart and start-limit behaviour.

## Tests

Three tiers, each in its own CI job (`.github/workflows/ci.yml`).

1. **The gate** (`gate`). `cargo test --workspace --all-features` runs the unit tables and the
   core scenario suite: every v1 mapping in both profiles, Cmd held across a profile switch,
   unplug mid-chord, `ReleaseAll` and `Resume` after the escape chord, the escape chord, the
   grab gate with a key held, Apple vs PC layout, and a property test that every emitted press
   has exactly one release. Socket tests run on a private socket in a temp directory. Scripts
   are synthesised in `tests/support`. No device is touched.
2. **The device tests** (`devices`). `crates/key_remap/tests/devices` runs the built `key_remap` binary
   against a fake keyboard on `/dev/uinput` and reads the "the key remap daemon keyboard" it writes back from
   `/dev/input`, through the real evdev grab, the real uinput device and the real socket. The
   harness is the `key-remap-devtest` crate. Every test is `#[ignore]`d and asserts
   `BARDO_KEY_REMAP_DEVICE_TESTS=1`. On a Linux machine you may use as a test bed (they create and grab
   a keyboard for a few seconds each; the fake is grabbed, your real keyboards are not):

   ```bash
   sudo modprobe uinput
   cargo build -p key-remap-daemon --bins --examples
   cargo test -p key-remap-daemon --test devices --no-run     # build as yourself
   sudo -E env "PATH=$PATH" BARDO_KEY_REMAP_DEVICE_TESTS=1 \
     target/debug/deps/devices-<hash> --ignored --test-threads=1
   ```

   They need root (the nodes are `root:input`), run one at a time (one fake name, one daemon),
   and dump the daemon's stderr, the output device's event trace and the status history when a
   test fails. Set `BARDO_KEY_REMAP_TEST_ARTIFACTS=DIR` to keep those logs in files.
3. **The systemd tier** (`systemd`). `ci/systemd-test.sh` installs the release binary, the unit,
   the udev rule and the polkit rule on a runner whose PID 1 is systemd, adds a drop-in that
   sets `BARDO_KEY_REMAP_GRAB_VIRTUAL`, plugs in a fake keyboard and checks that the unit starts under
   its hardening, that `kill -STOP` makes the watchdog restart it, that six quick `kill -9`s end
   in `failed` (start limit), and that `reset-failed` and `start` recover. It installs files, so
   it refuses to run outside CI unless `BARDO_KEY_REMAP_CI_ALLOW_INSTALL=1`.
