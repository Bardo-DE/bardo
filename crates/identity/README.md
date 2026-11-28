# The caller-identity library

Which program is calling: pinned process identity for Linux desktop services (pidfd, systemd unit, sandbox), judged against a root-owned table.

A service that does something only the desktop's own programs may ask for (take a screenshot, create an input device, install an update) needs to know who is on the other end of its socket or its bus message. A process id is not an answer: it can be reused, and the name of the cgroup a process sits in can be set by the process itself. The caller-identity library holds the caller by a pidfd, reads what `/proc` says of it afterwards, optionally asks the systemd manager about its unit, and judges the result against one table that says which program is the shell, the portal, the updater and so on.

Licensed under either of MIT or Apache-2.0, at your option.

## Crates

| Crate | Layer | Contents |
|---|---|---|
| `caller-identity-core` | pure, no I/O | ids, the caller table (TOML), the facts read about a process, `judge`, `Requirement`, `Standing`, and the seam over `/proc`: `ProcFs`, `Prober` over a `Held` process, the ownership walk |
| `caller-identity-linux` | sync I/O | `Pinned` (pidfd), `RealProc` (the real `/proc`; a fixture tree under feature `fake`), the table loader, the unit files a service without a manager reads |
| `caller-identity-dbus` | async adapter | `BusPeers` over zbus: the sender's process, the unit manager's facts, `require`, `owner_of`, `watch_owner` |
| `peers-callers` | adapter | the desktop's table from its files, the unit manager's requirement and `peers_on` for every service that asks over a bus |
| `seat-guard` | sync I/O | the guards of a socket every local account reaches (the seat's user, the shell check) |
| `caller-roles` | pure | the role names that services of every layer name (shell, portal, portal frontend, agent tier, Settings); the other roles are constants in the crate of the service that owns them |
| `caller-identity-dev` | dev only | developer rows (see below); never in a shipped program |

```rust
// In a bus method: who may call this?
let need = [Requirement::new(caller_roles::SHELL, Proof::Full)];
let caller = peers.require(header.sender().map(|s| s.as_str()), &need).await
    .map_err(caller_identity_dbus::access_denied)?;
```

## What a proof is

A caller is first held by a pidfd, and everything read from `/proc/<pid>` is read after that. The result is a `Standing`:

- `Service`: a row of the table matched, at a proof level: `Cgroup` (the cgroup names the unit; a speed bump, never a boundary), `Exe` (the image is root-owned all the way up and matches the row) or `Full` (and the systemd manager confirms the unit: root-owned definition, no user-writable drop-in, not transient, this process as its main process).
- `Sandboxed`: a Flatpak (its metadata, cross-checked against its cgroup's scope) or a Snap (its kernel label).
- `Program`: a root-owned image that is in no row.
- `Unverified`: nothing that can be remembered (an image the user can write, a hidden image, a caller that left).

The scope name of an application launcher is only ever a claim, carried for display; it never authorises anything. Roles come from the table and from the code that names the roles it accepts, never from a peer.

What this stops: a same-user process that does not touch the user's systemd configuration or environment cannot pass as `bardo-shell.service` (`systemd-run -u`, a moved cgroup, a copied binary, a user-level unit override, a drop-in). What it cannot stop: a same-user attacker who edits `~/.config/systemd/user` or `systemctl --user set-environment LD_PRELOAD=...` and waits for the next start of a trusted service (persistence in the user's own manager; the boundary CASW-05 names). Hardening for our own units: a drop-in under the user's directory makes `Full` fail rather than pass, and so does a unit that does not unset the loader variables.

Our units (`bardo-shell.service`, `bardo-compositor.service`, `bardo-portal.service`, `bardo-mouse-gestures.service`) carry the `UnsetEnvironment=` line documented on `LOADER_VARIABLES` (the dynamic loader's variables and the library search paths of GLib, GTK, GStreamer, Mesa, EGL, Vulkan and friends), which systemd applies after the manager environment, `Environment=` and `EnvironmentFile=`. A unit without it never proves `Full`.

Other limits, stated rather than hidden:

- A native program is known by its root-owned image. Anything that can run the same image with `LD_PRELOAD` or ptrace (Yama scope 1 lets a parent trace its child) rides on that program's standing.
- `Pin::Reopened` (a pidfd opened by pid, then the pid checked again) leaves one window: the connecting process died and its pid was reused before we looked. The bus daemon's own pidfd (`ProcessFD`) and `SO_PEERPIDFD` close it; this release uses both where the bus and the kernel (6.5) offer them.
- A client that connects, hands its socket to another process and exits before the first look is `Unverified`.

## The table

`packaging/dist/desktop-callers.toml` is a sample of the format and of this desktop's own table. A row has `app`, `exe` (a non-empty list of absolute patterns; one `*` per segment, never `**`), `role`, and optionally `unit` and `name`. A row with a `unit` is a service; a row without is a program matched by its image alone, whose ceiling is `Exe`. The loader refuses a file that is not owned by root alone, or whose directory anyone else can write. The caller-identity library reads no user file: nothing a person's file says makes a process more trusted. (The one exception is the crate `caller-identity-dev` below, whose rows reach `Proof::Dev` and no higher.)

## Developer rows

Scripts that run a checkout-built stand-in (an accounts stand-in, a router stand-in, an app built in the working tree) cannot be a service in the table: the image is one the person can write and there is no real unit, so `judge` calls it `Unverified` before it looks at any row. Every privileged call to it from such a script is refused. The crate `caller-identity-dev` gives those scripts a seam that cannot exist in a shipped build.

**The risk, plainly: rows read from a file a person owns let that person's programs take a service's role. No shipped program may depend on `caller-identity-dev`.** The consumers reach it only through a feature of their own that is off by default, and the desktop's gate refuses the crate in every release shape, as it does for `fake`.

- `DevRows::load(path, person)` reads such a file: a plain file (a link is refused), owned by `person` or root, not writable by group or others, within the table file's size cap, that parses as a table. `DevRows::added_to(table)` adds the rows through `Table::with_dev_rows`.
- `Table::with_dev_rows` puts developer rows after the shipped ones. A developer row never replaces a shipped row and never holds a unit the shipped table names, whichever table is merged into which.
- `judge` gives a native (never a sandboxed) process the standing `Service` with `Proof::Dev` when its image matches a developer row (and its cgroup leaf is the row's unit, when the row names one), checked before the user-writable rule. A process whose image a shipped row claims is judged by the shipped rows alone. A developer row never gives `Cgroup`, `Exe`, `Unit` or `Full`. The `Dev` variant is always part of `Proof` because `judge` grants it; a table without developer rows never yields it.
- `Proof::Dev` sorts below `Proof::Cgroup`, so no requirement of `Cgroup`, `Exe` or `Full` is met by it. A consumer accepts it only by asking: `Requirement::dev(role)`, which is exactly `Proof::Dev` for the role, listed next to its real requirement in its own debug builds and never in a release build. The real service meets the real requirement beside it; no proof between the two is let in.

## Building

`cargo test --workspace --all-features` needs `dbus-daemon` on the PATH (the bus tests start a private one). The Linux kernel must have `pidfd_open` (5.3). `unsafe` is forbidden in every crate; the pidfd comes through rustix, and `SO_PEERPIDFD` through nix.

## Tests and features

| Feature | Crate | What it does | Shipped? |
|---|---|---|---|
| `fake` | `caller-identity-linux`, `caller-identity-dbus` | `Pinned::introduced`, the fixture tree, `BusPeers::introduce` | never |
| `tokio` | `caller-identity-dbus` | `Offload::Tokio` and zbus on tokio | optional |
