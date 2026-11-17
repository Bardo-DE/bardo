# Findings

## Release bundle (lane desk-release, 2026-10-15)

`release/build.sh` stages the whole desktop under `/opt/bardo/current` with a destdir. Where a component's
installer could stage, it is used (the accounts service, the Settings app, anyview, mailo, and this repo's session installer). The
rest is staged by build.sh itself, marked INTERIM in the script, because the component's installer cannot
stage without sudo or writes `$HOME`. What each installer must gain so the INTERIM block can go (all
`--prefix DIR --destdir DIR`, no sudo, nothing under `$HOME` or `/etc` outside the destdir, `--no-build`
and `--locked` builds, the installed path (never the destdir-prefixed one) inside any unit or service file):

- The compositor: no installer at all. Needs one that installs `bin/compositor`, `bin/bardo-compositorctl` and
  `dist/bardo-compositor.service` with `ExecStart=<prefix>/bin/compositor` (the unit says `/usr/bin/compositor`; build.sh
  rewrites it).
- The shell (`dist/install.sh`): fixed `/usr/local`, `/etc/pam.d/shell`, `~/.config/bardo/key-remap`, sudo, and a build
  inside the installer. Needs `--prefix/--destdir/--no-build/--bin`, no key remap daemon layer copy into the home
  (split the per-user copy into the session), and a place for the PAM service.
- The key remap daemon (`install.sh`): stages under DESTDIR but at `/etc` and `/usr/local`, not under a prefix, and builds
  unlocked. Needs `--prefix` so the binary, the unit (with `ExecStart` for that prefix), the udev and
  polkit rules go under it.
- The mouse gestures daemon: SUPERSEDED 2026-10-17. It is a system service like the key remap daemon; `install/system.sh mouse_gestures` places it and
  build.sh stages its unit as a system file (see "the mouse gestures daemon as a system service" below).
- mailo (`dist/install.sh`): `MAILO_PREFIX` as a destdir writes the destdir-prefixed path into
  `org.bardo.Mail.service` (build.sh seds it out). Needs a real `--destdir` beside the prefix.
- The accounts service (`scripts/install.sh`): fine, but builds without `--locked` (build.sh builds locked first and passes
  `--no-build`); the top-level `install.sh` has no way to pass `--no-build`, so build.sh calls the accounts service directly.
- anyview (`dist/install.sh`): builds the mpv plugin's C plugin and fetches mpv-wgpu without `--locked`
  (build.sh builds it locked and passes `ANYVIEW_MPV_CPLUGIN`); `--mpv PATH` cannot be passed through the
  top-level `install.sh`, so the plugin manifest names the builder's `mpv` or `/usr/bin/mpv`.
- The updater: its `scripts/install.sh` installs the helper, polkit action, unit, bus file and release key on a
  machine (release/bootstrap.sh runs after it). The bundle carries only `bin/updater`, built from the
  manifest pin; `bardo-updater-apply` is deliberately outside the release. build.sh's staging of `bin/updater`
  is INTERIM until the updater has a stageable installer for the user program.
- The portal: named in the brief, but no repo beside this one and no entry in `manifest.toml`; not built.
  Add it to `ALL` and a `build_portal` in `release/build.sh` when it exists.

What the bundle exports and places (lane distro-slip, 2026-10-16; updater-core's allow-lists are the same
lists as `release.py`'s):

- Links (`[[export]]`, derived from the staged tree): everything staged under `share/wayland-sessions`,
  `share/xdg-desktop-portal`, `share/dbus-1/services`, `lib/systemd/user`, `share/applications`,
  `share/polkit-1/actions`, and now `share/bardo` (intents, skills, settings schemas, helpers, icons),
  `share/accounts` (providers, client tables), `share/models` (the model catalog) and `share/fonts` (the desktop
  lane ships `fonts/Noto-COLRv1.ttf` to `share/fonts/bardo/`). Commands are `exports.toml` links in `/usr/local/bin`.
  A distribution's own file at the same path is a conflict and is left alone.
- Files (`[[system]]`, staged in `system/<absolute path>`): `/etc/pam.d/shell`, the key remap daemon's `bardo-key-remap.service` (at
  `/usr/lib/systemd/system`), `70-key-remap.rules`, `key_remap.polkit.rules` and `modules-load.d/key-remap.conf` (`uinput`,
  written by build.sh because the key remap daemon ships none), the mouse gestures daemon's `70-mouse-gestures.rules` and `modprobe.d/99-mouse-gestures.conf`,
  and the accounts service's `/etc/accounts/callers.toml` (the accounts service's installer is pointed at `<prefix>/system/etc`).
- Still data only (no home on the system): `share/shell/key-remap.toml` (the shell session's key remap daemon layer, which the shell's
  installer copied into the home), `share/key_remap/key_remap.plasma.toml` (the Plasma overlay; not used by this
  desktop).
- Not staged yet: the shell's DDC/CI udev rule and `i2c-dev` module list (`dist/udev`, `dist/modules-load.d`; the shell's own
  installer leaves them out on purpose, ddcutil's package ships the rule on Fedora; audit D7 decides) and
  nothing of the key remap daemon's: its Caps Lock rule now ships as `/usr/lib/bardo/key-remap/key-remap.toml` (see pa2-desk).

Whether the release should write `/etc/pam.d` at all is the updater's open question 5; deleting the folder from
both lists (`ALLOWED_SYSTEM_DIRS`, `SYSTEM_DIRS`) turns it off.

Other choices: file modes are normalised to 0644 or 0755 (executable bit kept) so the bundle does not depend on
the builder's umask; the date is the packing time (`SOURCE_DATE_EPOCH` overrides it); `release/release.pub`
is committed by the owner and the CI refuses to publish without it.

## Distro audit (lane 5, 2026-10-16)

B3/D18: the workflow builds in `ubuntu:24.04` (was `fedora:44`), with `release/deps.sh debian build`, rustup with each
repo's pinned toolchain, and Zig 0.16.0 from ziglang.org with its SHA-256 pinned; `check-glibc.sh` and `check-ldd.sh`
run after every build. The smoke job installs the signed release with bardo-updater-apply in `ubuntu:24.04`, `fedora:44` and
`archlinux:latest`; publishing waits for it. B2: four more export folders (above). D17: the system step (above).
N2: `lib.sh` gives the install hint per family. Not run (no compiler, minisign or runner here): the workflow, the
package names marked `# unverified` in `deps.sh` (Ubuntu t64 and libdav1d names, Fedora libdav1d), the selftest's
new cases, and `smoke.sh`. The smoke runs the helper as root against `/etc/bardo/updater/config.toml` and the real
`/opt/bardo`, which is why it only runs with `--really` as root.

## Round 2 (release.py aligned with updater-core)

Applied from the updater's FINDINGS "Release pipeline": `.updater` reserved, mode rules (readable by owner, not
group/other writable), semver.org regex, real date parse, the five size limits in pack and verify (the
bundle-size and file-count limits are not forged in selftest: too big), DEL escaped in `toml_str`, prehashed
(`ED`) signatures required (`minisign -S -H`; needs minisign 0.11 or later), exports must be bundle files
(a folder target is refused by verify; pack derives only file exports). The README path is
`scripts/install.sh`. The updater is in `manifest.toml` (`[repo.updater]`, rev e8af08f4); the workflow's
`updater_ref` input is gone. polkit's action directory follows a link into /opt on a real machine only if
polkit does; confirm on a machine.

## desk-callers (2026-10-16)

The top-level `install.sh` now installs the memory service (the memory daemon, `/etc/bardo/memory-callers.toml`) and
The actions service (its four daemons, `bardo-do`, and the `/etc/bardo/*-callers.toml` tables), between the accounts service and detent.
Both are in `ORDER`, the `--only` help, the README list and `gate/installer-check.sh` (stubs, plan order, the
staged arguments, and the not-staged case).

What I found and decided:

- Neither installer takes `--sysconfdir`; `/etc/bardo` (and the actions service's `/etc/xdg/bardo`) are fixed. Not
  invented: the top-level declares those places as written, under the destdir, and passes only `--prefix` and
  `--destdir`. No `--distro`, `--sandbox` or `--no-build` is passed (nothing existing passes them either); the
  installers read the family from the machine, so Ubuntu, Fedora and Arch work. No linker is asked for.
- They do not fit the accounts service's "print the root command" path: both refuse to run as root ("Run this as yourself")
  and escalate (`$SUDO`, sudo, doas, pkexec) per step themselves. A root hint for them would fail. So without a
  `--destdir` they are listed as "yourself" with the plain command (the same way the shell and the key remap daemon are); with a
  `--destdir` they run here and never escalate. A destdir that you cannot write stops them with their own message.
- `release/build.sh` is not changed: it builds by name from a fixed list and neither is in it. Adding them
  needs the installers to stage `/etc/bardo` inside the prefix (a `--sysconfdir`, or put_system-style output for
  `memory-callers.toml` and the actions tables), because build.sh refuses files outside `/opt/bardo/current` and
  `release.py` allows system files only in listed folders. Ask the memory service and the actions service for `--sysconfdir`, then add
  `build_memory` and `build_actions` (build locked, `--no-build --bin-dir`). The manifest already lists both
  with gate rows; revs untouched.
- `install/uninstall.sh` is the session's alone; no top-level uninstall lists components.

## Release bundle: emoji font and monitor brightness (lane desk-callers, item 212, 2026-10-16)

- The session component of `release/build.sh` now stages `fonts/Noto-COLRv1.ttf` and the licence
  (`OFL-Noto-COLRv1.txt`, the name `install.sh` uses) at `share/fonts/bardo/`. `share/fonts` is an automatic export,
  so the bundle links `/usr/share/fonts/bardo/Noto-COLRv1.ttf`; Bardo's `emoji_file.rs` looks for
  `<data dir>/fonts/bardo/Noto-COLRv1.ttf` over `XDG_DATA_DIRS` (default includes `/usr/share`), and fontconfig
  reads `/usr/share/fonts` too. The bundle ships the font on every distro (Fedora's own copy is byte-identical,
  harmless); `install.sh` skips it where fontconfig already has one, which a bundle cannot test.
- The shell ships both files (`dist/udev/60-shell-ddc.rules`, `dist/modules-load.d/shell-ddc.conf`), and the updater's system
  folders (origin/master at b607414, `ALLOWED_SYSTEM_DIRS`) include `/etc/udev/rules.d` and `/etc/modules-load.d`, so
  both are staged with `put_system` at those paths (the same folders the key remap daemon uses). No updater follow-up needed. The
  local ~/bardo/updater checkout is five commits behind origin (its link list still has 7 folders); `release.py`'s lists
  match origin. Same-tag duplicate with Fedora's `60-ddcutil-i2c.rules` is harmless (install.sh's "unless ddcutil
  has it" test is not reproduced).
- `pack.sh` needed no change: it lists whatever is staged. `release/selftest.sh` (the "release (pack, verify, hostile
  releases)" check) now also stages a font and a modules-load file and asserts the font link and the system entry.

## Release bundle: the memory service and the actions service (lane desk-callers, 2026-10-16)

- `release/build.sh` builds the memory service (`memoryd`) and the actions service (its four daemons, `bardo-do`, `agent-acp-server`; not the
  voice daemon) after the accounts service, locked, then runs their installers with `--bin-dir`, `--prefix /opt/bardo/current`,
  `--destdir <stage>` and `--sysconfdir /opt/bardo/current/system/etc`. The caller tables land as system files in
  `/etc/bardo`, the actions service's default settings in `/etc/xdg/bardo`. These steps need the memory service >= 6aade09 and the actions service >=
  e44112c (the `--sysconfdir` commits); the manifest revs are unchanged and the next bump brings them, so a release
  built before that fails in these two steps.
- Decision (owner, 2026-10-16): `/etc/xdg/bardo`, the XDG system-wide config folder, is an allowed system folder
  (the updater aa427f2, `ALLOWED_SYSTEM_DIRS`), and the updater leaves alone a file a person edited there. `release.py` and
  `release/README.md` carry the same list; `manifest.toml` moves only the updater's rev to aa427f2. The selftest accepts
  a file in `/etc/xdg/bardo` and refuses one at `/etc/xdg/other.toml` (now 25 hostile releases).
- `install/cargo.sh` gives the memory service the `net.git-fetch-with-cli` setting its own installer passes (the actions service's does not).
- Not exported: `bardo-do` and `agent-acp-server` stay under `/opt/bardo/current/bin` (no `/usr/local/bin` link in
  `exports.toml` yet), and the shell completions and interface XML ship in the bundle without links.
- The release workflow has a new job `agents` (the memory service and the actions service together; both need perl and make, which
  `release/deps.sh` already installs). Not run here; it fails until the manifest carries the memory service >= 6aade09 and
  The actions service >= e44112c.

## desk-pprefix (2026-10-16)

B1 is closed by the accounts service 2e5cd74 (in the pinned de06006), so `install.sh` no longer forces the accounts service to `/usr`: it gets
the desktop `--prefix` (default `/usr/local`) and `--sysconfdir` (default `/etc`) like the other components, and the
"accounts goes to /usr" note is gone. Checked in the accounts service's source at de06006, with `--prefix /usr/local --sysconfdir /etc`
(`scripts/install.sh` puts providers, `clients.toml`, the tailnet drop-in under `$prefix/share/accounts` and the caller
tables at `$sysconfdir/accounts`):

- providers, `clients.toml` and the model catalog: accountd, syncd and inferd take `Xdg::shipped("accounts/providers")`,
  `("accounts/clients.toml")` and `("models/catalog")` (`account-core/src/xdg.rs`): the first `$XDG_DATA_DIRS` entry
  (default `/usr/local/share:/usr/share`) that has the path, else `/usr/share/...`.
- `callers.toml` and `inferd-callers.toml`: `/etc/accounts/<file>` is searched first (`account-daemon-support/src/system_callers.rs`,
  `SYSTEM_CALLERS`, `INFERD_CALLERS`, `search`), so `--sysconfdir /etc` is found. Since 36f5014 the other places are a
  fixed list (`TABLE_DIRS`), not `$XDG_*`, and a table counts only when root holds it (root-owned, no group or other
  write along its real path), so it must be installed as root, as before.
- Not verified: that the user session's `XDG_DATA_DIRS` keeps `/usr/local/share` (unset gives the default; a session that
  sets it without that entry would not find an install under `/usr/local`). Nothing was built or installed;
  `gate/installer-check.sh` and a dry run passed.
- The model catalog was installed by nothing on this repo's master, so inferd had no model catalogue on a fresh install.
  `install.sh` now has a `models` step right after the accounts service (`scripts/install.sh --prefix P --sysconfdir S [--destdir D]`,
  The model layer 3ae1211; it builds nothing and writes only `<prefix>/share/models/catalog`). Outside a `--destdir` it is left to
  the person like the memory service (its installer escalates itself). `release/build.sh` has `build_models` (`--no-build`, staged
  under `share/models`, an auto-exported folder in `release.py`) and the CI `agents` job builds `memory,actions,models`.
  Not run: the CI job and `build_models` (no build here; the model layer installer's dry run lists the catalog files).
- A prefix other than `/usr` and `/usr/local` is on no search path the session has. The plan now prints (never writes)
  an `environment.d` line `XDG_DATA_DIRS=<prefix>/share:/usr/local/share:/usr/share` for the D-Bus service files and the
  shipped data. Units are a separate matter: systemd's user manager reads `/usr/lib`, `/usr/local/lib` and
  `$XDG_DATA_DIRS/systemd/user`, never `<prefix>/lib/systemd/user`, so `XDG_DATA_DIRS` alone does not find them; the note
  also prints a root `ln -sf <prefix>/lib/systemd/user/* /usr/local/lib/systemd/user/`. That link command and the
  systemd search-path claim come from systemd's documented behaviour and were not tried on a machine. The session
  installer's bardo-compositor.service check (`install/install.sh`) only tests that the file exists under the prefix, so it
  does not catch this.
- The release bundle is not affected: it lives under `/opt/bardo/current` but `release.py` exports `share/dbus-1/services`,
  `lib/systemd/user`, `share/accounts`, `share/models` and the other folders in `AUTO_EXPORT_DIRS` as links into `/usr/share`
  and `/usr/lib/systemd/user`, and the caller tables go to `/etc/accounts` as system files. `release/build.sh` is unchanged
  apart from the model step.

## desk-cua: the helper for apps that are not ours (B6, 2026-10-16)

- `install.sh` has a `cua` step right after the actions service (`scripts/install.sh --prefix P [--destdir D]`, cua f6fe8bc, the
  manifest rev). It builds the daemon `cuad` from source (release, locked) unless the programs are given, and writes
  `<prefix>/libexec/bardo/cuad`, `<prefix>/lib/systemd/user/cuad.service` with its `.service.d/sandbox.conf`, the two
  D-Bus activation files and the interface XML under `<prefix>/share/dbus-1`, the Settings schema under
  `<prefix>/share/bardo/settings`, and `/etc/bardo/cuad-callers.toml`. Read in full: nothing else is written, and
  `CONF_DIR` (`/etc/xdg/bardo`) is computed but never used.
- No `--sysconfdir` is passed, as for the memory service and the actions service: cuad reads `/etc/bardo/cuad-callers.toml` and the person's file
  only (`crates/cuad/src/callers.rs`), so a table placed elsewhere would be ignored. Outside a `--destdir` the step is left
  to the person, as themselves (the installer refuses root and escalates per step). `gate/installer-check.sh` checks the
  stub, the order, the arguments, the no-`--sysconfdir` rule and the left-to-you case.
- Build prerequisites: its installer wants cargo, a C compiler, pkg-config and git, and the dependencies build sqlite with a
  vendored openssl (perl and make). All are already in `install/packages.sh` (build group) and `release/deps.sh`; no new
  package is needed. Run-time: at-spi2-core (already the `at-spi` row, a recommended need); the cua installer prints a
  note when the accessibility service is missing. No runtime library beyond the ones the other programs already load,
  so `release/deps.sh` is unchanged (the CI check of linked libraries will say if that is wrong).
- Release: `release/build.sh` has `build_cua` (`cargo build -p computer-use-daemon`, then the installer with `--bin-dir`, `--prefix
  /opt/bardo/current`, `--destdir <stage>`, `--sysconfdir /opt/bardo/current/system/etc`). Everything it stages is in an
  exported folder (`libexec/bardo` and `share/dbus-1/interfaces` stay in the bundle without links, as the actions service's do) or is
  the one system file `system/etc/bardo/cuad-callers.toml`, in `/etc/bardo`, an allowed system folder; no new folder is
  needed. The CI `agents` job builds `memory,actions,models,cua`. cua's own build config needs no
  `net.git-fetch-with-cli` entry in `install/cargo.sh` (its config file has none, like the actions service's).
- Not run: the CI job and `build_cua` (no build here). `gate/installer-check.sh` and a dry run passed.

## desk-cua: xwayland-satellite on every distribution (D5, 2026-10-16)

- Facts, read on GitHub today: licence MPL-2.0 (`LICENSE`; Cargo.toml `license = "MPL-2.0"`), which is fine for a
  separate program; latest release v0.8.3 (2026-09-24, not a prerelease), with a committed `Cargo.lock` (so `--locked`
  works), edition 2024, `rust-version` 1.85.0, no `rust-toolchain`. Dependencies by its README: Xwayland >= 23.1, xcb,
  xcb-util-cursor; cargo, clang and optional git to build. Its manifest also uses the xcb extensions composite, randr and
  res. The compositor's own memo (`docs/xwayland-decision.md`) already says MPL-2.0 and "a separate process". Fedora packages it
  (0.8.2 on Fedora 43, 44 and 45, packages.fedoraproject.org); Arch packages it (the audit verified 0.8.3).
- Release bundle: `release/build.sh` has a `satellite` component (`cargo install --locked --git <url> --tag v0.8.3 --root
  <scratch>`, run from `/` with `RUSTUP_TOOLCHAIN` 1.98.1, then only the program is copied), staged at `bin/xwayland-satellite`
  with an MPL notice (version, licence, source address) at `share/licenses/xwayland-satellite/NOTICE.txt`, recorded as
  `xwayland-satellite v0.8.3` in the release's component list. `release/exports.toml` links it to
  `/usr/local/bin/xwayland-satellite` (on the user manager's default PATH, ahead of a distro's `/usr/bin` copy, so the
  bundle's wins), which is how the compositor finds it. The CI `small` job builds it with the other small components.
- Build dependencies in `release/deps.sh` (the CI image installs them): Ubuntu `libxcb-composite0-dev libxcb-randr0-dev
  libxcb-res0-dev libxcb-cursor-dev` (the last two checked on packages.ubuntu.com for noble; the first two written from
  memory), Fedora `xcb-util-cursor-devel` (libxcb-devel already has the extensions; unverified), Arch `xcb-util-cursor`
  (libxcb has them). clang, pkg-config and libxcb are already in the lists. Runtime lists (the smoke jobs): the same
  libraries without `-dev`, and the Xwayland package on all three (`xwayland`, `xorg-x11-server-Xwayland`, `xorg-xwayland`).
- Source install: `install/packages.sh` keeps `xwayland-satellite` for Fedora and Arch (the Fedora `# unverified` mark is
  gone) and the Ubuntu note now carries the apt build line and the exact `cargo install --locked --git ... --tag v0.8.3`
  line, in plain words; `docs/distros.md` has the same under "X11 apps on Ubuntu". Not run. The `--check` note is one
  line, because notes are printed that way.
- Not verified: that `cargo install --git ... xwayland-satellite` builds with the package list above (no build here);
  the first CI run shows.
- Superseded by the follow-up below: a satellite in `~/.cargo/bin` may not be on the PATH the compositor's unit sees.
- `gate/distro-check.sh` checks that the Ubuntu note carries the cargo line and that the tag matches `release/build.sh`.

### Follow-up: full MPL text and the Ubuntu config line (2026-10-16)

- The bundle now carries the full MPL-2.0 text beside the binary: `build_satellite` shallow-clones the tag
  (`git clone --depth 1 --branch v0.8.3`) in the scratch folder, checks the first line of `LICENSE` is the Mozilla Public
  License, and stages it as `share/licenses/xwayland-satellite/LICENSE` next to `NOTICE.txt` (which keeps the source
  address and now also the commit). The release selftest checks no licences for any component, so it asserts nothing new.
- The compositor has a path setting (the compositor 5ea8b42, `config/sections/xwayland.rs` and `xwayland/resolve.rs`): key
  `satellite` in the `[xwayland]` table of `$XDG_CONFIG_HOME/bardo/compositor/settings.toml` (default
  `~/.config/bardo/compositor/settings.toml`); a value with a `/` is used as the full path and is not expanded (no `~`), empty means
  look in the PATH; it takes effect at the next login. So the Ubuntu note no longer asks for `sudo install`: it prints the
  cargo line (as yourself, never root) and then `satellite = "<home>/.cargo/bin/xwayland-satellite"` with the path to the
  file, and says the person writes it. The installer writes nothing there. `gate/distro-check.sh` and `docs/distros.md`
  match. Not run on a machine: that compositor reads the key as described (read from its source only).

## Production audit lane pa-desk (2026-10-16)

Written without a compiler, a systemd user manager, polkit or minisign; nothing here was run except `bash -n`. What to
check first is listed under "Not verified" at the end of this section.

### ROOT-03: the crash guard consults the updater

- `session/desktop-session` runs `"$(bin updater)" recover` once, before the respawn loop, and only when it is not a nested
  test run and the updater is installed (`have updater`). Exit 5 (the previous version was put back) is logged as "the latest
  update did not start; the previous version was put back" and the session starts the restored version; any other nonzero
  exit is logged and the session starts anyway. The updater's own words go to guard.log.
- guard.log moved from `$XDG_RUNTIME_DIR` (erased at the end of the session, which is when it is needed) to
  `${XDG_STATE_HOME:-$HOME/.local/state}/desktop-session/`, created at start, moved to `guard.log.1` once past 256 KiB.
  compositor.log stays in the runtime folder (the drop-in appends to it and the socket name is read from it).
  `BARDO_SESSION_STATE` (tests) still puts both in one folder.
- When the guard ends the session it prints `pkexec /usr/libexec/bardo-updater-rollback` (only where that program is
  installed; `BARDO_SESSION_ROLLBACK_HELPER` overrides the path for tests). That command asks for the administrator's
  password, by the owner's decision.
- **Not closed, needs a decision (the updater FINDINGS 22):** the updater counts different boots, so a crash loop inside one boot
  (login screen, log in, guard ends the session, log in again) is not rolled back until the computer restarts; the manual
  line above is the only way back within a boot. Closing it needs the guard to tell root, on a channel a program of the
  person cannot forge, that the session was ended by the guard; there is none today and none is invented here.
- The scenario `session-crash-guard` checks the way-back line and that guard.log is in the state folder while compositor.log
  stays in the runtime folder. The `updater recover` call itself is not testable there (it is skipped when nested, which is
  the only way that scenario runs); see the release scenarios below.

### ROOT-02: the update service starts in every login; the shell confirms the start

- `session/desktop-session.target` now has `Wants=bardo-updater.service`, so the daily check runs without anyone enabling the
  unit (the updater's `scripts/install.sh` deliberately does not enable it: that would write under the home folder). A computer
  without the updater has no such unit and the line is ignored. `gate/installer-check.sh` checks the staged target has the line.
- The session does **not** run `updater confirm`: the shell does, about 30 s after the shell answers (the shell lane, landing
  separately). **Landing order matters:** with `updater recover` in the session and no `updater confirm` anywhere, every
  update would be put back after its third restart of the computer. Land (and pin in `manifest.toml`) the shell revision that
  confirms before or together with this commit. Nothing in this repo can check that; it is the release gate's job.
- The updater also installs a D-Bus activation file for the update service, so a call to it (Settings, `updater status`)
  starts the unit in a login that does not have the target; the target line makes it deterministic.

### ROOT-07 (desktop half): system files are named one by one

- The updater's helper now accepts a system file only when a rule of `ALLOWED_SYSTEM_FILES` names it (13 rules, file by
  file; the folder list is gone). `release/system-files.list` is the committed copy, `release.py` reads it
  (`system_ok`, so `pack` and `verify` refuse any other path), `release/check-system-list.sh` compares it with the updater's
  Rust source, and the `small` build job runs that check against the pinned checkout. The selftest renders the list in
  The updater's format and checks equal, one rule missing and one rule extra, packs with four unnamed files (a polkit rule in an
  allowed folder, a second PAM file, an unnamed udev rule, `/etc/bardo/callers.toml`, which does not end in `-callers.toml`
  with something before it) and forges one more hostile release (`system-unnamed`). The selftest ran here (minisign 0.12): pass.
- `manifest.toml` moves the updater from aa427f2 to 769704c, the tip with the exact list, the three helpers and the session
  calls. The release cannot be installed by the older helper once it carries a path the older list does not name; the
  hand-installed helper must be replaced on each computer (the updater's `scripts/install.sh`).
- Every file `build.sh` stages is on the list. The accounts service, the memory service, the actions service and cua stage their caller tables through their own
  installers, so a name they write that the list does not cover is only found when `pack` runs in CI.
- Superseded wording above: "allowed system folders" (this file and the README) now means the list; the stale line in
  "Whether the release should write `/etc/pam.d` at all" is answered: one file, `/etc/pam.d/shell`, named.

### Packaging: the updater's three programs and one polkit file

- The updater's `scripts/install.sh` (origin/master 769704c) installs `bardo-updater-apply`, `bardo-updater-rollback` and `bardo-updater-health`
  in `PREFIX/libexec`, `org.bardo.updater.policy` (three actions) in `PREFIX/share/polkit-1/actions`, the user unit and bus
  file, `/etc/bardo/updater/config.toml` and, with `--pubkey`, `/usr/share/bardo/updater/release.pub`. The desktop's own `install.sh` does
  not run it (it needs root and a key, and builds with cargo's defaults: ROOT-17's linker and root-build points stay with
  The updater); the desktop's path is `release/bootstrap.sh` after it, and both now say so.
- New `release/check-helpers.sh` holds the install to the owner's decisions: the three programs exist, are root-owned and
  not group/other writable; the policy has exactly the three actions, each naming its program; apply and health allow only an
  active local session without a password, rollback is `auth_admin_keep`. `smoke.sh` now installs the three programs and the
  policy the way the updater's install does and runs it before the release; `bootstrap.sh` runs it before it asks for anything.
  The workflow's `small` job uploads all four updater programs and the policy file (artifact `helper`). The selftest covers
  a complete install, a rollback without a password, a missing program and a group-writable one.
- `docs/distros.md` has a short "Updates" section (what is installed where, the password rules, the by-hand way back).
- Not verified: `scripts/install.sh --bin-dir` flow on a real machine; that polkit follows the policy file's `exec.path`
  to `/usr/libexec/bardo/updater-*` on Fedora and Arch as on Ubuntu (it reads the path, not the distro).

### CI scenarios for the start check (ROOT-02, ROOT-03)

- `release/health-scenarios.sh` and the workflow job `health` (publishing waits for it). Scenario 1: install two signed
  releases, two logins each running `updater recover` then `updater confirm`, four more restarts: the newest release stays.
  Scenario 2: install two, six starts inside one restart (not counted), then restarts that never confirm: the third makes
  `updater recover` exit 5 and say the previous version came back; the next three leave it alone; a newer release starts
  its own count. The releases are made with the repo's own `pack.sh` and a throwaway key; the programs are the updater's real
  ones.
- **The gap:** `bardo-updater-health` reads the boot id from a fixed path and the settings from `/etc/bardo/updater/config.toml`, and
  pkexec clears the environment, so no variable can stand in for them. The scenario therefore needs root and the right
  to mount: it binds a made-up boot id over `/proc/sys/kernel/random/boot_id` (a `--privileged` container job) and puts a
  stand-in `pkexec` (runs its arguments; the container is already root) first on the path. It cannot run in the jail
  (`scenarios/` is unprivileged, with no mounts) and so is not part of `desktop-gate.sh`; the gate's checks of it are
  `bash -n` only. The jail-run `session-crash-guard` cannot cover `updater recover` because the session skips it when nested.
  A seam in the updater (a boot-id file and a settings path taken from an environment variable that only a non-setuid run
  honours) would let it run unprivileged; it is not asked for, since a root helper must not read either from its caller.
- Not verified: that a hosted runner starts a container with `--privileged` and allows `mount --bind` over a procfs file
  (Docker mounts `/proc/sys` read-only without `--privileged`); that `updater confirm` creates its state folders under the
  scratch home; the exact words of `updater recover` when it restores (the scenario looks for "previous version").
  If `mount` is refused the script exits 2 and the job fails; that is the signal, not a pass.

### ROOT-18: the jail hides the person's keys (partly)

- `harness/jail.sh` showed the whole host read-only, including `~/.ssh`, `~/.config/bardo-release` and API-key files, and a
  `--bind` dir is writable: code run in a lane could copy a key into the worktree. `harness/secret-paths.list` now names the
  places keys live (14 folders, 7 files); the jail covers each that exists with an empty folder or an empty file, and
  sets `JAIL_REAL_HOME` so `selftest.sh` checks they read empty (verified here: `selftest.sh --all` passes, `.ssh`,
  `.gnupg` and `~/.cargo/credentials.toml` show empty inside).
- **Deferred, a decision:** the verified fix is an allow-list (show only `/usr`, `/etc/os-release`, the worktree and scratch).
  Lanes also read `~/.cargo`, `~/.rustup`, the sibling checkouts and the gate's build directories under the home folder, so
  an allow-list needs a pass over every scenario and the lane gate to find what they read; I could not run those here. A
  list of known places misses a key kept somewhere new, so a new kind of secret needs a line in the list.
- `--cap-drop`, `--unshare-uts` and a seccomp filter (also in the finding) are not added: the user namespace already has
  no capabilities outside it, and a filter needs the same lane-by-lane test.

### ROOT-08: what builds the signed release is pinned

- rustup: `release/install-rustup.sh` downloads `rustup-init` 1.29.1 for x86_64 from `static.rust-lang.org` and checks its
  SHA-256 (dda72343...; read from the published `.sha256` and from the downloaded file on 2026-10-16) instead of
  `curl sh.rustup.rs | sh`. Any other version needs `--sha256`. The toolchains themselves still come from rustup's
  own signed manifests.
- Container: the build jobs and the `health` job use `ubuntu:24.04@sha256:534baea6...` (the index digest on Docker Hub,
  read 2026-10-16). It does not follow Ubuntu's security rebuilds until someone bumps it; apt inside it is still
  unversioned (the archive is signed and the base is fixed; pinning every package version would break at the first archive
  update and was not done). smoke keeps `fedora:44` and `archlinux:latest` moving on purpose: they only test, they sign nothing.
- xwayland-satellite: `SATELLITE_REV` (b83eab90..., the commit `v0.8.3` names) replaces the tag in the build; the commit is
  fetched once, verified by `rev-parse`, built with `cargo install --locked --path`, and the licence, the notice and the
  component record come from the same checkout (the second clone is gone). `gate/distro-check.sh` checks the value is a
  full commit hash. The source-install line for people in `install/packages.sh` and `docs/distros.md` still says
  `--tag v0.8.3` (it is typed by a person, not by CI).
- Cache: `restore-keys` is gone, so a cache is restored only for exactly this manifest. Cost: the first build after a
  manifest change is cold (the compositor and the shell take the longest).
- Not verified: that `git fetch --depth 1 <url> <full sha>` is accepted by GitHub for that repository (it is for public
  repositories that allow reachable-sha fetches, as GitHub does), and that `cargo install --path` of the repository root
  installs only `xwayland-satellite` (its root `Cargo.toml` is a package that is also a workspace root; read from
  the file at that commit).

### ROOT-09: the signing key is in a protected environment

- The workflow's `pack` job runs in the environment `release-signing` (the signing key and its password) and `publish` in
  `release-publish` (the token that can create a release in desktop-releases); the `prepare` job also refuses any ref but the
  default branch and `v*` tags. The pack job checks `release/release.pub` is committed, and that the environment has a key,
  before it signs anything, and prints the public key's id line in every run. `release/README.md` "What you do once" steps 3,
  5 and 6 are the owner's one-time commands (create both environments with a reviewer and branch/tag restriction, a tag ruleset
  on `v*`, set the secrets in the environments, delete any repository-level copies). Nothing was created: no repository,
  secret, environment or key.
- Until the owner has done that, the run stops at `pack` with a message naming the missing piece. `release/release.pub` is not in
  the repository (only the owner can make the key), so the first release also needs step 3.
- **Needs a decision:** a second signer and a list of next keys (so a key can be rotated without every person reinstalling the
  helper) need a change in the updater's `PublicKey` and a new helper on every computer. Not done.
- Limits: environment rules protect the secrets from a pushed branch, not from someone who can change the default branch or
  push a `v*` tag; approvals are a click by the owner, who is also the only reviewer.

### ROOT-17 (desktop half)

- The desktop's "First install" no longer says `sudo scripts/install.sh --pubkey` (which compiles the updater's whole dependency
  tree as root and, with a strict root umask, leaves `/etc/bardo/updater` and `/usr/share/bardo/updater` unreadable to the update
  service). It builds the three programs as the person, installs them with `--bin-dir`, and runs the install under
  `umask 022`; the CI never used the script (`smoke.sh` and `health-scenarios.sh` `install -m` the programs).
- **Left to the updater (interface asks):** `scripts/install.sh` should use `install -d -m755` and `install -m` instead of `mkdir -p`
  and the caller's umask, and refuse to build when run as root; `bardo-updater-apply`, `bardo-updater-rollback` and `bardo-updater-health` should
  be their own package with only `updater-core`, `tar`, `zstd` and `sha2` (today they share a package with zbus, tokio, ureq
  and rustls); the repository should run `cargo deny check advisories` in CI (`deny.toml` checks licences only). The desktop's
  workflow builds the updater from the manifest pin, so the helper's supply chain is whatever that package's dependencies are.

### ROOT-14 (desktop half): a login shell that never finishes no longer leaves a black screen

- The second stage of `desktop-session` is entered through one login shell so the person's `~/.profile` PATH applies.
  That shell now gets no input (`</dev/null`, so a `read` in a startup file ends at once) and a watchdog started before it
  ends the session after 60 s (`login_shell_secs`) if the second stage has not marked itself started
  (`$XDG_RUNTIME_DIR/desktop-session-login-shell.<pid>`), after logging one line that names `~/.profile`. The kill happens
  only if that PID's command line still has `--in-login-shell` (a PID can be reused). Tried here with a stand-in shell that
  sleeps: the line is printed and the session ends with status 143; the healthy path (marker written, watchdog silent) is
  read, not run, because stage two needs a real compositor. Not covered by `session-crash-guard` (it runs nested, which skips
  the login shell).
- Not done: `exec '$0'` is still single-quoted (a path with a quote in it breaks, as before; the standard places do not
  have one, and an escape that works in bash, zsh and fish alike was not found); the second stage still sources
  `~/.config/desktop-session/session.env`, which decides the compositor command line (a same-user process can already edit
  `~/.profile`, so this adds little, as the verified row says).
- **Interface ask for the accounts service:** accountd should not accept a unit by name alone for the `compositor` and shell roles: check the
  unit's fragment path and drop-in paths (`systemctl --user show -p FragmentPath -p DropInPaths`), since the drop-in that
  decides what runs as `bardo-compositor.service` is written under `$XDG_RUNTIME_DIR/systemd/user/`, which any process of the person
  can write. (Done on the session side, lane pa-units: bardo-compositor.service and shell.service are installed root-held in
  `/etc/systemd/user` and carry `UnsetEnvironment` for the loader variables; the session passes its arguments as
  `COMPOSITOR_SESSION_ARGS` in the user manager's environment, writes no drop-in, and removes the old runtime one at start.)

## pa2-desk: the table of the desktop's programs, the input service, the keyboard map (lane pa2-desk, 2026-10-17)

What changed: `dist/desktop-callers.toml` (the caller-identity library's table, one row per role, `settings` twice, four paths per program), placed by
`install.sh` first (`install/system.sh callers`) and by every release (`system/etc/bardo/desktop-callers.toml`); the input service
(`input_broker`: program, `/usr/local/bin/input_broker` export, socket, service, `72-input-broker.rules`, `input-broker.conf`) in the release and in
`install.sh`; the key remap daemon installed by `install/system.sh key_remap`, not by the key remap daemon's installer; `release/system-files.list` takes the updater's four input broker
rules (it equals the updater origin/master, 17 rules, `check-system-list.sh` says so); three new gate checks and one test for them.

- **The socket lets any local program connect, on purpose.** `bardo-input-broker.socket` is `SocketMode=0666`, `DirectoryMode=0755`
  (`/run/bardo/input-broker/input.sock`); the input broker's own header says who may have a device is "decided by the identity of the caller, never by this
  mode" (the caller-identity library over the pidfd and this table: `pad_daemon` for the mouse gestures daemon, `keymap_daemon` for the key remap daemon, at `Exe`). So the key remap daemon's service (a
  `DynamicUser`) and the mouse gestures daemon (the person's user service) both connect, and nothing in the desktop changes the mode. The unit comments, the input broker's
  README and the wave-2 design (4.1) agree; the daemon was not run.
- **`settings` has two rows.** The first keeps the caller-identity library's (unit `org.bardo.Settings.service`, for processes that unit starts); the second has no
  unit (`app = "org.bardo.Settings"`, the `detent` images), because Settings is also started by a launcher outside any unit and the updater asks for it at
  `Exe` (the updater FINDINGS 24, its ask to this lane). Four roles are known by image alone: `settings` (second row), `updater_cli`,
  `sandbox_launcher`, `manager`. `check-callers-table.sh` allows no other row without a unit (the brief said "settings only"; the caller-identity library's own
  table and the updater need the other three).
- **Each program is listed in four places**, not three: `/opt/bardo/releases/*/...` (what `/proc/<pid>/exe` shows), `/opt/bardo/current/...`
  (what the release's units say in `ExecStart`, and the caller-identity library matches `ExecStart` against these patterns for `Full`), `/usr/local/...` (a build from
  source with the default prefix; the daemons in `libexec` also get `/usr/local/libexec/...`, which caller-identity library's sample lacks) and `/usr/...`.
  `check-callers-table.sh` compares each unit's `ExecStart` in the checkouts with its row (13 of 15 units found; the updater's and
  xdg-desktop-portal's are not in a sibling).
- **The key remap daemon is no longer installed by the key remap daemon's installer.** Its `install.sh` always places `dist/key_remap.uinput.conf` (`DeviceAllow=/dev/uinput`
  as a drop-in) and `dist/70-key-remap.rules` (`GROUP="input"`, `MODE="0660"` on the node), the "standalone" pieces; the owner rule is that nothing
  shipped gives the node to a group or the person. The release no longer stages `70-key-remap.rules` or `key-remap.conf` (the input broker's `input-broker.conf`
  loads `uinput`); a computer that got them from an earlier release has them removed by the updater when unchanged. `72-input-broker.rules` sorts after
  `70-` and sets root and 0600, so a leftover `70-key-remap.rules` is overridden, but `system.sh key_remap` warns about it and about
  `/etc/systemd/system/bardo-key-remap.service.d/10-uinput.conf`. Lost by not running the key remap daemon's installer: the Plasma overlay (`--desktop plasma`, the default on
  KDE), `--plasma-focus`, and `restorecon` on its own list (system.sh runs `restorecon -F` on what it placed). A KDE machine that wants the overlay
  runs the key remap daemon's installer by hand and then removes its drop-in.
- **Resolved (2026-10-17): the Caps Lock rule ships as the key remap daemon's vendor file, `/usr/lib/bardo/key-remap/key-remap.toml`.** It used to be
  `/etc/bardo/key-remap/key-remap.toml`, which no release could carry (a release must never write the administrator's `/etc`, and the updater's
  list named no such file). The key remap daemon gained a vendor layer under the machine file (built-in < vendor < `/etc/bardo/key-remap/key-remap.toml` < the
  person's own; same trust as the machine file, read only when root-only; the key remap daemon's FINDINGS "Vendor layer"), the updater's list gained
  `/usr/lib/bardo/key-remap/key-remap.toml` (18th rule, none removed), `install/system.sh key_remap` places `dist/key-remap.toml` there every time
  (644 root:root, like the callers table) and `--remove key_remap` takes it back while unchanged, `build.sh` stages it with
  `put_system`, and `system-files.list` has the same rule. `/etc/bardo/key-remap` is no longer touched by any install or release. Left: (1) a
  computer whose the updater helper predates the rule must re-run the updater's install by hand before the release that lists it (an older
  helper refuses that release whole), (2) a `/etc/bardo/key-remap/key-remap.toml` an earlier `./install.sh` wrote stays (it holds the same rule, so
  it changes nothing; delete it by hand, `--remove key_remap` no longer looks at it), (3) the key remap daemon and the updater pins in `manifest.toml`
  must move to revs that have the vendor layer and the rule before a release is cut (until then `check-system-list.sh` fails against
  the old updater pin, on purpose), and (4) the key remap daemon watches `/usr/lib/bardo/key-remap` only if it exists when the daemon starts; `system.sh`
  restarts a running service when it changed any file, which covers a first install.
- **A table or a unit placed by `install.sh` and a later release can conflict.** The updater replaces a system file only if it is absent, equal, or what
  an earlier release placed; anything else it leaves and reports (`plan_system`, `Conflict`). `desktop-callers.toml`, `input_broker.*` and the
  rules come from the same sources in both paths, so they are equal when the desktop checkout is the release's. `bardo-key-remap.service` was not (the
  release started `/opt/bardo/current/bin/key_remap`, `install.sh` started `<prefix>/bin/key_remap`): fixed by lane dharn, see its section below.
- **Before the first release that carries the input broker**, everyone re-runs the updater's install by hand (a helper with the four rules and the step that
  reloads systemd and the device rules and starts `bardo-input-broker.socket`); an older helper refuses that release whole (the updater FINDINGS 25).
  `release/README.md` says so. `release/check-system-list.sh` fails against the manifest's current the updater pin (5c33809, 13 rules) until the pin
  step moves it; `build.sh` builds `input_broker` only once the manifest has `[repo.input_broker]` (and `release/checkout.sh` reads the manifest), which
  this lane did not add.

Could not be determined by reading (nothing was built, installed or started; `shellcheck` is not on this machine, so `bash -n` was the only
syntax check, and `gate/installer-check.sh`, `release/*.sh` and `install/system.sh` were not run beyond `--dry-run` of `system.sh`):

| # | Fact | How to check |
|---|---|---|
| 1 | `Table::from_toml` accepts `dist/desktop-callers.toml` (the repo's checks re-implement the reader's rules in Python from the caller-identity library's docs; the caller-identity library was not compiled) | `cargo test` in the caller-identity library with `include_str!` of this file, or `TableFiles::load` on an installed copy |
| 2 | `/usr/libexec/xdg-desktop-portal` (Ubuntu, Fedora) and `/usr/lib/xdg-desktop-portal` (Arch) are the frontend's paths; `bardo-updater.service` and `xdg-desktop-portal.service` ExecStart are not compared (not in a checkout) | `dpkg -L`, `rpm -ql`, `pacman -Ql` per distribution |
| 3 | `gate/installer-check.sh` passes with the new order (callers first, the input broker before the key remap daemon, the key remap daemon's own installer never run) | run it |
| 4 | `release/build.sh --only input_broker,key_remap,mouse_gestures,session` stages the four input broker files, the table and no key remap daemon device rule; `retarget` accepts the mouse gestures daemon's `/usr/local/bin/mouse_gestures` | a local build (`release/README.md`, "Local trial") |
| 5 | `bardo-input-broker.service`, `bardo-input-broker.socket` and `bardo-key-remap.service` as the checkouts ship them pass `systemd-analyze verify` here (Fedora 44; the only complaint is that `/usr/local/bin/input_broker` does not exist yet); the rendered `ExecStart` differs only when the prefix is not `/usr/local`; not tried on Ubuntu or Arch | `systemd-analyze verify` there |
| 6 | `modprobe uinput` and the `uinput` module exist on all three; `udevadm trigger --subsystem-match=misc` re-applies `72-input-broker.rules` to a node that is already there | the three distributions |
| 7 | `/etc/bardo/key-remap` ownership when `ConfigurationDirectory=key_remap` meets `DynamicUser=yes` (the key remap daemon's own open item) and whether the daemon reads a root-owned `key-remap.toml` there | a start of the real service |
| 8 | The table check is not in CI: the pack job has no caller-identity checkout (the caller-identity library is not in `manifest.toml`); `--self` runs it where a caller-identity checkout is beside this repo | add the caller-identity library to the manifest or fetch its tag in the job |
| 9 | `check-no-uinput-access.sh` reads rule text; a rule that reaches the node without naming `uinput` is not seen | `udevadm test /sys/devices/virtual/misc/uinput` per distribution, `getfacl /dev/uinput` |

## dharn: the compositor asks per app, and the harness answers (lane dharn, 2026-10-17)

The compositor asks the person the first time a program watches the screen, types, reads the clipboard, lists windows, uses an input method, locks the
screen or changes the displays (the compositor ARCHITECTURE "Consent"). Nobody answers inside a scenario.

- **One helper writes the floor: `harness/lib/compositor-floor.sh`** (sourced by `harness/lib/all.sh`). `floor_make NAME [TOOL...]` writes
  `$JAIL_SCRATCH/compositor-floor/NAME/privacy.toml` with an `allow` row per power of each installed tool (`floor_powers` is the table: grim and
  wf-recorder `screen`, wl-copy/wl-paste/cliphist `clipboard`, wtype `keys`, fcitx5 `typing_help` and `keys`, wlr-randr `displays`, ...); an unknown
  tool is an error, so a new tool is added in one place. `floor_shim` does that and points `COMPOSITOR_REPO` and `COMPOSITOR_DIR` at a stand-in checkout
  whose `dev/up.sh` appends `--system-dir` (the compositor's own `dev/up.sh` passes it on), for the scripts that start the compositor themselves: the shell's library
  (its pin, `3fecef0`, has no floor of its own; its HEAD writes one with grim and wl-copy/wl-paste, which the appended flag overrides: no scenario here
  runs those) and shell-host's.
- **Every scenario that starts the compositor passes the flag:** the eleven accounts scenarios (through `acc_env`), `detent-on-compositor-at-1.5`,
  `launcher-p95-on-compositor`, `appearance-reaches-apps`, `appearance-reaches-bardo-apps`, `shell-boots-on-compositor`, `portal-bardo`,
  `shell-host-on-compositor-and-kwin` (floor: fcitx5, which accept-ime runs), `minimise-target-round-trip`, `space-id-survives-restart` and
  `session-crash-guard` (through the session's backend arguments). `key-remap-yields-and-regrabs` starts nothing (a skip). Most floors are empty on
  purpose: the shell, the Settings app and the rig take no power a row can name.
- **Programs run from `build/` (lane dharn, second pass).** The compositor judges a program by its image, and a program under `build/` is writable by the
  person, so it is "unverified" and asked every time, whatever the floor says; and without `/etc/bardo/desktop-callers.toml` (the jail has none) the shell,
  The Settings app and the portal are nobody and the shell cannot connect. The compositor 679e93d (cas12) lets a debug-inject build's `--system-dir` floor name such a
  program by its `readlink -f` path; the cas13 lane makes the same build read `DIR/desktop-callers.toml` (owned by the person, not group or world
  writable, not a link) as test caller rows in the caller-identity library's format (`app`, `role`, `exe`, no `unit`). `floor_make NAME [TOOL...]` now writes both files,
  mode 600, from one list of TOOLs: `floor_powers` (the powers a row allows), `floor_role` (the role in the caller list: the shell and the shell-host demos that
  map the shell's surfaces `shell`, the Settings app `settings`, the portal `portal`; the caller list keeps `dist/desktop-callers.toml`'s app ids) and `floor_image`
  (where the harness built it: `build/tools/bin`, `build/portal-target`, `build/shell-target`, `build/detent-target`, `build/shell-host-target/debug/examples`,
  else PATH; a program not built is left out and its step says so). The stale rule "a floor cannot name a `build/` program" is gone from the header.
  Per scenario: `detent-on-compositor-at-1.5` `bardo-compositorctl detent` (the `shot` is `screen`; the Settings app is `settings`); `portal-bardo` `portal` (`screen` and
  `portal`); `space-id-survives-restart` `compositor-probe` (`windows`); `shell-host-on-compositor-and-kwin` `fcitx5` (`typing_help keys`), `demo_lock` (`lock`)
  and the seven demos that take an exclusive keyboard or an overlay (hover, scroll, ime, swipe, switcher, drag, popup_keys) as `shell`: the compositor gives
  the exclusive keyboard only to the shell (CASC-02), which is why accept-modifiers and accept-ime failed. Not run (lane rules), and cas13 was not in
  The compositor yet when this was written: the file name, the `[[peer]]` keys and the role names are from the lane brief. Not changed: `shell-boots-on-compositor`,
  `launcher-p95-on-compositor`, `appearance-reaches-*` (they pass without the shell role; giving the shell the row is the real shape and a later edit once this
  one is green) and the accounts scenarios (they run the shell and the Settings app at their pins, a different image from `build/shell-target`).
- **Display confirmation.** A change by a program (wlr-randr and the like) is asked ("keep the new settings?", 15 s, reverts unless kept). No
  scenario changes outputs through a program: `set_scale` and the size changes go through `bardo-compositorctl debug output set`, which
  configures the output directly (`debug/dump.rs` calls `core_loop::configure_output`) and never opens a trial (checked in the compositor 377c937).
- **Release notes and the floor file (lane dharn).** The repo had no standing release notes (they are typed into the workflow's notes box or the tag's
  message); `docs/release-notes.md` now holds the next release's text, with the sentences about the first-use questions and the display confirmation,
  and `release/README.md` points to it. No `/etc/bardo/privacy.toml` is shipped by `install/`, `release/` or `dist/` (nothing names it), and a release
  could not place one: the updater's list allows `/etc/bardo` only for names ending `-callers.toml`. No gate script keeps a list of forbidden shipped
  files (`check-no-uinput-access.sh` reads rule text), so no line was added; the administrator writes the floor, the person's own answers live in
  `$XDG_STATE_HOME/bardo/compositor/privacy.toml`.
- **bardo-key-remap.service is one file from `install.sh` and from a release (lane dharn).** Both already put it at `/usr/lib/systemd/system/bardo-key-remap.service`
  (the updater's list, 4th rule); what differed was its `ExecStart`: the release rewrote it to `/opt/bardo/current/bin/key_remap` (`retarget`),
  `install/system.sh` writes `<prefix>/bin/key_remap`. The release now stages the unit as the key remap daemon ships it, `ExecStart=/usr/local/bin/key_remap`, the link
  `release/exports.toml` makes to `bin/key_remap` (what the input broker's unit already did), so at the default prefix `/usr/local` the two files are equal
  byte for byte and the updater never sees a conflict. `dist/desktop-callers.toml` already lists `/usr/local/bin/key_remap` and the release paths, and the
  link resolves to the release's program for the identity check. A non-default `--prefix` still writes its own path: that install is not a release's.
  `gate/installer-check.sh` asserts the list rule, the byte-equal unit at the default prefix, that `build.sh` stages the unit at that path without
  `retarget`, and the export link. Not run here (lane rules); the byte-for-byte case was tried by hand with `system.sh key_remap --destdir` in a scratch folder.
  A computer that already holds the previous release's unit (`/opt/bardo/current/bin/key_remap`) gets the new one as that release's own earlier file.

## pa3-root: install, release and session (lane pa3-root, 2026-10-17)

- **NEW-ROOT-1: a program root runs is never put where the person can replace it.** `./install.sh --prefix ~/.local` used to print the root command for
  The input broker and the key remap daemon with the same prefix; run as root, `install/system.sh` then wrote `ExecStart=/home/u/.local/bin/input_broker` into a unit that runs as
  root, in a folder the person owns. Two layers now. `install/system.sh` (the part that does the work) refuses, exit 1 and a plain sentence, when `<prefix>/bin`
  or any folder above it is not owned by root or can be written by group or others (`install/rootheld.sh`, `not_root_held`; symlinks followed, a folder
  that does not exist yet judged by its nearest parent). `install.sh` does not stop the whole desktop install for it: it hands these two parts `/usr/local`
  (the release's place; `dist/desktop-callers.toml` lists only `/usr/local/bin` and `/usr/bin` for them anyway) and the plan says so. Chosen over "always
  /usr/local" so that a packager's `--destdir ... --prefix /usr` still lands in `/usr/bin`; a `--destdir` is not judged (it is a staging root; the folder it
  becomes is the packager's to hold). Test: `gate/installer-check.sh` 2e (a user-owned prefix is refused for both parts, `/usr` is accepted, the plan
  moves them to `/usr/local` and says so, a staged prefix is kept). Not covered: root-owned but group-writable folders (the test cannot make one without
  root); the function was run by hand over `/tmp`, `~/.local`, `/usr/local` and a missing path.
- **NEW-ROOT-2: a build job supplies only its own paths.** `pack` used to extract all seven stage tars into one tree and sign it, so a build script run by
  a dependency of one job (mailo, say) could write `bin/input_broker` or `bardo-input-broker.service` into its tar and have it signed as root-run code.
  `release/stage-owners.list` is the table of path patterns per job and `release/stage-owners.py check` runs in the Merge step before any extraction:
  it refuses a member outside its job's lines, a member that is not a plain file or folder, a path outside `stage/opt/bardo/current` and `components.d`,
  and a path two jobs both supply. `input_broker` and `key_remap` moved out of the `small` job into a job of their own (`root`), so nothing built beside them
  can reach their program, units, udev rules or the polkit rule; `system/etc/bardo/desktop-callers.toml` (the trust table) is in the `small` list
  only (the session builds it) and no wider pattern of another job matches it. Tests: `release/selftest.sh` (every root-run path refused for every
  other job, a duplicate, a link, a stray path, an unknown job, and the list's job names equal the workflow's matrix). Not run on a runner: the lists
  were written from `build.sh` and the installers it runs, so the first run may stop on a file a component installs that no line names (add the
  line; the fault names the job and the path). Left as is: inside one job a compromised component can still write another component's paths (the
  `small` job holds six components); only the root-run parts and the trust table were separated.
  N-1 (final audit): the job used to come from the tar's file name after a merged download, so an artifact `stage-x` holding `stage-input-broker.tar`
  could pass as the input broker's. The workflow now downloads each artifact into its own folder and `check` takes the job from the folder name, refusing a
  folder that holds anything but exactly its own `stage-JOB.tar` (a plain file). `selftest.sh` forges a foreign tar, a second tar, a stray file, a
  wrongly named tar, an empty folder and a linked tar. Not run on a runner: that `download-artifact` v8 with a pattern and no merge makes one
  folder per artifact, named for it, is from its documentation.
- **NEW-ROOT-3 (desktop side).** `release/release.py`'s `EXPORT_ROOTS` and `AUTO_EXPORT_DIRS` lose `/usr/share/polkit-1/actions` (the same ten folders as the updater's
  `ALLOWED_LINK_DIRS`); `release/selftest.sh` forges `export-polkit` (verify refuses) and checks a staged `share/polkit-1/actions/*.policy` is packed but not
  exported. The desktop ships no polkit action; the key remap daemon's file is a rule (`/etc/polkit-1/rules.d/key_remap.polkit.rules`, in the exact list). If one is ever needed it
  is an exact rule in the updater's `system.rs` and `release/system-files.list` for that file name, and `check-helpers.py`'s test (no `allow_active=yes` on a privileged
  action) has to be applied to it first. Nothing to name today.
- **NEW-ROOT-4: the shell is told when the run is the safe fallback.** `session/desktop-session` sets `BARDO_SESSION_MODE` to `normal` before each compositor start and to
  `safe` before `bardo-compositor --safe`, in the user manager's environment (`systemctl --user set-environment`, the route `COMPOSITOR_SESSION_ARGS` takes; shell.service reads the
  manager's environment when it starts, and so does a restart by the manager) and exported for the nested test path; both variables are unset at every way out
  (`unset_session_args`). The reader is the shell: it must skip `updater confirm` when the variable is `safe` (interface ask to the shell lane, below in the report: the shell's `release.rs`).
  Until the shell lands that, nothing changes for the person. Test: `scenarios/session-crash-guard.sh` (the fake compositor logs the variable: three `normal`, then `safe`; the shell
  stand-in sees `normal`). The non-nested `set-environment` call is not covered by a test (the scenario runs without a user manager).
- **NEW-PI-1: every trusted unit throws away the loader variables, and the portal frontend gets a drop-in.** the caller-identity library proves `Full` only when a unit's `UnsetEnvironment=`
  holds all 22 names of `LOADER_VARIABLES` (caller-identity-core `facts.rs`), and the compositor and the portal require `Full`; the distro's `xdg-desktop-portal.service` has none, so
  on a real install the frontend would be refused for every call. `session/xdg-desktop-portal.service.d/10-bardo.conf` (generated from the constant, not typed) is
  placed by `install/install.sh` at `<sysconfdir>/systemd/user/xdg-desktop-portal.service.d/10-bardo.conf` (root-held, `/etc/systemd/user`) and by every release at
  `lib/systemd/user/...`, which the packer exports as a link under `/usr/lib/systemd/user`: a folder the updater already allows, so no change to its list (a link there is
  judged by the caller-identity library like any fragment; the manager reports a drop-in path the same way). `release/stage-owners.list` names it for the `small` job. The source of truth is
  The caller-identity library's constant: `gate/check-loader-unset.sh` reads it from `caller-identity-core/src/facts.rs` and checks the drop-in and every `unit =` row of
  `dist/desktop-callers.toml` (plus `bardo-input-broker.service`) whose file is in a sibling checkout; `gate/installer-check.sh` runs it on the staged drop-in and on generated
  trees (3d); `gate/desktop-gate.sh --self` runs it. Units that live in other repositories and still lack the full line are on `gate/loader-unset-pending.txt`
  (a debt list: accountd.service in the accounts service, bardo-key-remap.service in the key remap daemon, org.bardo.Settings.service in the Settings app, bardo-input-broker.service in the input broker, which has 3 of 22);
  the check fails when one of them is complete and still listed. `updater/dist/bardo-updater.service` got the line in this lane. Not verified: that a drop-in that is a
  symlink into `/opt/bardo/current` is judged `RootOwned` (read: `trust_of_path` is the same for fragments and drop-ins, and bardo-compositor.service is already such a link).
- **ROOT-17 leftovers.** `install -d` without a mode in `install/install.sh`, `install/fonts.sh`, `release/smoke.sh` and `release/health-scenarios.sh` now says `-m 755`, and
  `install/install.sh` and `install/system.sh` set `umask 022`, so a root umask of 077 no longer makes `<prefix>/share/xdg-desktop-portal` a 0700 folder.
- **ROOT-14 finished: the session settings file is data.** `~/.config/desktop-session/session.env` was sourced as shell (everything it set exported, before the session
  had started). `read_settings` now reads lines `NAME=VALUE` and takes only `BARDO_SESSION_QUICK_SECS`, `BARDO_SESSION_FIRST_RUN_DELAY` (whole numbers) and
  `BARDO_SESSION_BACKEND_ARGS`; anything else is ignored with a line in the log, and nothing in the file is run. The other `DESKTOP_SESSION_*` knobs (the compositor command,
  stubs, state folder) stay environment-only, which is how the harness sets them. Test: `gate/installer-check.sh` 3e takes the function out of the script and runs it on a
  file that holds a command, a `$(...)`, `PATH=` and a bad number. Still open, not ours: the accounts service's unit-path check (the accounts service ask in the ROOT-14 entry above, on hold);
  `COMPOSITOR_SESSION_ARGS` in the user manager is same-user writable and the compositor refuses a floor not owned by root, so no bypass.
- **ROOT-18 (more places, still a deny-list).** `harness/secret-paths.list` grew by browser profiles, cloud and secret-manager tool folders and token files (43 lines).
  The allow-list (show only `/usr`, the worktree and scratch) still needs a pass over the scenarios to learn what each reads: left, as the plan says.
- **ROOT-15 (the shell's i2c `uaccess` rule): accepted by default, owner may overrule.** `shell/dist/udev/60-shell-ddc.rules` tags i2c-dev nodes `uaccess`, so the person at the
  seat can talk to the monitor's controller: the same as every distro that ships ddcutil's rule. The alternative is a root broker for brightness like the input broker; that
  is a design and an owner decision. Nothing changed here (the file is in the shell).
- **ROOT-16 (the key remap daemon's polkit rule allows `restart`): kept.** The shell restarts a failed keymap itself (`RestartUnit` in the shell's `key_remap/backend`, tested in
  `key_remap_systemd_bus.rs`), so dropping the verb breaks recovery. Left as is: a same-user process can restart (not replace) the key remap daemon, bounded by the unit's
  `StartLimitBurst`; the rule lives in the key remap daemon's `dist/key_remap.polkit.rules`.
- **ROOT-06 and ROOT-09: nothing new in code.** ROOT-06's remaining half is the per-file `size` in `[[file]]` (a format bump with a new helper on every machine, the updater
  FINDINGS 18, not started). ROOT-09's remaining half is the owner's one-time setup and the second-signer decision. ROOT-07 is closed by NEW-ROOT-2 and NEW-ROOT-3 above
  (the list now also bounds who may supply the files, and the second path through polkit's actions folder is gone).

## the mouse gestures daemon as a system service, plain units, Shift+Caps Lock (lane pr1, 2026-10-17)

- **The mouse gestures daemon is a system service (owner decision 2026-10-17).** Its unit is a system file at `/usr/lib/systemd/system/bardo-mouse-gestures.service`
  (a dynamic user, `/system.slice/bardo-mouse-gestures.service`), so the input service (the input broker) recognises it by the same proof as the key remap daemon
  and no person's process can fill its control group. Desktop side: `install/system.sh mouse_gestures` (program, unit, `70-mouse-gestures.rules`
  giving the mouse's raw node to the `input` group and to no person, `99-mouse-gestures.conf`; `--prefix`, `--destdir`, `--remove`, the
  root-held prefix rule NEW-ROOT-1 applies to it too; `install.sh` plans it with the other system parts and moves a user-owned
  prefix to `/usr/local`), `release/build.sh` stages the unit with `put_system` and no `retarget` (the same bytes as install.sh at
  the default prefix), no user unit is staged any more, and `gate/installer-check.sh` covers the install, the removal, the plan and
  the byte equality (its stub unit carries the loader line). Release jobs: `root` is split into `input_broker`, `key_remap` and `mouse_gestures`
  (NEW-ROOT-2: each program the system manager starts supplies only its own paths; `release/stage-owners.list`, the workflow matrix
  and `release/selftest.sh` agree). Not done: the settings the person writes in their home (`~/.config/bardo/mouse-gestures/gestures.toml`) and
  the programs a gesture runs in the session; both are open in the mouse gestures daemon's FINDINGS ("Open after the move to a system service").
- **Needs the updater (a new helper on every computer).** `release/system-files.list` gained three rules, in this order after
  `/etc/modprobe.d exact 99-mouse-gestures.conf`: `/usr/lib/systemd/system exact bardo-mouse-gestures.service`, `/usr/lib/systemd/user ending .service`
  and `/usr/lib/systemd/user/xdg-desktop-portal.service.d exact 10-bardo.conf`. `check-system-list.sh` fails against the updater until
  The updater's `ALLOWED_SYSTEM_FILES` carries them in the same order, and a release that holds such a file cannot be installed by a helper
  that lacks them. The updater's branch pr1 (1c9273a) has them in that order (`release/check-system-list.sh --updater` reports the 21 rules equal), and its helper replaces a link into the install root where a file is placed (never writing through it) and refuses a release that both links and places a path; the pin and the helper on every computer move with it. Until the pinned the updater is that one, do not cut a release from this tree.
- **Units named by a caller row are plain files in a release.** A staged `lib/systemd/user/X.service` is exported as a link under
  `/usr/lib/systemd/user`; the unit manager's judge (`caller-identity-dbus` `fragment`, `trust_of_path` over `symlink_metadata`) reads a link
  (mode 0777) as `Writable`, so the unit never proves `Full` (the unverified note of NEW-PI-1 is now settled the other way for a link:
  it reads writable). `release/build.sh` `plain_units` stages every unit that a `unit =` row of `dist/desktop-callers.toml` names
  (the shell, the compositor, the portal, Settings, accountd, syncd, inferd, memoryd, intentd, companiond, cuad) as a system file at
  `/usr/lib/systemd/user/X.service`, with its `sandbox.conf` drop-ins appended to its text (the same to systemd, and a drop-in that is
  a link would also read as writable), and the portal frontend's drop-in at `.../xdg-desktop-portal.service.d/10-bardo.conf`; it
  stops if a named unit is still staged where it would be exported. The paths are in `release/stage-owners.list` by exact name.
  `install/install.sh` already copied bardo-compositor.service and shell.service as plain files to `/etc/systemd/user`; the other units of a
  source install are put in `<prefix>/lib/systemd/user` by their own installers and the printed hint for a prefix off the search
  paths still says `ln -sf`, which makes links that do not prove `Full` (left; change the hint to copy the named units when a
  source install is meant to run the services at `Full`). On a machine that already holds the old links, bardo-updater-apply must replace
  a link by a file at the same path (the updater's side).
- **The three rows.** `settings` (its unit row), `pad_daemon` and `keymap_daemon` already carried both a `unit` and an `exe`; the
  second `settings` row has no unit on purpose (a launcher; the updater asks for it at `Exe`). Nothing to change in the table beyond
  The mouse gestures daemon comment. `gate/check-callers-table.sh` and `gate/check-loader-unset.sh` pass.
- **Shift+Caps Lock** (`dist/key-remap.toml`): `{ when = "shift+capslock", then = "capslock" }` next to the Caps Lock rule. It moved from
  The shell's user layer because the key remap daemon refuses a Shift trigger in a user layer (NEW-PI-5). The shell must drop it from its forwarded layer.
