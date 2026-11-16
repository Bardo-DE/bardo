#!/usr/bin/env bash
# Build bardo-mouse-gestures and install it as a system service.
#
#   ./install.sh [--dry-run] [--prefix P] [--destdir D] [--no-build]
#
#   (no flags)    the program in /usr/local/bin (a root-owned file: the input service serves only a
#                 bardo-mouse-gestures that only root can change), its unit in /etc/systemd/system, the device
#                 rule, the kernel module options and the module-load file, all copied through $SUDO,
#                 sudo, doas or pkexec (or printed, when none is found, for you to run as root).
#                 Then the service is enabled and started the same way. The input service
#                 (bardo-input-broker) comes with the desktop and makes the virtual devices; bardo-mouse-gestures has no
#                 access to /dev/uinput. The desktop's own installer (install/system.sh bardo-mouse-gestures)
#                 does the same for a desktop install.
#   --prefix P    a packaged layout under P: P/bin/bardo-mouse-gestures, the unit in P/lib/systemd/system (its
#                 ExecStart names P/bin/bardo-mouse-gestures), and the system files in P/lib/udev/rules.d,
#                 P/lib/modprobe.d and P/lib/modules-load.d. Nothing is enabled or started. Root is
#                 used for the copy only when needed ($SUDO, sudo, doas or pkexec, whichever is
#                 found first).
#   --destdir D   stage the packaged layout under D as you (no root, no systemd, no udev); the
#                 prefix defaults to /usr/local. $DESTDIR works too.
#   --no-build    use the binary that is already built (or $BARDO_MOUSE_GESTURES_BIN)
#   --dry-run     print every command, change nothing (does not build)
#
# Never run by the build or the tests: it changes which program owns the mouse's scrolling.
set -euo pipefail

UNIT=bardo-mouse-gestures.service
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TREE="$(cd "$HERE/../.." && pwd)"   # the bardo tree
DESTDIR="${DESTDIR:-}"
PREFIX=""
BUILD=yes
DRY_RUN=no

say() { printf '%s\n' "$*"; }

absolute_arg() {
  case "$2" in
    /*) local v="${2%/}"; printf '%s' "${v:-/}" ;;
    *) say "install.sh: $1 needs an absolute path"; exit 2 ;;
  esac
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run) DRY_RUN=yes ;;
    --prefix) PREFIX="$(absolute_arg --prefix "${2:?--prefix needs a directory}")"; shift ;;
    --destdir) DESTDIR="$(absolute_arg --destdir "${2:?--destdir needs a directory}")"; shift ;;
    --no-build) BUILD=no ;;
    -h|--help) sed -n '2,24p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) say "install.sh: unknown argument $1 (try --help)"; exit 2 ;;
  esac
  shift
done
[[ -z "$DESTDIR" || -n "$PREFIX" ]] || PREFIX=/usr/local

# The words that put a command in front of root ($SUDO may hold arguments), empty when none.
ESCALATE=()
pick_escalator() {
  local candidate words
  for candidate in "${SUDO:-}" sudo doas pkexec; do
    [[ -n "$candidate" ]] || continue
    read -ra words <<<"$candidate"
    if command -v "${words[0]}" >/dev/null 2>&1; then
      ESCALATE=("${words[@]}")
      return 0
    fi
  done
  return 0
}
pick_escalator

# The escalation word for a printed command line: the helper found, or nothing (run as root).
printed_root() {
  if [[ "$(id -u)" == 0 ]]; then return 0; fi
  if (( ${#ESCALATE[@]} > 0 )); then printf '%s ' "${ESCALATE[@]}"; else printf ''; fi
}

# set_layout: where each file goes. The standalone install puts everything in /usr/local/bin and
# /etc (root-owned); a prefix is a package layout with the vendor directories.
set_layout() {
  if [[ -z "$PREFIX" ]]; then
    LAYOUT=system
    BINDIR=/usr/local/bin
    UNITDIR=/etc/systemd/system
    UDEVDIR=/etc/udev/rules.d
    MODPROBEDIR=/etc/modprobe.d
    MODULESDIR=/etc/modules-load.d
  else
    LAYOUT=package
    BINDIR="$PREFIX/bin"
    UNITDIR="$PREFIX/lib/systemd/system"
    UDEVDIR="$PREFIX/lib/udev/rules.d"
    MODPROBEDIR="$PREFIX/lib/modprobe.d"
    MODULESDIR="$PREFIX/lib/modules-load.d"
  fi
}
set_layout

# run DESCRIPTION CMD...: print, then run unless this is a dry run.
run() {
  local what="$1"; shift
  say "  $what"
  say "      \$ $(printf '%q ' "$@")"
  [[ "$DRY_RUN" == yes ]] || "$@"
}

# put_file MODE SRC DST: copy SRC to DST (under DESTDIR), through the escalation helper when a
# machine path needs root and we are neither root nor staging.
put_file() {
  local mode="$1" src="$2" dst="$3"
  if [[ -n "$DESTDIR" || "$(id -u)" == 0 ]]; then
    run "install $dst" install -Dm"$mode" "$src" "$DESTDIR$dst"
  elif (( ${#ESCALATE[@]} > 0 )); then
    run "install $dst" "${ESCALATE[@]}" install -Dm"$mode" "$src" "$dst"
  else
    say "  install $dst"
    say "      (no sudo, doas or pkexec here: run this as root)"
    say "      \$ install -Dm$mode $src $dst"
  fi
}

BIN="${BARDO_MOUSE_GESTURES_BIN:-${CARGO_TARGET_DIR:-$TREE/target}/release/bardo-mouse-gestures}"
if [[ "$BUILD" == yes ]]; then
  if [[ "$DRY_RUN" == yes ]]; then
    say "  would run: cargo build --release --locked -p mouse-gestures-daemon (in $TREE)"
  else
    (cd "$TREE" && cargo build --release --locked -p mouse-gestures-daemon)
  fi
fi

# The unit names the installed program, never the DESTDIR-prefixed path, and always a root-owned one.
RENDERED="$(mktemp -d)"
trap 'rm -rf "$RENDERED"' EXIT
sed "s|^ExecStart=.*|ExecStart=$BINDIR/bardo-mouse-gestures|" "$HERE/dist/bardo-mouse-gestures.service" >"$RENDERED/$UNIT"

put_file 755 "$BIN" "$BINDIR/bardo-mouse-gestures"
put_file 644 "$RENDERED/$UNIT" "$UNITDIR/$UNIT"
put_file 644 "$HERE/dist/70-bardo-mouse-gestures.rules" "$UDEVDIR/70-bardo-mouse-gestures.rules"
put_file 644 "$HERE/dist/99-bardo-mouse-gestures.conf" "$MODPROBEDIR/99-bardo-mouse-gestures.conf"
put_file 644 "$HERE/dist/bardo-mouse-gestures-modules.conf" "$MODULESDIR/bardo-mouse-gestures.conf"

if [[ "$LAYOUT" == package ]]; then
  say "Installed under $DESTDIR$PREFIX. On the machine: systemctl enable --now $UNIT"
  exit 0
fi

# root_step DESCRIPTION CMD...: one step on the machine as root, through the escalation helper, or
# printed when there is none.
root_step() {
  local what="$1"; shift
  if [[ "$(id -u)" == 0 ]]; then
    run "$what" "$@"
  elif (( ${#ESCALATE[@]} > 0 )); then
    run "$what" "${ESCALATE[@]}" "$@"
  else
    say "  $what"
    say "      (no sudo, doas or pkexec here: run this as root)"
    say "      \$ $*"
  fi
}

# An earlier install ran bardo-mouse-gestures as a service of the person; it would fight the system service for
# the mouse.
if [[ -f "${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user/$UNIT" ]]; then
  say "  note: ${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user/$UNIT is an earlier install; stop and remove it:"
  say "      \$ systemctl --user disable --now $UNIT && rm ${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user/$UNIT"
fi

# Temporary: an install from before the rename keeps its config in /etc/palmrest; carry it over once.
# Delete in the release after everyone has installed once.
if [[ -d /etc/palmrest && ! -e /etc/bardo/mouse-gestures ]]; then
  root_step "move /etc/palmrest to /etc/bardo/mouse-gestures" \
    sh -c 'mkdir -p /etc/bardo && mv /etc/palmrest /etc/bardo/mouse-gestures'
fi

root_step "load uinput now (it loads at every start of the computer anyway)" modprobe uinput
root_step "let the mouse's own scrolling go (bardo-mouse-gestures scrolls)" \
  sh -c 'echo 0 > /sys/module/hid_magicmouse/parameters/emulate_scroll_wheel'
root_step "apply the device rule" udevadm control --reload-rules
root_step "give the mouse's node to the service" udevadm trigger --subsystem-match=hidraw
root_step "tell systemd about the new service" systemctl daemon-reload
root_step "turn bardo-mouse-gestures on" systemctl enable --now "$UNIT"
