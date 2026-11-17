# packaging

How the bardo tree becomes a desktop: the installer, the session files, the signed release bundle, the test jail and
the whole-desktop gate. No Rust, no product code. The programs are the crates of this tree (`crates/<name>`), one
cargo workspace with one target directory; there are no other repositories to check out.

| Path | What |
|---|---|
| `install/plain.sh` | `cargo xtask install`: builds every part as you and installs it with one sudo, a line per part (see Installing) |
| `install.sh` | installs the desktop: builds what is selected in one cargo run, then runs each component's own installer (`crates/<name>/...`) in order (callers, accounts, inference, sync, models, memory, actions, computer-use, settings, compositor, shell, portal, input-broker, key-remap, files, mouse-gestures, session; callers, input-broker, key-remap and mouse-gestures are this folder's `install/system.sh`) and the colour emoji font; `--check` lists what the machine lacks with the exact dnf, apt or pacman line; see below and `docs/distros.md` |
| `docs/distros.md` | Ubuntu 24.04+, Fedora and Arch: what is checked, the install lines, picking the session at login, the Ubuntu AppArmor note |
| `docs/release-notes.md` | the notes for the release being prepared, in plain words (what a person sees change); the text for the release's notes |
| `install/` (helpers) | `distro.sh` (family from os-release, sudo/doas/pkexec), `packages.sh` (the one package table), `check.sh`, `cargo.sh` (builds the tree from `/` with `--manifest-path`, so the system linker is used; the part to cargo-package table), `fonts.sh` |
| `dist/` | `desktop-callers.toml` (the table of the desktop's own programs, one row per role, installed as `/etc/bardo/desktop-callers.toml`; the update service, the screen-sharing service and the input service refuse every caller without it) and `bardo-key-remap.toml` (bardo-key-remap's vendor file, installed as `/usr/lib/bardo/key-remap/key-remap.toml`: Caps Lock switches the input method, Shift+Caps Lock is the real Caps Lock) |
| `install/system.sh` | `callers`, `bardo-input-broker`, `bardo-key-remap` and `bardo-mouse-gestures` parts: the table; the input service (program, socket, service, the rule that makes `/dev/uinput` root-only, the module list) and the keyboard map's program, service, restart rule and `/usr/lib/bardo/key-remap/key-remap.toml` (replaced each time; `/etc/bardo/key-remap/key-remap.toml` stays the administrator's and is never touched). Never runs sudo, prints the root command; `--remove PART` takes back only what is unchanged. It replaces bardo-key-remap's own installer, which would give the `input` group the device file and the service a drop-in to open it. The `bardo-mouse-gestures` part is the touchpad service, a system service like bardo-key-remap (a dynamic user; its program, its unit, the rule that gives the mouse's raw node to the `input` group and to no person, the module options); it replaces bardo-mouse-gestures's own installer, and its settings are `/etc/bardo/mouse-gestures/gestures.toml` |
| `gate/check-callers-table.sh`, `gate/check-no-fake.sh`, `gate/check-no-uinput-access.sh` | the table has a row for every role of peephole's list and the units' paths agree with it; no build command or crate turns the `fake` feature on; nothing shipped opens `/dev/uinput` to the person or a group. `gate/input-checks-selftest.sh` tests the three on generated trees; all four run in `gate/desktop-gate.sh --self` |
| `fonts/` | `Noto-COLRv1.ttf` (OFL) with its source and sha256: Ubuntu and Arch ship a bitmap emoji font our renderer cannot draw |
| `gate/distro-check.sh` | the family detection, package table, escalation, the cargo helper and `--check` against made-up machines |
| `harness/jail.sh` | the one jail: a bubblewrap namespace with no input devices, no GPU, no session, no network |
| `harness/lib/` | `require_jail`, step timing and exit codes, budgets, run ids, a private dbus-daemon |
| `harness/scenario-build.sh` | outside the jail: builds what the scenarios run (bardo-compositor tools, foot, bardo-shell, layer-shell-host's demos, bardo-settings, the account daemons and their rigs, bardo-portal) from the tree into the git-ignored `build/`; the gate runs it first |
| `harness/selftest.sh` | proves the jail holds (`--all` also runs the negative check outside) |
| `gate/desktop-gate.sh` | `--self`, `--merge`, `--phase P<n>`: prints `DESKTOP GATE GREEN (<tier>)` |
| `session/` | the "bardo-shell (bardo-compositor)" login: `bardo-session` (bardo-compositor and bardo-shell as the user units `bardo-compositor.service` and `bardo-shell.service`, the session's arguments passed through the user manager's environment, never a drop-in; the portal is bardo-portal, started by D-Bus activation, `XDG_CURRENT_DESKTOP=bardo`, and the crash-loop guard: three quick exits, then `bardo-compositor --safe`, then the session ends with a guard log line that also names `pkexec /usr/libexec/bardo/bardo-updater-rollback` as the way back by hand; the log is kept in `$XDG_STATE_HOME/bardo-session/guard.log`, and before the first bardo-compositor starts the session runs `bardo-updater recover`, which puts the previous version back when the latest update failed to start in several restarts of the computer; your other session stays available at the login screen), the wayland-sessions entry, `bardo-portals.conf`, `bardo-session.target` (which also wants `bardo-updater.service`, the update service, so its daily check runs in every login) |
| `release/` | signed release bundle of the whole desktop: `build.sh` (components built from this tree, staged under `/opt/bardo/current`), `pack.sh`, `verify.sh`, `system-files.list` (the system files a release may place, bardo-updater's list), `check-helpers.sh` (bardo-updater's three privileged programs and its polkit file are all in place), `health-scenarios.sh` (the start check: good logins keep a release, failed starts put the previous one back), `selftest.sh` (in `gate/desktop-gate.sh --self`), `README.md` (what to set up once and how to cut a release); built by `.github/workflows/release.yml` and published to the public PoHsuanLai/desktop-releases |
| `install/` | `install.sh` and `uninstall.sh` for `session/` (`--dry-run`, `--prefix`, `--destdir`, `--sysconfdir`, `--sessions-dir`, `--tree`); it also puts `bardo-compositor.service` and `bardo-shell.service` (from the bardo-compositor and bardo-shell crates) root-held in `<sysconfdir>/systemd/user`, where the account service trusts them as the compositor and the shell; never sudo, never `$HOME`; the uninstall removes exactly the files the install wrote, and only while they are unchanged |
| `gate/installer-check.sh` | `install.sh` against stub installers in a scratch dir: the dry-run plan and order, PREFIX/DESTDIR passed through, stop at the first failure, no sudo, nothing written to the home |
| `gate/shell-accept-skips.txt` | the scripts of bardo-shell's `dev/accept.sh --compositor compositor` that may SKIP in the phase gate, with the closing phase |
| `gate/expected-skips.toml` | scenarios not runnable yet, each with a finding id and the closing phase |
| `scenarios/` | whole-desktop scenarios; each calls `require_jail` first and exits with the plan's codes |
| `harness/lib/accounts.sh` | what the `accounts-*` scenarios share: the real bardo-accountd, bardo-syncd and bardo-inferd, the accounts' test rig (`account-rig-servers`, `-client`, `-secrets`) and ferry's (`sync-rig-servers`: the OAuth issuer and the Graph fake, `sync-rig-client`: an app talking to bardo-syncd) on bardo-compositor's private bus with bardo-shell drawing the sheets and bardo-settings as Settings; the caller identities (a fixture /proc for the daemons' `test-proc-root` builds); the secret scan (`secret-scan.py`) over a `busctl monitor` record |
| `harness/lib/regress.sh`, `pixels.py`, `sheet-bus.py` | what the `accounts-regress-*` scenarios share: checks that fail a step (`must`), the sheet's traffic read from the recorded bus (what bardo-accountd shows it, what the keys sent back), and measurements of a capture (the card's box, equal pixels, dark text, contrast). bardo-shell's surfaces write no debug probe (layer-shell-host does not run ui-blitz's), so the sheet is read this way; bardo-settings's controls are read by role and name |

## Installing

Clone the bardo tree and run `cargo xtask install` (rustup is needed; `cargo xtask install --check` says what else this machine lacks). `cargo xtask` runs from anywhere in the tree:

```
git clone https://github.com/Bardo-DE/bardo.git && cd bardo && cargo xtask install
cargo xtask install bardo-shell bardo-settings   # only these parts
cargo xtask install --build       # build only, as you
cargo xtask install --check       # what this machine lacks
cargo xtask install --parts       # every part, in install order
```

The first line builds everything as you in one cargo run, then installs it (sudo asks once). PREFIX (default `/usr/local`) comes from the environment.

One line per part; the whole output is in `~/.local/state/desktop-install.log`, and a failure prints
its last lines. The build is one `cargo build --release --locked -p ...` for every program of the parts you named,
at the root of the tree, into `<tree>/target`. `install/plain.sh` holds which packages each part needs
(`part_packages`, in `install/cargo.sh`) and how each part is installed (its own installer, run with `--no-build`
and `CARGO_TARGET_DIR`, so nothing builds as root). `PREFIX` (default `/usr/local`) and `BARDO_TREE` (default the parent of this folder) can be
set in the environment.

`install.sh`, below, is the packaging path (`--destdir`, `--sysconfdir`, a staged tree) and `--check`.

`./install.sh [--dry-run] [--prefix DIR] [--destdir DIR] [--sysconfdir DIR] [--tree DIR] [--no-build] [--only NAME,...]`
builds once, then runs each component's own installer (`crates/<name>/...`), in this order: callers
(`install/system.sh callers`: `/etc/bardo/desktop-callers.toml`, first because the update service and the portal backend read it when they start and refuse every caller without it), accounts
(`scripts/install.sh`: the account daemon and the caller table), inference (`scripts/install.sh`: bardo-inferd, the AI broker, its unit and caller table), sync (`scripts/install.sh`: the sync daemon bardo-syncd, the helper that shows drives in `~/Cloud`, their units and the activation file; no `--sysconfdir`), models (`scripts/install.sh`: the model catalogue bardo-inferd reads and the engine installer's helper), memory (`scripts/install.sh`: the memory daemon, its unit
and `/etc/bardo/memoryd-callers.toml`), actions (`scripts/install.sh`: the action router, the companion, the reader, the tool
server and `bardo-do`, with their units and the `/etc/bardo/*-callers.toml` tables; they read the accounts, so they come
after it), computer-use (`scripts/install.sh`: the helper that lets the assistant use apps that are not ours, its unit and
`/etc/bardo/computer-use-callers.toml`; it needs at-spi2-core at run time), settings (`scripts/install.sh`: Settings, which
the accounts' caller table knows as the unit `org.bardo.Settings.service`), compositor (`scripts/install.sh`: the compositor and its user unit), shell (`dist/install.sh`), portal
(`scripts/install.sh`: the portal backend for screen sharing and screenshots, started by D-Bus activation), input-broker
(`install/system.sh input-broker`: the input service, the only maker of virtual keyboards and pads, with `/dev/uinput` closed to everybody
else), key-remap (`install/system.sh key-remap`, not its own installer), files, mouse-gestures (`install/system.sh mouse-gestures`, not its own installer: the touchpad service, a system service like key-remap), then the session itself (`install/install.sh`: `bardo-session`, the user target, the login entry; it never runs sudo and prints the root command when a place is not writable). `--prefix` (default `/usr/local`) and `--destdir` are passed through in the form each
installer takes; the first failure stops the run and names the component and what was installed before it.
`--dry-run` prints the plan and runs and builds nothing.

It never runs sudo and never writes under `~/.config` or `/etc` itself. A component runs only when every place
it writes is writable by you (a user prefix, or a `--destdir`); the rest are printed as "yourself", with the
command, to run with the right privileges: a system prefix, shell (fixed destinations, its own
sudo or home files), callers, input-broker, key-remap and mouse-gestures outside a `--destdir` (`install/system.sh`, which you run with sudo), models outside a `--destdir` (its installer asks for sudo itself), and memory, actions and computer-use outside a `--destdir` (their caller tables go to the
fixed `/etc/bardo`; their installers refuse to run as root and ask for sudo for just those steps, so the command is
printed to run as yourself). Neither takes `--sysconfdir`. `gate/installer-check.sh` tests all of this against stub
installers.

Nothing that can see input devices, a GPU output or the user's session runs outside
`harness/jail.sh`. Exit codes: 0 pass, 1 test failed, 2 could not run safely (a SKIP, fails unless
listed), 3 environment incomplete, 4 budget overrun, 5 jail breach.

The lane gate for the tree is `~/rs-wt/gate-lane-jailed.sh <tree dir>` (it hands the bardo tree to its own gate): fmt, clippy
and the nextest archive build outside, the tests inside `harness/jail.sh`.

Conventions: the bardo-compositor-scope plan (`~/rs-wt/compositor-scope/20-FREEZE-PLAN.md`) sections 3, 4.14
and 6; shell code follows the style of `crates/shell/dev/lib`.
