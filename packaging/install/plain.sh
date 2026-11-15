#!/usr/bin/env bash
# The plain installer behind `cargo xtask install`: build the whole desktop once from this tree as you, then install it,
# one line per part. Each part's whole output goes to the log; a failure prints the log's last lines and stops.
#
#   install/plain.sh build   [PART...]   build as you (cargo, release, this system's linker): one cargo run, one target dir
#   install/plain.sh install [PART...]   build, then install; sudo asks once, at the start
#   install/plain.sh parts               print every part, in install order
#
# With no PART, every part in install order. PREFIX (default /usr/local) and BARDO_TREE (default the parent of
# packaging/) come from the environment. The tree is one cargo workspace: every program is built by one
# `cargo build --release -p ...` at its root into <tree>/target (or $CARGO_TARGET_DIR), and each part's own installer
# (crates/<part>/scripts/install.sh, or install/system.sh) is run with --no-build to place what was built, so nothing
# builds as root.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
desktop=$(dirname "$here")
# shellcheck source=install/cargo.sh
. "$here/cargo.sh"
tree=$(tree_root)
prefix=${PREFIX:-/usr/local}
target=${CARGO_TARGET_DIR:-$tree/target}
export CARGO_TARGET_DIR=$target
log=${XDG_STATE_HOME:-$HOME/.local/state}/desktop-install.log

all=(callers porter steward ferry stoker almanac docket cua detent casement sill transom turnstile keycap files palmrest session)

[ "${1:-}" != parts ] || { echo "${all[*]}"; exit 0; }

# install_part PART: install PART from what was built in $target. As root (sudo) only where the installer wants
# root for the whole run; the rest run as you and use the sudo already granted. sudo drops the environment, so the
# root ones pass CARGO_TARGET_DIR through env.
install_part() {
  local c=$tree/crates p=$prefix d=$desktop
  case $1 in
    callers) sudo "$d/install/system.sh" callers ;;
    porter) sudo env CARGO_TARGET_DIR="$target" "$c/porter/scripts/install.sh" --prefix "$p" --sysconfdir /etc --no-build ;;
    steward) sudo env CARGO_TARGET_DIR="$target" "$c/steward/scripts/install.sh" --prefix "$p" --sysconfdir /etc --no-build ;;
    ferry) sudo env CARGO_TARGET_DIR="$target" "$c/ferry/scripts/install.sh" --prefix "$p" --no-build ;;
    stoker) "$c/stoker/scripts/install.sh" --prefix "$p" --sysconfdir /etc --no-build ;;
    almanac) "$c/almanac/scripts/install.sh" --prefix "$p" --no-build ;;
    docket) "$c/docket/scripts/install.sh" --prefix "$p" --no-build ;;
    cua) "$c/cua/scripts/install.sh" --prefix "$p" --no-build ;;
    detent) sudo env PREFIX="$p" DETENT_BIN="$target/release/detent" "$c/detent/scripts/install.sh" ;;
    casement) sudo env CARGO_TARGET_DIR="$target" "$c/casement/scripts/install.sh" --prefix "$p" --locked --no-build ;;
    sill) "$c/sill/dist/install.sh" --no-build ;;
    transom) sudo env CARGO_TARGET_DIR="$target" "$c/transom/scripts/install.sh" --prefix "$p" --no-build ;;
    turnstile | keycap | palmrest) sudo env CARGO_TARGET_DIR="$target" "$d/install/system.sh" "$1" --prefix "$p" --tree "$tree" ;;
    # Its installer builds as you and asks for sudo itself only when the prefix is not yours.
    files) FILES_PREFIX=$p "$c/files/dist/install.sh" --no-build ;;
    session) sudo "$d/install/install.sh" --prefix "$p" --sysconfdir /etc --tree "$tree" ;;
  esac
}

# quietly LABEL CMD...: one line for the step, the output to the log.
quietly() {
  local label=$1
  shift
  printf '  %-10s %s… ' "$label" "$verb"
  { echo; echo "=== $label: $verb ($(date -Is))"; } >>"$log"
  if (cd "$tree" && "$@") >>"$log" 2>&1 </dev/null; then
    echo done
  else
    printf 'FAILED\n\n'
    tail -n 25 "$log" | sed 's/^/    /'
    printf '\n  the whole output is in %s\n' "$log"
    exit 1
  fi
}

mode=${1:?usage: plain.sh build|install|parts [PART...]}
shift
parts=("$@")
[ ${#parts[@]} -gt 0 ] || parts=("${all[@]}")
for part in "${parts[@]}"; do
  case " ${all[*]} " in *" $part "*) ;; *) echo "no part named $part (the parts: ${all[*]})" >&2; exit 2 ;; esac
done
mkdir -p "$(dirname "$log")"
: >"$log"

verb=building
quietly "all parts" build_parts "${parts[@]}"
[ "$mode" = install ] || exit 0

echo "  sudo asks for your password once:"
sudo -v
while sleep 50; do sudo -n true 2>/dev/null || exit; done &
keepalive=$!
trap 'kill "$keepalive" 2>/dev/null || true' EXIT
verb=installing
for part in "${parts[@]}"; do quietly "$part" install_part "$part"; done

# Your user manager outlives a logout, so it would keep the units it read before this install and the
# background services' old programs. Reload it, restart the services that are running (the session's
# own casement, sill and transom wait for the next login; Settings and the apps for their next start)
# and start the sockets the install linked in sockets.target.wants.
daemons=(accountd inferd syncd memoryd intentd companiond readerd cuad actions-mcp docket-agent)
reload_user() {
  systemctl --user daemon-reload &&
    systemctl --user try-restart "${daemons[@]/%/.service}" &&
    for wants in "$prefix/lib/systemd/user/sockets.target.wants"/*.socket; do
      [ -e "$wants" ] || continue
      systemctl --user start "$(basename "$wants")" || return
    done
}
if systemctl --user show-environment >/dev/null 2>&1; then
  verb=reloading
  quietly services reload_user
fi
echo
echo "  installed to $prefix. Log out and back in to \"sill (casement)\" to use it."
