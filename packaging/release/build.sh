#!/usr/bin/env bash
# Build the components from this tree and stage them for a release bundle.
#
#   release/build.sh --version X [--out DIR] [--tree DIR] [--only NAME[,NAME]]
#
#   --version X     the release version (recorded in OUT/version; pack.sh takes it again)
#   --out DIR       default build/release under this folder. Gets stage/ (the destdir), target/ (cargo's
#                   output, cacheable) and components.d/<name>.txt ("name rev" lines for pack.sh; the rev is
#                   the tree's commit)
#   --tree DIR      the bardo tree (default $BARDO_TREE, else the parent of packaging/)
#   --only NAMES    just these (casement sill session porter steward ferry stoker almanac docket cua detent transom turnstile keycap palmrest files slipway satellite); the
#                   CI builds the heavy ones in separate jobs and merges the stage/ trees
#
# Everything is one cargo workspace: each component is built with `cargo build --release --locked -p ...` at the
# tree's root into one target directory ($CARGO_TARGET_ROOT, else OUT/target), and installed with the prefix
# /opt/bardo/current under the destdir OUT/stage, through the component's own installer where it can stage
# (porter, steward, ferry, stoker, almanac, docket, cua, detent, files, and the session in install/). The others (casement, sill, transom, turnstile,
# keycap, palmrest, slipway) have an installer that cannot stage without sudo or writes $HOME; their staging below is marked
# INTERIM and listed in FINDINGS.md with what each installer must learn. The tree's own commit is the version of every
# component: there are no pins to check. anyview and mailo are separate repositories that pin this tree; they
# are not built here.
# Builds use this system's own linker: install/cargo.sh runs cargo from / with --manifest-path, so the tree's
# .cargo/config.toml is not read.
# Exit: 0 staged, 1 a build or install failed, 2 bad usage.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
P=/opt/bardo/current
# shellcheck source=../install/cargo.sh
source "$ROOT/install/cargo.sh"
ALL=(casement sill session porter steward ferry stoker almanac docket cua detent transom turnstile keycap palmrest files slipway satellite)
# xwayland-satellite (MPL-2.0; a separate program casement starts for X11 apps), built from its own public repository at
# one commit, locked. Ubuntu 24.04 does not package it. Bump both lines together: SATELLITE_REV is the commit the tag
# names (`git ls-remote $SATELLITE_URL 'refs/tags/v0.8.3^{}'`), because a tag can be moved and a commit cannot.
# release/README.md and docs/distros.md name the tag.
SATELLITE_URL=https://github.com/Supreeeme/xwayland-satellite
SATELLITE_TAG=v0.8.3
SATELLITE_REV=b83eab900644e4c7c77982ce3d44cb490f0c5e1d
# The Rust it is built with: the channel the tree pins (the workflow installs it); satellite itself needs 1.85 or later.
SATELLITE_RUST=1.98.1

version="" out="$ROOT/build/release" tree="$(tree_root)" only=""
usage() { sed -n '2,/^set -euo/p' "${BASH_SOURCE[0]}" | sed '$d' | sed 's/^# \{0,1\}//'; }
die() { echo "build.sh: $*" >&2; exit "${2:-1}"; }
while [ $# -gt 0 ]; do
  case "$1" in
    --version) version=${2:?}; shift ;;
    --out) out=${2:?}; shift ;;
    --tree) tree=${2:?}; shift ;;
    --only) only=${2:?}; shift ;;
    -h|--help) usage; exit 0 ;;
    *) usage >&2; die "unknown argument $1" 2 ;;
  esac
  shift
done
[ -n "$version" ] || { usage >&2; die "--version is required" 2; }
mkdir -p "$out"; out=$(cd "$out" && pwd); tree=$(cd "$tree" && pwd)
export BARDO_TREE=$tree
stage="$out/stage"; root="$stage$P"
mkdir -p "$root" "$out/components.d"
printf '%s\n' "$version" >"$out/version"

selected=("${ALL[@]}")
if [ -n "$only" ]; then
  selected=()
  for n in ${only//,/ }; do
    case " ${ALL[*]} " in *" $n "*) selected+=("$n") ;; *) die "no component named $n (known: ${ALL[*]})" 2 ;; esac
  done
fi

tree_rev=$(git -C "$tree" rev-parse HEAD)

# repo_dir NAME: the folder of NAME (a component name) in the tree, under crates/; records "NAME rev" (the tree's commit) for pack.sh.
repo_dir() {
  local repo=$1 dir folder
  case "$repo" in
    casement) folder=compositor ;;
    sill) folder=shell ;;
    porter) folder=accounts ;;
    steward) folder=inference ;;
    ferry) folder=sync ;;
    stoker) folder=models ;;
    almanac) folder=memory ;;
    docket) folder=actions ;;
    cua) folder=computer-use ;;
    detent) folder=settings-app ;;
    transom) folder=portal ;;
    turnstile) folder=input/input-broker ;;
    keycap) folder=input/key-remap ;;
    palmrest) folder=input/mouse-gestures ;;
    slipway) folder=updater ;;
    *) folder=$repo ;;
  esac
  dir="$tree/crates/$folder"
  [ -d "$dir" ] || die "$repo: no folder at $dir (--tree says where the bardo tree is)" 2
  echo "$repo $tree_rev" >"$out/components.d/$repo.txt"
  printf '%s' "$dir"
}

# cargo_in ARGS...: a locked release build of the tree's workspace into the one target directory.
cargo_in() {
  echo "== cargo build --release --locked $*"
  (export CARGO_TARGET_DIR="${CARGO_TARGET_ROOT:-$out/target}"; build_tree "$@")
}
tdir() { printf '%s/release' "${CARGO_TARGET_ROOT:-$out/target}"; }

# put MODE SRC REL: install SRC at $root/REL.
put() { install -Dm"$1" "$2" "$root/$3"; }
# put_system MODE SRC DEST: a file bardo-updater-apply copies (as root) to the absolute DEST on the computer. It
# travels in the bundle at system/DEST and pack.sh lists it; DEST must be a file release/system-files.list (slipway's
# list, exactly) names: anything else is refused when the release is packed.
put_system() { put "$1" "$2" "system$3"; }
# retarget FILE FROM...: the unit's path to its program becomes the installed one. Each FROM is a path the unit may
# have started the program by (a unit may change its default between two revs of its repo); the one it has is replaced.
retarget() {
  local file=$1 from; shift
  for from in "$@"; do sed -i "s|^ExecStart=$from|ExecStart=$P/bin/$(basename "$from")|" "$file"; done
  grep -q "^ExecStart=$P/bin/" "$file" || die "$file: no ExecStart to retarget"
}

build_casement() {
  local d; d=$(repo_dir casement)
  cargo_in -p compositor-app -p compositor-ctl
  # INTERIM: casement has no installer here; it ships dist/bardo-compositor.service with ExecStart=/usr/bin/bardo-compositor
  # $BARDO_COMPOSITOR_SESSION_ARGS (retarget keeps the arguments). The unit is root-held, as the account service requires of a fixed
  # role: staged in lib/systemd/user here, and made a plain system file at /usr/lib/systemd/user by plain_units below.
  put 755 "$(tdir)/casement" bin/casement
  put 755 "$(tdir)/bardo-compositorctl" bin/bardo-compositorctl
  put 644 "$d/dist/bardo-compositor.service" lib/systemd/user/bardo-compositor.service
  # Its settings schema, so Settings draws its rows (Spaces: switch to an app's Space).
  local schema; schema=$(mktemp -d)
  "$(tdir)/casement" --write-schema "$schema" >/dev/null
  put 644 "$schema/compositor.settings.toml" share/quire/settings/compositor.settings.toml
  rm -rf "$schema"
  retarget "$root/lib/systemd/user/bardo-compositor.service" /usr/bin/bardo-compositor
}

build_sill() {
  local d; d=$(repo_dir sill)
  cargo_in -p shell-app
  # INTERIM: dist/install.sh has fixed destinations and sudo. Same files, under the prefix; the PAM
  # service is a system file (bardo-updater-apply places it), and the keycap layer sits under share/sill
  # (FINDINGS.md).
  put 755 "$(tdir)/sill" bin/sill
  put 644 "$d/dist/intents/org.bardo.Shell.toml" share/quire/intents/org.bardo.Shell.toml
  (cd "$d/dist/skills" && find . -type f | sort) | while IFS= read -r f; do
    put 644 "$d/dist/skills/$f" "share/quire/skills/${f#./}"
  done
  local schema; schema=$(mktemp -d)
  "$(tdir)/sill" --write-schema "$schema" >/dev/null
  put 644 "$schema/shell.settings.toml" share/quire/settings/shell.settings.toml
  rm -rf "$schema"
  # Its published style names, so Settings can tell a person's style using one from an internal name .
  local parts; parts=$(mktemp -d)
  "$(tdir)/sill" --write-selectors "$parts" >/dev/null
  put 644 "$parts/sill.toml" share/quire/selectors/sill.toml
  rm -rf "$parts"
  put_system 644 "$d/dist/pam.d/sill" /etc/pam.d/bardo-shell
  # External-monitor brightness (distro audit D7): the uaccess rule for the i2c buses and the list that loads
  # i2c-dev. Both folders are on slipway's list of system folders. Fedora's ddcutil package ships a rule with the
  # same tag; two rules with the same tag are harmless, so the install.sh "unless ddcutil has it" test is not needed.
  put_system 644 "$d/dist/udev/60-bardo-shell-ddc.rules" /etc/udev/rules.d/60-bardo-shell-ddc.rules
  put_system 644 "$d/dist/modules-load.d/bardo-shell-ddc.conf" /etc/modules-load.d/bardo-shell-ddc.conf
  put 644 "$d/dist/keycap/keycap.toml" share/sill/keycap.toml
  # The shell's unit, root-held like casement's (the account service trusts fixed roles only from such units; plain_units below).
  put 644 "$d/dist/bardo-shell.service" lib/systemd/user/bardo-shell.service
  retarget "$root/lib/systemd/user/bardo-shell.service" /usr/bin/bardo-shell
}

build_session() {
  echo "desktop $tree_rev" >"$out/components.d/desktop.txt"
  # The units bardo-compositor.service and bardo-shell.service are staged by their own components (they are not
  # necessarily in this job); the sessions entry lands in the exported directory, the user target in
  # lib/systemd/user (this installer puts it under <sysconfdir>/systemd/user).
  "$ROOT/install/install.sh" --prefix "$P" --destdir "$stage" --tree /nonexistent \
    --sysconfdir "$P/lib" --sessions-dir "$P/share/wayland-sessions"
  # The table of the desktop's own programs, a system file slipway places at /etc/bardo/desktop-callers.toml (the rule
  # `/etc/bardo/*-callers.toml` names it). slipway's update service, the portal backend and the input service read it
  # and refuse every caller without it, so it travels in every release.
  put_system 644 "$ROOT/dist/desktop-callers.toml" /etc/bardo/desktop-callers.toml
  # The colour emoji font (install.sh puts the same two files in <prefix>/share/fonts/quire/): ubuntu and arch ship
  # a bitmap build our renderer cannot draw. share/fonts is exported to /usr/share/fonts, where quire's search
  # (ui-blitz emoji_file.rs: <data dir>/fonts/quire/Noto-COLRv1.ttf for each XDG data dir, /usr/share included) finds it.
  put 644 "$ROOT/fonts/Noto-COLRv1.ttf" share/fonts/quire/Noto-COLRv1.ttf
  put 644 "$ROOT/fonts/OFL.txt" share/fonts/quire/OFL-Noto-COLRv1.txt
  # quire's look Extensions (Editorial), from the ui folder of this tree (install.sh above is given no tree, so it stages none).
  local q="$tree/crates/ui" f
  (cd "$q/extensions" && find . -type f | sort) | while IFS= read -r f; do
    put 644 "$q/extensions/$f" "share/quire/extensions/${f#./}"
  done
}

build_porter() {
  local d; d=$(repo_dir porter)
  cargo_in -p account-daemon
  # --sysconfdir under the bundle's system tree: the caller table lands at system/etc/porter/callers.toml, a system
  # file bardo-updater-apply places at /etc/porter/ (where porter reads it).
  (cd "$d" && CARGO_TARGET_DIR="${CARGO_TARGET_ROOT:-$out/target}" \
    ./scripts/install.sh --no-build --prefix "$P" --destdir "$stage" --sysconfdir "$P/system/etc")
}

build_steward() {
  local d; d=$(repo_dir steward)
  cargo_in -p inference-daemon
  # Same as porter's: system/etc/porter/inferd-callers.toml, a system file bardo-updater-apply places at /etc/porter/.
  (cd "$d" && CARGO_TARGET_DIR="${CARGO_TARGET_ROOT:-$out/target}" \
    ./scripts/install.sh --no-build --prefix "$P" --destdir "$stage" --sysconfdir "$P/system/etc")
}

build_ferry() {
  local d; d=$(repo_dir ferry)
  cargo_in -p sync-daemon -p drive-mount-daemon
  # ferry's installer has no --sysconfdir (it writes nothing under /etc: syncd's caller row is in porter's table). The units and
  # the activation file name the prefix's libexec.
  (cd "$d" && CARGO_TARGET_DIR="${CARGO_TARGET_ROOT:-$out/target}" \
    ./scripts/install.sh --no-build --prefix "$P" --destdir "$stage")
}

build_stoker() {
  local d; d=$(repo_dir stoker)
  # The installer copies the model catalogue to share/stoker/catalog, an exported folder (pack.sh links it under
  # /usr/share/stoker), and the engine installer's helper (bardo-engine-helper) to libexec/quire; both were built above.
  cargo_in -p engine-install
  (cd "$d" && CARGO_TARGET_DIR="${CARGO_TARGET_ROOT:-$out/target}" ./scripts/install.sh --no-build --prefix "$P" --destdir "$stage" --sysconfdir "$P/system/etc")
}

build_almanac() {
  local d; d=$(repo_dir almanac)
  cargo_in -p memory-daemon
  # The caller table
  # lands at system/etc/quire/memoryd-callers.toml, a system file bardo-updater-apply places at /etc/bardo. The unit and the
  # activation file name the prefix's libexec. Built above, so the installer only installs (--bin-dir).
  (cd "$d" && ./scripts/install.sh --bin-dir "$(tdir)" --prefix "$P" --destdir "$stage" --sysconfdir "$P/system/etc")
}

build_docket() {
  local d; d=$(repo_dir docket)
  cargo_in -p action-daemon -p companion-daemon -p agent-reader-daemon -p action-mcp -p action-cli -p agent-acp-bin
  # The caller tables land at system/etc/quire/*-callers.toml
  # and the default settings at system/etc/xdg/quire/, both system files slipway places (/etc/xdg/bardo needs
  # slipway >= aa427f2). Not built with --with-voice (the voice daemon needs the sound and clang development packages).
  (cd "$d" && ./scripts/install.sh --bin-dir "$(tdir)" --prefix "$P" --destdir "$stage" --sysconfdir "$P/system/etc")
}

build_cua() {
  local d; d=$(repo_dir cua)
  cargo_in -p computer-use-daemon
  # The daemon lands in libexec/quire, the unit (with its
  # protections) in lib/systemd/user, the activation files in share/dbus-1/services and the Settings schema in
  # share/quire/settings, all exported folders; the interface XML in share/dbus-1/interfaces stays in the bundle
  # unlinked (nothing reads it at run time). The caller table lands at system/etc/quire/computer-use-callers.toml, a system file
  # slipway places at /etc/bardo. Its dependencies build sqlite with a vendored openssl, which needs perl and make (deps.sh has them).
  (cd "$d" && ./scripts/install.sh --bin-dir "$(tdir)" --prefix "$P" --destdir "$stage" --sysconfdir "$P/system/etc")
}

build_detent() {
  local d; d=$(repo_dir detent)
  cargo_in -p settings-app
  CARGO_TARGET_DIR="${CARGO_TARGET_ROOT:-$out/target}" "$ROOT/install.sh" --tree "$tree" --prefix "$P" --destdir "$stage" --no-build --only detent
}

build_files() {
  local d; d=$(repo_dir files)
  cargo_in -p files --bin files
  # BARDO_FILES_PREFIX is the installed path (the service files name it), BARDO_FILES_DESTDIR the stage.
  (cd "$d" && BARDO_FILES_BIN="$(tdir)/files" BARDO_FILES_PREFIX="$P" BARDO_FILES_DESTDIR="$stage" ./dist/install.sh --no-build)
}

build_transom() {
  local d; d=$(repo_dir transom)
  cargo_in -p portal-daemon
  # The portal backend (screen sharing, screenshots). Same files and places as transom's scripts/install.sh,
  # under the prefix: the program in libexec, the user unit in lib/systemd/user, the D-Bus activation file
  # and the portal file in the exported share directories (pack.sh links those under /usr). Both the unit and
  # the activation file name /usr/libexec/bardo/bardo-portal; they must name the installed program. The routing file
  # (bardo-portals.conf) is the session's, staged by the session component.
  put 755 "$(tdir)/transom" libexec/transom
  put 644 "$d/dist/bardo-portal.service" lib/systemd/user/bardo-portal.service
  put 644 "$d/dist/org.freedesktop.impl.portal.desktop.bardo.service" \
    share/dbus-1/services/org.freedesktop.impl.portal.desktop.bardo.service
  put 644 "$d/dist/bardo.portal" share/xdg-desktop-portal/portals/bardo.portal
  local f
  for f in lib/systemd/user/bardo-portal.service share/dbus-1/services/org.freedesktop.impl.portal.desktop.bardo.service; do
    sed -i "s|/usr/libexec/bardo/bardo-portal|$P/libexec/transom|g" "$root/$f"
    grep -q "$P/libexec/transom" "$root/$f" || die "$f: no /usr/libexec/bardo/bardo-portal to retarget"
  done
}

build_turnstile() {
  local d; d=$(repo_dir turnstile)
  # The root service that makes palmrest's and keycap's virtual input devices. Only the daemon is built (-p input-broker-daemon,
  # no feature: its `fake` kernel and identity are for its tests and never ship; gate/check-no-fake.sh). The program is
  # exported to /usr/local/bin/bardo-input-broker (exports.toml), which the unit starts as shipped. The four files are system
  # files (turnstile's lane named them; slipway's list has them since its input-service change): the socket and the
  # service in the vendor folder, the rule that makes /dev/uinput root-only and takes the `uaccess` tag off it, and the
  # module list. Nothing here gives the node to the person or to the `input` group.
  cargo_in -p input-broker-daemon
  put 755 "$(tdir)/turnstile" bin/turnstile
  put_system 644 "$d/dist/bardo-input-broker.socket" /usr/lib/systemd/system/bardo-input-broker.socket
  put_system 644 "$d/dist/bardo-input-broker.service" /usr/lib/systemd/system/bardo-input-broker.service
  put_system 644 "$d/dist/72-bardo-input-broker.rules" /etc/udev/rules.d/72-bardo-input-broker.rules
  put_system 644 "$d/dist/bardo-input-broker-modules.conf" /etc/modules-load.d/turnstile.conf
}

build_keycap() {
  local d; d=$(repo_dir keycap)
  cargo_in -p key-remap-daemon
  # INTERIM: install.sh stages under DESTDIR but at /etc and /usr/local, outside the prefix. The unit and the polkit
  # rule are system files (bardo-updater-apply places them); the unit goes to /usr/lib/systemd/system (the vendor folder; /etc
  # stays the administrator's). On the desktop the input service (turnstile) owns /dev/uinput and hands keycap each
  # device, so none of keycap's standalone pieces for that node ship: not its 70-bardo-key-remap.rules (the `input` group may
  # write the node), not the drop-in dist/bardo-key-remap.uinput.conf (DeviceAllow=/dev/uinput), and not a module list of its own
  # (turnstile's turnstile.conf loads uinput). The unit is staged as keycap ships it, starting /usr/local/bin/bardo-key-remap, the
  # link exports.toml makes to bin/keycap (turnstile's unit does the same): that is also what install/system.sh writes at
  # the default prefix, so slipway sees the same file from a release and from install.sh (it replaces a system file only
  # when it is absent, equal or its own earlier one). The desktop's Caps Lock rule travels as keycap's vendor file,
  # /usr/lib/bardo/key-remap/key-remap.toml; /etc/keycap stays the administrator's, so no release writes a file there.
  put 755 "$(tdir)/keycap" bin/keycap
  put_system 644 "$d/dist/bardo-key-remap.service" /usr/lib/systemd/system/bardo-key-remap.service
  put_system 644 "$d/dist/bardo-key-remap.polkit.rules" /etc/polkit-1/rules.d/bardo-key-remap.polkit.rules
  put_system 644 "$ROOT/dist/keycap.toml" /usr/lib/bardo/key-remap/key-remap.toml
  put 644 "$d/dist/keycap.plasma.toml" share/keycap/keycap.plasma.toml
}

build_palmrest() {
  local d; d=$(repo_dir palmrest)
  cargo_in -p mouse-gestures-daemon
  # INTERIM: install.sh stages under DESTDIR but at /etc and /usr/local, outside the prefix. palmrest is a system service like
  # keycap (owner decision 2026-10-17): its unit is a system file in /usr/lib/systemd/system, staged as palmrest ships it,
  # starting /usr/local/bin/bardo-mouse-gestures, the link exports.toml makes to bin/palmrest (that is also what install/system.sh writes at
  # the default prefix, so slipway sees the same file from a release and from install.sh). The input service serves only a
  # palmrest only root can change, and believes it only in /system.slice/bardo-mouse-gestures.service. Its device rule gives the mouse's
  # raw node to the `input` group and to no person; no rule for /dev/uinput and no module list (turnstile's loads uinput).
  put 755 "$(tdir)/palmrest" bin/palmrest
  put_system 644 "$d/dist/bardo-mouse-gestures.service" /usr/lib/systemd/system/bardo-mouse-gestures.service
  put_system 644 "$d/dist/70-bardo-mouse-gestures.rules" /etc/udev/rules.d/70-bardo-mouse-gestures.rules
  put_system 644 "$d/dist/99-bardo-mouse-gestures.conf" /etc/modprobe.d/99-bardo-mouse-gestures.conf
}

build_slipway() {
  local d; d=$(repo_dir slipway)
  # updater-root is the crate of the three programs that run as root (bardo-updater-apply, bardo-updater-rollback,
  # bardo-updater-health); they are built here, in the same target folder, so the workflow can hand them to the smoke job.
  cargo_in -p updater-daemon -p updater-root
  # INTERIM: slipway's scripts/install.sh installs the helpers, polkit action, unit and bus file on a machine
  # (first install, release/bootstrap.sh); the release carries only the user program. bardo-updater-apply,
  # bardo-updater-rollback and bardo-updater-health are not in the bundle: a release must not replace a binary that runs as root.
  put 755 "$(tdir)/slipway" bin/slipway
}

build_satellite() {
  # No checkout beside the repos: the commit is fetched once into a scratch folder, and the program is built from that
  # folder (so the code, the licence text and the notice all come from the same commit). Installed into the scratch root
  # and only the program is copied, so cargo's own bookkeeping files stay out of the bundle. Run from / so no tree's cargo
  # config is read. It lands in bin/ and is exported to /usr/local/bin (exports.toml) so casement finds it on the
  # session's PATH. The program is unmodified MPL-2.0 code: the bundle carries the licence text from the same commit and a
  # notice saying where the source is.
  local croot; croot=$(mktemp -d)
  git -C "$croot" init -q src
  git -C "$croot/src" fetch -q --depth 1 "$SATELLITE_URL" "$SATELLITE_REV" || die "xwayland-satellite: cannot fetch $SATELLITE_REV"
  git -C "$croot/src" checkout -q --detach FETCH_HEAD
  [ "$(git -C "$croot/src" rev-parse HEAD)" = "$SATELLITE_REV" ] || die "xwayland-satellite: fetched a commit other than $SATELLITE_REV"
  echo "== cargo install --locked --path $croot/src (xwayland-satellite $SATELLITE_TAG at $SATELLITE_REV)"
  (cd / && RUSTUP_TOOLCHAIN="$SATELLITE_RUST" CARGO_TARGET_DIR="${CARGO_TARGET_ROOT:-$out/target}/satellite" \
    cargo install --locked --path "$croot/src" --root "$croot")
  put 755 "$croot/bin/xwayland-satellite" bin/xwayland-satellite
  head -n 1 "$croot/src/LICENSE" | grep -q 'Mozilla Public License' || die "xwayland-satellite $SATELLITE_REV: LICENSE is not the Mozilla Public License"
  put 644 "$croot/src/LICENSE" share/licenses/xwayland-satellite/LICENSE
  local notice; notice=$(mktemp)
  printf 'xwayland-satellite %s (commit %s), licence MPL-2.0 (the text is the LICENSE file beside this one), shipped unmodified as a separate program.\nSource: %s/tree/%s\n' \
    "$SATELLITE_TAG" "$SATELLITE_REV" "$SATELLITE_URL" "$SATELLITE_REV" >"$notice"
  put 644 "$notice" share/licenses/xwayland-satellite/NOTICE.txt
  rm -rf "$croot"
  rm -f "$notice"
  echo "xwayland-satellite $SATELLITE_REV" >"$out/components.d/xwayland-satellite.txt"
}

# plain_units: every user unit that dist/desktop-callers.toml names (a `unit =` row, so it can be proven at `Full`) travels as a
# system file that bardo-updater-apply copies, as a plain root-owned file, to /usr/lib/systemd/user. A file staged in lib/systemd/user
# is exported as a link into /opt/bardo/current instead, and the unit manager's judge reads a link as writable (its mode is
# 0777), so a unit laid out that way, or with a drop-in that is such a link, is refused at `Full`: the shell, the compositor,
# the portal backend and the accounts services would not be trusted. The drop-ins of a unit staged here (the sandbox.conf
# of each background service) are appended to the unit's own text, in file-name order, which systemd reads the same as
# the drop-in; a drop-in of a unit staged elsewhere (the portal frontend is the distribution's) is a system file of its own, which
# a rule of release/system-files.list must name. Other user units (mailo-watch.service, bardo-session.target) stay exported
# links. Runs in every job on whatever that job staged.
callers_units() { awk '$1 == "unit" { gsub(/"/, "", $3); print $3 }' "$ROOT/dist/desktop-callers.toml" | sort -u; }
plain_units() {
  local unit src dropin merged
  for unit in $(callers_units); do
    src="$root/lib/systemd/user/$unit"
    if [ -f "$src" ]; then
      merged=$(mktemp)
      cat "$src" >"$merged"
      if [ -d "$src.d" ]; then
        for dropin in "$src.d"/*; do
          [ -f "$dropin" ] || die "$dropin: a drop-in of $unit that is not a plain file"
          { printf '\n# From %s.d/%s\n' "$unit" "$(basename "$dropin")"; cat "$dropin"; } >>"$merged"
          rm -f -- "$dropin"
        done
        rmdir "$src.d"
      fi
      put_system 644 "$merged" "/usr/lib/systemd/user/$unit"
      rm -f -- "$merged" "$src"
    elif [ -d "$src.d" ]; then
      for dropin in "$src.d"/*; do
        [ -f "$dropin" ] || die "$dropin: a drop-in of $unit that is not a plain file"
        put_system 644 "$dropin" "/usr/lib/systemd/user/$unit.d/$(basename "$dropin")"
        rm -f -- "$dropin"
      done
      rmdir "$src.d"
    fi
  done
  # Nothing named by the table may stay behind as an exported link.
  for unit in $(callers_units); do
    [ ! -e "$root/lib/systemd/user/$unit" ] && [ ! -e "$root/lib/systemd/user/$unit.d" ] || die "$unit is still staged in lib/systemd/user, where it would be exported as a link"
  done
}

for name in "${selected[@]}"; do
  echo "=== $name"
  "build_$name"
done
plain_units

# Nothing may land outside the prefix, and the bundle has no links.
stray=$(cd "$stage" && find . -mindepth 1 -not -path "./opt" -not -path "./opt/quire" -not -path "./opt/bardo/current*" | head -n 3)
[ -z "$stray" ] || die "files outside $P: $stray"
links=$(find "$stage" -type l | head -n 3)
[ -z "$links" ] || die "symlinks in the stage (the bundle has none): $links"
echo "build.sh: staged ${selected[*]} in $stage"
