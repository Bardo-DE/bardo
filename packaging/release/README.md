# Releases

A signed binary bundle of the whole desktop, built from one commit of the bardo tree by `build.sh`, packed and signed by
`pack.sh`. bardo-updater, the updater, reads it (its default feed is the public repo `PoHsuanLai/desktop-releases`).

**Release CI is not set up in bardo yet.** There is no workflow in this tree (the old one lived in the desktop
repo and was not imported), no signing key is committed (`release.pub`), and no secrets, tokens or environments exist for
this repository. Until they do, a release is cut by hand with the local steps below. Several rows below say "the workflow"
or "the CI": that is the workflow still to be written; the scripts themselves work on their own. `release/selftest.sh` also
reads `.github/workflows/release.yml` at its end and fails without it.

| File | What |
|---|---|
| `build.sh` | builds each component from this tree (`cargo build --release --locked -p ...`, run from `/` with `--manifest-path` so the system linker is used, into one target directory; see `install/cargo.sh`) and installs it under `/opt/bardo/current` into a staging root |
| `pack.sh` | stage -> `release.toml` (digests, modes, components, exports), `bundle.tar.zst` (deterministic), `release.toml.minisig` |
| `verify.sh` | checks a packed release the way bardo-updater will; the CI runs it before publishing |
| `system-files.list` | the system files a release may place, one rule each (a committed copy of bardo-updater's `ALLOWED_SYSTEM_FILES`); `release.py` reads it, so `pack` and `verify` refuse a `system/` path no rule names |
| `check-system-list.sh` | fails unless that list equals bardo-updater's own (the workflow runs it; the bardo-updater crate is in this tree) |
| `check-helpers.sh` | fails unless bardo-updater's own install is complete: `bardo-updater-apply`, `bardo-updater-rollback` and `bardo-updater-health` in `/usr/libexec`, and the one polkit file with its three actions (installing and the good-start note need no password at the keyboard, going back asks for the administrator's); `smoke.sh` and `bootstrap.sh` run it |
| `health-scenarios.sh` | two scenarios with bardo-updater's real programs on a throwaway computer: two logins that run `recover` and `confirm` keep the newest release, and restarts that never confirm put the previous release back on the third; the workflow's `health` job runs it in a privileged container, and publishing waits for it |
| `selftest.sh` | packs a fake stage with a throwaway key, verifies it, and checks that hostile releases are refused; a SKIP (exit 2) without minisign; run by `gate/desktop-gate.sh --self` |
| `check-glibc.sh` | fails if a program in the stage needs a newer glibc (default 2.39), libstdc++ (GCC 13) or libgcc ABI than Ubuntu 24.04 has; the workflow runs it after each build |
| `check-ldd.sh` | fails if `ldd` reports a library "not found" for any program in a tree; run after the build and again inside each distribution |
| `deps.sh` | the packages the flow needs per family (debian, fedora, arch): `tools`, `build`, `runtime` |
| `install-rustup.sh` | rustup from its published program for one version, SHA-256 checked (replaces `curl https://sh.rustup.rs | sh` in the workflow) |
| `install-zig.sh` | Zig 0.16 from the official tarball, SHA-256 checked (temor builds with it; no distribution ships a fixed 0.16) |
| `smoke.sh` | installs a packed release with bardo-updater's helper on a throwaway machine (a CI container) and checks the links, system files, libraries and a repeat install |
| `lib.sh` | sourced by the scripts: the distribution family and the one-line install hint for a missing tool |
| `bootstrap.sh` | first install on a machine: downloads the latest release, verifies it, runs bardo-updater's helper |
| `exports.toml` | exports beyond the ones derived from the staged tree (`/usr/local/bin` links) |
| `release.pub` | the public key (not committed yet; see "What you do once") |

## The format (frozen; bardo-updater parses exactly this)

Tag `v<semver>`; assets `release.toml`, `release.toml.minisig`, `bundle.tar.zst`.

`release.toml`: `format = 1`, `version`, `date`, `notes`, `bundle = { name, sha256, size }`, a `[components]`
table (component -> rev, from build.sh's `components.d/`: the tree's commit), one `[[file]]`
(`path`, `sha256`, `mode = 0o755`) per regular file sorted by path, and `[[export]]` (`link`, `target`
relative to `/opt/bardo/current`). The bundle is a tar.zst of regular files and directories only (no links),
clean relative paths, owner 0, mtime 0, modes 0644 or 0755. Exports may only go under
`/usr/share/wayland-sessions`, `/usr/share/xdg-desktop-portal`, `/usr/share/dbus-1/services`,
`/usr/lib/systemd/user`, `/usr/share/applications`, `/usr/share/bardo`,
`/usr/share/fonts` and `/usr/local/bin`; every file staged under
the first seven directories (as `share/...` or `lib/systemd/user`) is exported to the same path under `/usr`,
and `exports.toml` adds the rest. There is no polkit actions folder in the list: an action can make `pkexec` run a program as root with no
password, so no release can add one (bardo-updater's own comes from its `scripts/install.sh`).

`[[system]]` (`path`): a file bardo-updater's helper copies, as root, to an absolute path on the computer. Its
contents are the bundle file `system/<path>` (so `system/etc/pam.d/bardo-shell` lands at `/etc/pam.d/bardo-shell`); every
file under `system/` must be listed and every listed path needs its file. The path must be named by a rule of
`system-files.list`, which is bardo-updater's `ALLOWED_SYSTEM_FILES` word for word: nineteen rules, each a folder and either an
exact file name or a name ending (`/etc/bardo/*-callers.toml`, `/etc/xdg/bardo/*.toml`). A path no rule names
refuses the whole release, at `pack`, at `verify` and in the helper. Because the helper is installed by hand and is
never replaced by a release, adding a system file means changing bardo-updater's list, installing the new helper on every
computer, and changing `system-files.list` in the same step (`check-system-list.sh` fails the build if the two differ).
`build.sh` stages bardo-key-remap's unit and polkit rule (no device rule and no module list of its own: the input service owns `/dev/uinput`),
the input service's `bardo-input-broker.socket`, `bardo-input-broker.service`, `72-bardo-input-broker.rules` (the node becomes root-only and loses the `uaccess` tag)
and `bardo-input-broker.conf` (loads `uinput`), with its `/usr/local/bin/bardo-input-broker` export (`exports.toml`), the table of the desktop's own programs
`/etc/bardo/desktop-callers.toml` (from `dist/`; the update service, the screen-sharing service and the input service refuse every caller
without it),
bardo-mouse-gestures's unit (a system service since 2026-10-17, like bardo-key-remap), its udev rule (the mouse's raw node to the `input` group, no person) and module options, bardo-shell's PAM service and its external-monitor brightness files (the udev rule
`60-bardo-shell-ddc.rules` and the `i2c-dev` module list) the accounts', memory's, actions' and computer-use's caller tables (`/etc/bardo`) and the actions' default settings (`/etc/xdg/bardo`) there. The session component also
stages the colour emoji font at `share/fonts/bardo/Noto-COLRv1.ttf` (with its licence), which is exported to
`/usr/share/fonts/bardo/`, a folder the desktop's emoji search reads.

A unit that a row of `dist/desktop-callers.toml` names (the shell, the compositor, the portal backend, the accounts, intent, companion and memory services, Settings, the
computer-use daemon) must be a **plain root-owned file**, not a link: a staged `lib/systemd/user/X.service` is exported as a link under `/usr/lib/systemd/user`, and the
unit manager's judge reads a link (mode 0777) as a fragment anyone could change, so the unit never proves `Full` and its callers are refused. `build.sh`'s `plain_units`
therefore stages those units, with the `sandbox.conf` drop-ins appended to their text, as system files at `/usr/lib/systemd/user/X.service` (the rule `/usr/lib/systemd/user
ending .service` of the list), and the portal frontend's drop-in at `/usr/lib/systemd/user/xdg-desktop-portal.service.d/10-bardo.conf`; the build stops if a named unit is
still staged where it would be exported. Units that no row names (`mailo-watch.service`, the session target) stay exported links. The system units of bardo-input-broker, bardo-key-remap
and bardo-mouse-gestures are system files in `/usr/lib/systemd/system` and were never links.

## What you do once

Nothing is set up for bardo yet. What a release needs, whichever way it is later automated:

1. A minisign key pair (`minisign -G -p release.pub -s release.key`), the secret key kept out of every repo and protected
   by a password. The public key is committed as `release/release.pub`: `verify.sh` checks against it, and the file bardo-updater reads
   on a machine is `/usr/share/bardo/updater/release.pub` (the same file). There is one signing key, and bardo-updater's privileged helper
   trusts exactly that key file, so replacing a lost or stolen key means each person runs bardo-updater's install with the new
   `--pubkey` again.
2. A place to publish the three files (`release.toml`, `release.toml.minisig`, `bundle.tar.zst`): bardo-updater's feed and
   `bootstrap.sh` currently point at `PoHsuanLai/desktop-releases`.

CI (the build matrix, the glibc and library checks, the install in clean Ubuntu, Fedora and Arch containers, the publishing
secrets) is to be written; this file will describe it when it exists.

## Cutting a release

Commit what you want released (every component is built from the tree's commit), then by hand, with `MINISIGN_KEY_FILE` set:

```
release/build.sh --version 0.2.0 --out /tmp/rel
cat /tmp/rel/components.d/*.txt >/tmp/rel/components.txt
release/pack.sh --stage /tmp/rel/stage --version 0.2.0 --notes notes.txt --out /tmp/rel/out --components /tmp/rel/components.txt
release/verify.sh /tmp/rel/out --pubkey release/release.pub
```

The notes for the next release are kept in `docs/release-notes.md` while it is prepared; use them as the text. `build.sh --only NAMES`
builds a part (compositor, shell, session, accounts, inference, sync, models, memory, actions, computer-use, settings, portal, input-broker, key-remap, mouse-gestures,
files, bardo-updater, satellite); xwayland-satellite is built from its public repository at the commit in `build.sh`, `SATELLITE_REV`, the one
`SATELLITE_TAG` names. anyview and mailo are separate repositories and are not built here.

If the build is later split into parallel jobs, each job may supply only the paths `release/stage-owners.list` gives it. Before the stage tars are merged, the pack job runs
`release/stage-owners.py check` on them and stops when a tar holds a path outside its job's lines (a build script in one job writing
`bin/bardo-input-broker` or `bardo-input-broker.service`, say), anything that is not a plain file or folder, or a path that two jobs both supply. A component
that starts to install a new file needs a line for it in that list. The job is taken from the artifact, not from a file name: the pack job
downloads each `stage-JOB` artifact into a folder of its own (no `merge-multiple`) and the check refuses a folder that holds anything but
exactly its own `stage-JOB.tar`, so an artifact `stage-x` holding a file `stage-input-broker.tar` is not bardo-input-broker's.

## First install on a machine

bardo-updater installs the desktop, so bardo-updater's own pieces come first. Its `scripts/install.sh` puts three privileged
programs in `/usr/libexec`, the one polkit file that names them, the user unit and bus file, the system settings and the
release key at `/usr/share/bardo/updater/release.pub`:

| Program | What it does | Password |
|---|---|---|
| `bardo-updater-apply` | installs a signed release that is newer than the one in use | none, for the person at the keyboard (it checks the signature itself) |
| `bardo-updater-rollback` | goes back to the previous release | the administrator's, kept for a few minutes |
| `bardo-updater-health` | keeps the record of which starts went well and puts the previous release back after several failed ones | none, and only root can write its record |

Then `release/bootstrap.sh` fetches the latest release from desktop-releases, checks it with `verify.sh` against that key,
checks bardo-updater's install with `check-helpers.sh`, and runs `pkexec /usr/libexec/bardo/bardo-updater-apply` on it:

```
cd bardo
cargo build --release --locked -p updater-root
sudo sh -c 'umask 022 && crates/updater/scripts/install.sh --bin-dir target/release --pubkey /path/to/release.pub'
packaging/release/bootstrap.sh
```

The build runs as yourself, so no build step of the dependency tree runs as root, and the install gets the finished
programs with `--bin-dir`; without `--bin-dir` bardo-updater's script builds them itself, as whoever runs it, which under `sudo`
means as root. The `umask 022` keeps `/etc/bardo-updater` and `/usr/share/bardo-updater` readable by your own session when root's umask is
stricter (the script writes with the umask it is run under, and the update service could not then read its settings or the
key). None of the three programs is part of any
release (a release must not replace a program that runs as root); later releases arrive through bardo-updater's daily check, and a
change to a helper is a new run of bardo-updater's install by hand. `release/bootstrap.sh --verify-only` downloads and checks
without installing. The way back by hand, from a text console, is `pkexec /usr/libexec/bardo/bardo-updater-rollback`.

From then on the session does the rest: `bardo-session.target` wants the update service, `bardo-session` runs
`bardo-updater recover` before the compositor starts, and the shell runs `bardo-updater confirm` about half a minute after it
answers. `smoke.sh` covers the install on a throwaway machine and `health-scenarios.sh` the two start-check cases; neither runs in CI yet.

### Before the first release that carries the input service

Release N is the first to carry the four bardo-input-broker files. bardo-updater's privileged helper is never replaced by a release, and an older one refuses
a release that lists a file its rule list does not name, so **everyone re-runs bardo-updater's install by hand before that release** (the
`cargo build ... -p updater-root && sudo sh -c 'umask 022 && crates/updater/scripts/install.sh ...'` lines above, with a bardo-updater that has the four rules and, in
`apply/settle.rs`, the step that reloads systemd and the device rules and starts `bardo-input-broker.socket`). A computer that skips it keeps working
at the version it has: the release is refused whole and says why. `release/system-files.list` and the bardo-updater crate of this tree move in
the same step; until then `release/check-system-list.sh` fails against the old pin, on purpose.

Carried by a release: `/usr/lib/bardo/key-remap/key-remap.toml` (Caps Lock switches the input method), bardo-key-remap's vendor file. `/etc/bardo/key-remap/key-remap.toml` stays the
administrator's and no release or install writes it; bardo-key-remap lays it over the vendor file. The updater's list gained the rule after the first release that
carries the input service, so a computer whose helper is older than that rule must re-run bardo-updater's install by hand before the release that lists it
(an older helper refuses that release whole). `./install.sh` places the same file (`install/system.sh bardo-key-remap`, replacing it each time).

## Local trial

`release/selftest.sh` needs only python3, zstd and minisign. A real local build is
`release/build.sh --version 0.0.0-dev --out /tmp/rel` (from this tree; the tree's commit is each component's rev), then `release/pack.sh --stage /tmp/rel/stage --version 0.0.0-dev --notes notes.txt
--out /tmp/rel/out --components /tmp/rel/components.txt` (`cat /tmp/rel/components.d/*.txt >components.txt`) with
`MINISIGN_KEY_FILE` set, and `release/verify.sh /tmp/rel/out --pubkey release/release.pub`.

## Limits worth knowing

- GitHub-hosted runners have about 14 GB of free disk and a 6 hour job limit; bardo-compositor and bardo-shell are the
  heavy builds and each has its own job and its own cache. The repo cache limit is 10 GB: if the cache
  evicts, builds get slower, not wrong.
- The signing needs minisign 0.11 or later (it signs prehashed by default; `pack.sh` also passes `-H`, and
  `verify.sh` refuses a legacy signature, which is what bardo-updater does).
- bardo-updater's limits are checked by `pack.sh` and `verify.sh`: bundle 1 byte to 8 GiB, at most 100000 files, files
  totalling at most 16 GiB, notes at most 16 KiB, `release.toml` at most 24 MiB; `.bardo-updater/` is a reserved
  path; modes must be readable by the owner and not writable by group or others; exports point at files.
- The signature covers `release.toml`; the digests in it cover the bundle and every file.
