# Architecture

The caller-identity library answers one question for a Linux desktop service: which program is on the other end of
this connection? It holds the caller by a pidfd, reads what `/proc` says of it afterwards, asks the
systemd manager about its unit when the table names one, and judges the result against a
root-owned table of the desktop's own programs. Consumers (a compositor, a portal backend, an
updater, a root input service) then say which roles a method accepts.

## 1. Crates and allowed edges

| Crate | Layer | Contents | Depends on |
| --- | --- | --- | --- |
| `caller-identity-core` | pure | ids, `Table` (TOML codec), `Facts`, `Standing`, `judge`, `Requirement`, the text parsers of `procinfo` and `ServiceDirectives`, and the seam over `/proc`: `ProcFs`, `Prober` (over a `Held` process), the ownership walk | serde, toml, thiserror |
| `caller-identity-linux` | sync I/O | `Pinned` (pidfd), `RealProc` (the real `ProcFs`; the fixture tree under `fake`), `TableFiles` (the loader), `unit_facts` (the unit files a service without a manager reads), `fake` | caller-identity-core, rustix, nix (`SO_PEERPIDFD` only), thiserror |
| `caller-identity-dbus` | async adapter | `BusPeers`: credentials, the manager's unit facts (`UnitSource`, `ManagerLink`), `require`, `owner_of`, `watch_owner`, `Offload` | caller-identity-linux, caller-identity-core, zbus 5, thiserror |
| `peers-callers` | adapter | the desktop's table from its files, the requirement the unit manager must meet, `peers_on(connection, table)` | caller-identity-dbus, caller-identity-linux |
| `seat-guard` | sync I/O | the guards of a socket every local account reaches: the seat's user, the shell check (built on `Requirement`), how a failed accept is handled | caller-identity-core, caller-identity-linux, rustix |
| `caller-roles` | pure | the role names that services of every layer name: shell, portal, portal frontend, agent tier, Settings | caller-identity-core |
| `caller-identity-dev` | dev only | `DevRows`: developer rows read from a file the person owns (section 8); never in a shipped program | caller-identity-core, caller-identity-linux |

The other role names are constants in the crate of the service that owns them (`roles` modules of
`account-wire`, `sync-bus`, `memory-dbus`, `updater-dbus`, `compositor-dbus`, `action-dbus`,
`input-broker-wire`, `compositor-ipc`), each checked against `packaging/dist/desktop-callers.toml` by a test of its own.
The five in `caller-roles` are named by services in layers below their owners, which may not depend on
them.

`caller-identity-core` reaches no bus, system-call layer, runtime or executor; `caller-identity-linux` reaches no bus or
executor (`cargo xtask check layers`). Every crate sets `unsafe_code = "forbid"`: the pidfd comes
from rustix.

## 2. Layout

```
caller-identity-core/src/{lib, ids, cgroup, procinfo, unitfile, facts, standing, requirement, table, judge, procfs, probe, walk, error}.rs
caller-identity-linux/src/{lib, pin, unix, procfs, owners, unit_files, tablefile, error, fixture, fake}.rs
caller-identity-dbus/src/{lib, who, owner, manager, offload, timeout, error}.rs
.github/workflows/ci.yml
```

`fixture.rs` and `fake.rs` exist only with feature `fake`.

## 3. The trust rules

A `Pinned` peer is a process held by a pidfd. Everything read from `/proc/<pid>` is read after
pinning, and the pidfd is checked to be still alive at the end (`Pinned::alive()` polls it: readable
means exited). A process cannot be replaced while it is alive, so the facts belong to the pinned
process. A peer gone before the facts are read is `Unverified { why: PeerGone }`.

How the pin is obtained:

| Source | Pin | Strength | Made by |
|---|---|---|---|
| bus, the daemon offers `ProcessFD` in `GetConnectionCredentials` | that fd, checked against the reported pid by its `fdinfo` | `AtConnect` | `caller-identity-dbus` |
| bus, no `ProcessFD` | `pidfd_open(ProcessID)` at once, then the credentials are asked again: same pid and user | `Reopened` | `caller-identity-dbus` |
| unix socket | `SO_PEERPIDFD`: the kernel's pidfd of the connector, checked against `SO_PEERCRED`'s pid by its `fdinfo` | `AtConnect` | `Pinned::of_stream` |
| unix socket, kernel before 6.5 | `SO_PEERCRED` pid, `pidfd_open`, `SO_PEERCRED` again | `Reopened` | `Pinned::of_stream` |
| a descriptor the caller holds | the fd, checked against the offered pid by its `fdinfo` | `AtConnect` | `Pinned::from_fd` |

`SO_PEERPIDFD` comes through nix (`sockopt::PeerPidfd`, feature `socket`): rustix 1.1 has no safe
wrapper for it and the crates forbid `unsafe`. A peer that exited before the look is `PeerGone` (the
kernel says `ESRCH`), never the process that took its pid. Ubuntu 24.04 (6.8), Fedora and Arch all have
the option; an older kernel gets the reopened pin. See FINDINGS 1.

The proof ladder (`Proof`, ordered `Dev < Cgroup < Exe < Unit < Full`; `Dev` is only what a developer row gives, see section 8):

1. `Cgroup`: pinned, the cgroup path is read and its leaf names a unit the table has. Reachable only
   when the image cannot be read. A same-user process can move itself into a delegated cgroup: a
   speed bump, never a boundary.
2. `Exe`: the image is read (`readlink /proc/<pid>/exe`, a ` (deleted)` suffix stripped and the file
   judged through the open `/proc/<pid>/exe` inode), the file and every directory above it are
   root-owned and not group or world writable (the walk stops at the first directory that is not;
   at most 32 components), and the path matches one of the row's `exe` patterns.
3. `Full`: `Exe`, and the unit is real: the manager says `MainPID == pid`, the fragment is under
   root-owned directories and is root-owned, every drop-in is root-owned, `Transient == false`,
   `LoadState == loaded`, and `ExecStart` is one of the row's images. (`ExecStart` is matched
   against the row's patterns, not against the image: a unit may start a program through a stable
   symlink such as `/opt/bardo/current/bin/x` while the running image is the resolved release path.)
   Nothing is taken from a transient unit name or from an `app-*.scope` name for a service role.

The rules of `judge`, in order: a sandbox makes the peer `Sandboxed` and never a service; a hidden
image is a service only at `Cgroup`; an image or a directory anyone else can write is `Unverified`;
a root-owned image of a row with a unit, running in that unit's cgroup, is that service; a
root-owned image of a row without a unit is that program (`Exe` ceiling); anything else root-owned
is a `Program`. The scope name of a launcher (`app-<id>-<n>.scope`) is only a **claim**, carried in
`Program.scope_app` for display and never used to authorise.

Sandboxes: a Flatpak is read from `/proc/<pid>/root/.flatpak-info` and accepted only if the cgroup
also holds `app-flatpak-<escaped id>-<n>.scope`; a disagreement is `Unverified { SandboxDisagrees }`.
A Snap is read from the bus's `LinuxSecurityLabel` (`snap.<name>.<app>`). A sandboxed peer's image
path belongs to another mount namespace, so it is not read: its exe is `Hidden`.

## 4. The table

`packaging/dist/desktop-callers.toml` is the sample (and this desktop's) table, installed root-owned at
`/etc/bardo/desktop-callers.toml` with additions in `/etc/bardo/desktop-callers.d/*.toml`.
`TableFiles::load` merges the main file first and the drop-ins after it in name order; the first
row for a unit wins, so a later file adds programs and cannot take over a unit. A file is refused
whole unless the file and every directory above it are root-only. The format is refused whole for:
an unknown key, a row without `exe`, a relative or `..` path, `**` or two `*` in a segment, a
malformed id, two rows for one unit, more than 256 rows or 256 KiB. **the caller-identity library reads no user file** (`caller-identity-dev`, section 8, is the one exception).

## 5. API shape

- `caller-identity-core`: `Pid`, `Uid`, `UnitName`, `AppKey`, `RoleName`, `ExePath`, `ExePattern`,
  `CgroupPath`, `Facts`, `Table`, `Row`, `Standing`, `Proof`, `Requirement`, `Refused`,
  `judge(&Facts, &Table) -> Standing`, `Standing::holds(&[Requirement])`.
- `caller_identity`: `Pinned::{of_stream, from_pid, from_pid_with_uid, from_fd, alive, pidfd}`,
  `Prober::{probe, stand, stand_stream}`, `ProcFs`, `RealProc`, `TableFiles`.
- `caller-identity-dbus`: `BusPeers::{who, require, require_or_deny, owner_of, watch_owner}`,
  `ManagerCheck`, `UnitSource`, `ManagerLink`, `Offload` (an enum: `Inline`, `Tokio`), `access_denied`,
  `BUS_TIMEOUT` (2 s on every bus call).

No `bool` fields or parameters; enums that will grow are `#[non_exhaustive]`; errors are typed; a
library function reads no environment and no fixed path.

## 6. Recipes

- **Guarding a bus method:** build `BusPeers` once, then in the method
  `peers.require(sender, &[Requirement::new(caller_roles::SHELL, Proof::Full)]).await.map_err(access_denied)?`.
  The manager is asked only if a requirement wants `Full`.
- **Guarding a socket:** `Prober::stand_stream(&stream, &table)` right after `accept`, in the same
  loop turn, and never look again (a client may hand its socket on and exit).
- **A name that must be owned by a particular program:** `watch_owner` first, then `owner_of`;
  end the open work when the watch fires.
- **Testing a consumer:** enable `fake`; `BusPeers::introduce(unique_name, standing)` or build a
  `FixtureProc` tree and pass it to `BusPeers::with_proc`.
- **Adding a role:** add a row to the table and a constant in the `roles` module of the crate that owns
  the service (or in `caller-roles` when services below the owner's layer name it); that crate's role
  test checks the constant is a row of the shipped table.

## 7. Residual risks

1. Same-user persistence in the user's systemd configuration or environment (README).
2. A native program is known by its root-owned image; `LD_PRELOAD` and ptrace ride on its standing.
3. `Pin::Reopened`.
4. A client that connects, hands its socket on and exits before the first look is `Unverified`.

## 8. Developer rows (`caller-identity-dev`)

A checkout-built stand-in has a user-writable image and no real unit, so it is `Unverified` by rule 3
and tops out below `Full`. The desktop's developer and acceptance scripts need a seam, and it must
not exist in a shipped build. The seam is the crate `caller-identity-dev`, which no shipped program
depends on (the desktop gate greps `cargo tree` for it) and which the consumers reach through a feature of
their own (`debug`, `dev-callers`, `testing`):

| Piece | Where | What |
|---|---|---|
| `DevRows::load(path, person)` | `caller-identity-dev` | a plain file (no link), owned by `person` or root, not group- or world-writable, within `TABLE_FILE_CAP`, that parses as a table |
| `DevRows::added_to(shipped)` | `caller-identity-dev` | the rows added to a table through `Table::with_dev_rows` |
| `Table::with_dev_rows(rows)`, `Row::is_dev` | `caller-identity-core` | marks the rows; they come after every shipped row, never displace one, and a shipped unit is never taken, in either merge order |
| `Proof::Dev` | `caller-identity-core` | lowest proof, below `Cgroup`. The variant is always there because `judge` grants it to a developer row |
| `judge` rule 2b | `caller-identity-core` | a native process whose image matches a developer row (and whose cgroup leaf is the row's unit, when it has one), and that no shipped row claims by image, is that service at `Dev`; before rule 3 |
| `Requirement::dev(role)` | `caller-identity-core` | exactly `Proof::Dev` for `role`; a consumer lists it beside its real requirement in its debug builds, so the real service is let in by the real requirement and a stand-in by this one, and no proof between them is let in |

Rules that keep the seam from widening: only shipped rows are consulted for `Cgroup`, `Exe`, `Unit` and
`Full` (a developer row never names a unit or a program for those); a sandboxed process never
reaches the rule; a hidden image never matches. `Dev` meets no requirement of `Cgroup`, `Exe` or
`Full`. Rows from a person-owned file let that person's programs take a service's role; this is
acceptable only in a program's own debug builds.
