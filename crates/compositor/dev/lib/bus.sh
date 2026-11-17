# Shared state for the jailed dev scripts, and bus_up: a private dbus-daemon on
# dev/private-bus.conf as both session and system bus, under $JAIL_SCRATCH, exporting
# DBUS_SESSION_BUS_ADDRESS and DBUS_SYSTEM_BUS_ADDRESS.
#
# Owner: P0-T-dev. Sourceable; every other dev/lib file sources this one first.
#
# State (A8): everything a script writes goes under $JAIL_SCRATCH/compositor/<run>/ (a jail's
# scratch is its own, so jails run in parallel and the checkout is never written). The run dir
# holds env (a sourceable KEY=VALUE file), the daemons' logs and shots/. `latest` beside the run
# dirs names the newest. $BARDO_COMPOSITOR_STATE overrides the run dir.
#
# Tools (A7): built outside the jail by dev/bootstrap.sh into dev/out/tools/bin
# ($BARDO_COMPOSITOR_TOOLS overrides the tools dir); nothing here calls cargo or the network.

[ -z "${_COMPOSITOR_BUS_SH:-}" ] || return 0
_COMPOSITOR_BUS_SH=1

# shellcheck source=dev/lib/jail.sh
source "$(dirname "${BASH_SOURCE[0]}")/jail.sh"

# compositor_root: the checkout.
compositor_root() { (cd "$(dirname "${BASH_SOURCE[0]}")/../../../compositor" && pwd); }

# compositor_tool NAME: the path of a bootstrapped tool; exit 3 (environment incomplete) says how
# to build it.
compositor_tool() {
  local tools="${BARDO_COMPOSITOR_TOOLS:-$(compositor_root)/dev/out/tools}" path
  for path in "$tools/bin/$1" "$tools/$1"; do
    if [ -x "$path" ]; then echo "$path"; return 0; fi
  done
  echo "compositor: tool '$1' is missing from $tools/bin: run dev/bootstrap.sh outside the jail" >&2
  return 3
}

# compositor_state_dir: this run's state directory (created). $BARDO_COMPOSITOR_STATE wins; else the
# newest under the jail's scratch; else a new one.
compositor_state_dir() {
  local base dir
  if [ -n "${BARDO_COMPOSITOR_STATE:-}" ]; then mkdir -p "$BARDO_COMPOSITOR_STATE"; echo "$BARDO_COMPOSITOR_STATE"; return 0; fi
  : "${JAIL_SCRATCH:?compositor: JAIL_SCRATCH is not set: run inside ~/desktop/harness/jail.sh}"
  base="$JAIL_SCRATCH/compositor"
  mkdir -p "$base"
  if [ -L "$base/latest" ]; then dir=$(readlink "$base/latest"); else
    dir="$base/run-$(date +%Y%m%dT%H%M%S)-$RANDOM"
    mkdir -p "$dir" && ln -sfn "$dir" "$base/latest"
  fi
  mkdir -p "$dir"
  echo "$dir"
}

# compositor_new_state: starts a fresh run dir (the next compositor_state_dir returns it).
compositor_new_state() {
  : "${JAIL_SCRATCH:?compositor: JAIL_SCRATCH is not set: run inside ~/desktop/harness/jail.sh}"
  local base="$JAIL_SCRATCH/compositor" dir
  mkdir -p "$base"
  dir="$base/run-$(date +%Y%m%dT%H%M%S)-$RANDOM"
  mkdir -p "$dir" && ln -sfn "$dir" "$base/latest"
  BARDO_COMPOSITOR_STATE=$dir
  export BARDO_COMPOSITOR_STATE
}

# env_set KEY VALUE: records KEY in the run's env file (replacing an earlier value) and exports it.
env_set() {
  local file tmp
  file="$(compositor_state_dir)/env"
  tmp="$file.tmp.$$"
  { grep -v "^$1=" "$file" 2>/dev/null || true; printf '%s=%q\n' "$1" "$2"; } >"$tmp"
  mv "$tmp" "$file"
  export "$1=$2"
}

# compositor_env_load [ENV_FILE]: exports the run's env (the newest run's, by default).
compositor_env_load() {
  local file="${1:-$(compositor_state_dir)/env}"
  [ -f "$file" ] || { echo "compositor: no session env at $file: start one with dev/up.sh" >&2; return 1; }
  set -a
  # shellcheck source=/dev/null
  source "$file"
  set +a
}

# bus_up: starts the private bus (once per run). Needs the jail.
bus_up() {
  require_jail
  # A bus the caller started and recorded in this run's env file (a scenario with its own service
  # directory): its addresses come from that file, so the caller's shell need not export them.
  if [ -n "${BARDO_COMPOSITOR_BUS_PID:-}" ] && kill -0 "$BARDO_COMPOSITOR_BUS_PID" 2>/dev/null; then
    compositor_env_load
    return
  fi
  command -v dbus-daemon >/dev/null || { echo "bus_up: dbus-daemon is not installed" >&2; return 3; }
  local state out address pid
  state=$(compositor_state_dir)
  out=$(dbus-daemon --config-file "$(compositor_root)/dev/private-bus.conf" --fork --print-address=1 --print-pid=1 \
    2>"$state/dbus.log") || { echo "bus_up: dbus-daemon failed: $(cat "$state/dbus.log")" >&2; return 1; }
  address=$(printf '%s\n' "$out" | sed -n 1p)
  pid=$(printf '%s\n' "$out" | sed -n 2p)
  [ -n "$address" ] && [ -n "$pid" ] || { echo "bus_up: dbus-daemon printed no address" >&2; return 1; }
  env_set BARDO_COMPOSITOR_BUS_PID "$pid"
  env_set DBUS_SESSION_BUS_ADDRESS "$address"
  env_set DBUS_SYSTEM_BUS_ADDRESS "$address"
}

# bus_down: stops the bus by pid.
bus_down() {
  [ -z "${BARDO_COMPOSITOR_BUS_PID:-}" ] || kill "$BARDO_COMPOSITOR_BUS_PID" 2>/dev/null || true
}
