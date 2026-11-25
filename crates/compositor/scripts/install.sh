#!/usr/bin/env bash
# Build the compositor and install it: the compositor and its control tool, and the systemd user unit
# that starts the compositor in the desktop session (dist/bardo-compositor.service, with its ExecStart
# rewritten to the installed binary). The login entry and the session script are the desktop
# repo's (install/). Safe to run again: a file already in place and identical is left alone.
#
#   scripts/install.sh [--prefix P] [--destdir D] [--no-build] [--locked] [--dry-run]
#
#   --prefix P    where it is installed (default /usr/local): P/bin/bardo-compositor, P/bin/bardo-compositorctl,
#                 P/lib/systemd/user/bardo-compositor.service and P/share/bardo/settings/compositor.settings.toml
#   --destdir D   a staging root put in front of every path (for a package build); nothing then
#                 talks to systemd or SELinux
#   --no-build    use the release binaries that are already built (under $CARGO_TARGET_DIR, else
#                 ./target)
#   --locked      build with --locked (Cargo.lock as it is)
#   --dry-run     print every step, change nothing (no build)
#
# A place you cannot write is written through $SUDO (when set; empty means no helper), else sudo,
# doas or pkexec, whichever is installed; with none of them the commands are printed to run as root.
# The build runs at the root of the bardo tree (cargo build --release -p compositor-app -p compositor-ctl) into its target directory.
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
TREE=$(cd "$ROOT/../.." && pwd)
UNIT="$ROOT/dist/bardo-compositor.service"
BINS=(bardo-compositor bardo-compositorctl)

prefix=/usr/local destdir="" build=yes locked=no dry=no
usage() { sed -n '2,20p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; }
die() { echo "install.sh: $*" >&2; exit 2; }

while [ $# -gt 0 ]; do
  case "$1" in
    --prefix) prefix=${2:?--prefix needs a directory}; shift ;;
    --destdir) destdir=${2:?--destdir needs a directory}; shift ;;
    --no-build) build=no ;;
    --locked) locked=yes ;;
    --dry-run) dry=yes ;;
    -h|--help) usage; exit 0 ;;
    *) usage >&2; die "unknown argument $1" ;;
  esac
  shift
done
case "$prefix" in /*) ;; *) die "--prefix must be an absolute path" ;; esac
case "$prefix" in *[!A-Za-z0-9_./+-]*) die "--prefix may hold only letters, digits and _ . / + -" ;; esac
prefix=${prefix%/}
destdir=${destdir%/}

step() { printf '\n==> %s\n' "$*"; }
note() { printf '    %s\n' "$*"; }

# ---- privileges ---------------------------------------------------------------------------

# helper: the command that runs the rest as root: $SUDO (empty: none), else sudo, doas, pkexec.
helper() {
  if [ -n "${SUDO+x}" ]; then printf '%s' "$SUDO"; return; fi
  local tool
  for tool in sudo doas pkexec; do
    if command -v "$tool" >/dev/null 2>&1; then printf '%s' "$tool"; return; fi
  done
}

# writable PATH: PATH, or the nearest directory above it that exists, can be written by this user.
writable() {
  local path=$1
  while [ ! -e "$path" ]; do path=$(dirname "$path"); done
  [ -w "$path" ]
}

MANUAL=()

# run_at PATH CMD...: print and run CMD, through the helper when PATH is not writable; with no
# helper the command is kept to print at the end (the run then fails).
run_at() {
  local path=$1 helper_cmd=""
  shift
  if ! writable "$path"; then
    helper_cmd=$(helper)
    if [ -z "$helper_cmd" ]; then
      note "needs root: $*"
      MANUAL+=("$*")
      return 0
    fi
  fi
  note "\$ ${helper_cmd:+$helper_cmd }$*"
  # shellcheck disable=SC2086 # $helper_cmd is a command with optional words (SUDO="sudo -n")
  [ "$dry" = yes ] || ${helper_cmd:+$helper_cmd} "$@"
}

# put MODE SRC DEST: install SRC at DEST (under the destdir) unless DEST already is a copy of it.
put() {
  local mode=$1 src=$2 real="$destdir$3"
  if [ -f "$real" ] && cmp -s "$src" "$real"; then
    note "unchanged: $real"
    return 0
  fi
  run_at "$real" install -D -m "$mode" "$src" "$real"
}

# ---- the build ----------------------------------------------------------------------------

target="${CARGO_TARGET_DIR:-$TREE/target}/release"

step "1/5 build"
if [ "$build" = no ]; then
  note "skipped (--no-build): using $target"
elif [ "$dry" = yes ]; then
  note "would run: cargo build --release$([ "$locked" = no ] || echo ' --locked') -p compositor-app -p compositor-ctl"
else
  command -v cargo >/dev/null 2>&1 || die "cargo is not installed (install Rust from rustup.rs)"
  flags=(--release -p compositor-app -p compositor-ctl)
  [ "$locked" = no ] || flags+=(--locked)
  note "\$ cargo build ${flags[*]}"
  (cd "$TREE" && cargo build "${flags[@]}")
fi

step "2/5 the programs ($destdir$prefix/bin)"
for bin in "${BINS[@]}"; do
  [ "$dry" = yes ] || [ -x "$target/$bin" ] || die "no $target/$bin: build first (drop --no-build)"
  put 755 "$target/$bin" "$prefix/bin/$bin"
done

step "3/5 the user unit ($destdir$prefix/lib/systemd/user)"
[ -f "$UNIT" ] || die "$UNIT is missing"
rendered=$(mktemp)
# Kept when its install needs root: the printed command names it.
trap '[ ${#MANUAL[@]} -gt 0 ] || rm -f "$rendered"' EXIT
# Only the program path is replaced: the rest of the line (the session's arguments, $BARDO_COMPOSITOR_SESSION_ARGS) stays.
sed 's|^ExecStart=[^ ]*|ExecStart='"$prefix"'/bin/bardo-compositor|' "$UNIT" >"$rendered"
grep -q "^ExecStart=$prefix/bin/bardo-compositor\( \|\$\)" "$rendered" || die "$UNIT has no ExecStart= line to point at $prefix/bin/bardo-compositor"
put 644 "$rendered" "$prefix/lib/systemd/user/bardo-compositor.service"

step "4/5 the settings schema ($destdir$prefix/share/bardo/settings)"
# The Settings app finds the compositor's keys (Spaces, pointer, keyboard...) by this file.
if [ "$dry" = yes ]; then
  note "would run: $target/bardo-compositor --write-schema and install compositor.settings.toml"
else
  schema=$(mktemp -d)
  "$target/bardo-compositor" --write-schema "$schema" >/dev/null || die "bardo-compositor --write-schema failed"
  put 644 "$schema/compositor.settings.toml" "$prefix/share/bardo/settings/compositor.settings.toml"
  rm -rf "$schema"
fi

step "5/5 SELinux labels and the user manager"
if [ -n "$destdir" ]; then
  note "skipped: staging into $destdir"
else
  if command -v restorecon >/dev/null 2>&1; then
    run_at "$prefix/bin" restorecon -F "$prefix/bin/bardo-compositor" "$prefix/bin/bardo-compositorctl" "$prefix/lib/systemd/user/bardo-compositor.service"
  else
    note "no restorecon: skipped"
  fi
  if [ "$dry" = yes ]; then
    note "would run: systemctl --user daemon-reload"
  else
    systemctl --user daemon-reload 2>/dev/null || note "no user manager answered: run 'systemctl --user daemon-reload' in your session"
  fi
fi

if [ ${#MANUAL[@]} -gt 0 ] && [ "$dry" = no ]; then
  echo >&2
  echo "install.sh: these places need root and no sudo, doas or pkexec was found (or \$SUDO is empty)." >&2
  echo "Run them as root, or run this script again as root:" >&2
  printf '    %s\n' "${MANUAL[@]}" >&2
  exit 1
fi
step "done"
note "The desktop session starts bardo-compositor.service; install the login entry from packaging/session."
