# start_compositor [--outputs SPEC] [--clock wall|stepped] [--dmabuf none|soft] [--renderer R]
#                  [casement ARGS...]
#   A fresh run: the private bus (bus_up), fake-logind on it as the system bus, then
#   `casement --backend headless` in the current jail, waiting for its socket. Exports and records
#   in the run's env file: WAYLAND_DISPLAY (an absolute path in the jail's /run/user/$UID),
#   DBUS_SESSION_BUS_ADDRESS, DBUS_SYSTEM_BUS_ADDRESS, XDG_RUNTIME_DIR, BARDO_COMPOSITOR_PID,
#   BARDO_COMPOSITOR_LOGIND_PID, BARDO_COMPOSITOR_BUS_PID, BARDO_COMPOSITOR_STATE. Defaults: one 1920x1080 output at
#   scale 1, the stepped clock (virtual time: `bardo-compositorctl debug clock`), no dma-buf.
#   Test unit overrides: casement is started with this shell's environment, so a scenario with no
#   systemd sets BARDO_COMPOSITOR_DEBUG_PEER_UNITS="<program-or-pid>=<unit>[,...]" before start_compositor,
#   e.g. BARDO_COMPOSITOR_DEBUG_PEER_UNITS="bardo-shell=bardo-shell.service" start_compositor. Then a bus peer whose
#   program name (/proc/<pid>/comm, 15 bytes) is `bardo-shell` is taken to run in bardo-shell.service, and
#   org.bardo.Compositor1.Connect("shell") hands it the shell connection. An all-digit left side is
#   a pid. Only the jail's debug-inject build reads it; any other build ignores it. A bad value
#   makes casement exit 64 (start_compositor then prints the log). See FINDINGS "## debug-role".
# stop_compositor: stops what start_compositor started, each by the pid in the env file (never by
#   name), the bus last.
#
# Owner: P0-T-dev. Sourceable. Exit codes follow 20-FREEZE-PLAN 6.1: 2 outside the jail, 3 a
# tool is missing (run dev/bootstrap.sh), 1 the compositor did not come up (its log is printed).

# shellcheck source=dev/lib/bus.sh
source "$(dirname "${BASH_SOURCE[0]}")/bus.sh"

# How long start_compositor waits for the socket, in tenths of a second.
BARDO_COMPOSITOR_START_TENTHS="${BARDO_COMPOSITOR_START_TENTHS:-150}"

# _wayland_sockets DIR: the wayland-* sockets (not their .lock files) in DIR, one per line.
_wayland_sockets() {
  local path
  for path in "$1"/wayland-*; do
    case "$path" in *.lock) continue ;; esac
    if [ -S "$path" ]; then echo "$path"; fi
  done
  return 0
}

start_compositor() {
  require_jail
  local outputs="" clock=stepped dmabuf=none renderer="" extra=() state bin logind sockets_before
  while [ $# -gt 0 ]; do
    case "$1" in
      --outputs) outputs=${2:?--outputs needs a value}; shift 2 ;;
      --clock) clock=${2:?--clock needs a value}; shift 2 ;;
      --dmabuf) dmabuf=${2:?--dmabuf needs a value}; shift 2 ;;
      --renderer) renderer=${2:?--renderer needs a value}; shift 2 ;;
      # Anything else (e.g. --state-home DIR, to keep Space ids across a restart) goes to casement.
      *) extra+=("$1"); shift ;;
    esac
  done
  bin=$(casement_tool bardo-compositor) || return 3
  logind=$(casement_tool fake-logind) || return 3
  if [ -z "${BARDO_COMPOSITOR_STATE:-}" ]; then casement_new_state; _CASEMENT_STATE_OWNED=1; fi
  state=$(casement_state_dir)
  : "${XDG_RUNTIME_DIR:?start_compositor: XDG_RUNTIME_DIR is not set}"
  env_set XDG_RUNTIME_DIR "$XDG_RUNTIME_DIR"
  bus_up || return $?

  # fake-logind sits on the private bus as the system bus (Inhibit, Lock, TakeDevice).
  "$logind" >"$state/fake-logind.log" 2>&1 </dev/null &
  env_set BARDO_COMPOSITOR_LOGIND_PID "$!"

  sockets_before=$(_wayland_sockets "$XDG_RUNTIME_DIR")
  local args=(--backend headless --clock "$clock" --dmabuf "$dmabuf" --config-home "$state/config")
  [ -z "$outputs" ] || args+=(--outputs "$outputs")
  [ -z "$renderer" ] || args+=(--renderer "$renderer")
  "$bin" "${args[@]}" "${extra[@]}" >"$state/bardo-compositor.log" 2>&1 </dev/null &
  env_set BARDO_COMPOSITOR_PID "$!"
  env_set BARDO_COMPOSITOR_CLOCK "$clock"

  local tenths=0 socket=""
  while [ "$tenths" -lt "$BARDO_COMPOSITOR_START_TENTHS" ]; do
    if ! _alive "$BARDO_COMPOSITOR_PID"; then
      echo "start_compositor: casement exited before its socket appeared; $state/bardo-compositor.log:" >&2
      tail -20 "$state/bardo-compositor.log" >&2
      stop_compositor
      return 1
    fi
    socket=$(comm -13 <(printf '%s\n' "$sockets_before" | sort) <(_wayland_sockets "$XDG_RUNTIME_DIR" | sort) | head -1)
    [ -z "$socket" ] || break
    sleep 0.1
    tenths=$((tenths + 1))
  done
  if [ -z "$socket" ]; then
    echo "start_compositor: no wayland socket after $((BARDO_COMPOSITOR_START_TENTHS / 10)) s; $state/bardo-compositor.log:" >&2
    tail -20 "$state/bardo-compositor.log" >&2
    stop_compositor
    return 1
  fi
  env_set WAYLAND_DISPLAY "$socket"
  echo "casement is up: WAYLAND_DISPLAY=$socket (env: $state/env)" >&2
}

# _alive PID: the process exists and is not a zombie waiting to be reaped.
_alive() {
  [ -n "$1" ] && kill -0 "$1" 2>/dev/null && [ "$(awk '{print $3}' "/proc/$1/stat" 2>/dev/null)" != Z ]
}

# _stop_pid PID: a polite TERM, then KILL after two seconds. Only ever the pid it is given.
_stop_pid() {
  local pid=$1 tenths=0
  _alive "$pid" || return 0
  kill "$pid" 2>/dev/null || return 0
  while _alive "$pid"; do
    if [ "$tenths" -ge 20 ]; then kill -9 "$pid" 2>/dev/null || true; break; fi
    sleep 0.1
    tenths=$((tenths + 1))
  done
  wait "$pid" 2>/dev/null || true
}

stop_compositor() {
  require_jail
  local file state
  state=$(casement_state_dir)
  file="$state/env"
  if [ -f "$file" ]; then
    casement_env_load "$file"
    _stop_pid "${BARDO_COMPOSITOR_PID:-}"
    _stop_pid "${BARDO_COMPOSITOR_LOGIND_PID:-}"
    _stop_pid "${BARDO_COMPOSITOR_BUS_PID:-}"
    [ -z "${WAYLAND_DISPLAY:-}" ] || rm -f "$WAYLAND_DISPLAY" "$WAYLAND_DISPLAY.lock"
    : >"$state/stopped"
  fi
  unset BARDO_COMPOSITOR_PID BARDO_COMPOSITOR_LOGIND_PID BARDO_COMPOSITOR_BUS_PID
  # A run dir start_compositor made is finished with; one the caller named stays theirs.
  if [ -n "${_CASEMENT_STATE_OWNED:-}" ]; then unset BARDO_COMPOSITOR_STATE _CASEMENT_STATE_OWNED; fi
}
