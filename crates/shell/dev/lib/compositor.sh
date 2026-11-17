# Where the sibling layer-shell-host checkout and this repo are, and the two compositors a script runs
# on: the compositor (the sibling the compositor checkout's dev/up.sh, headless, INSIDE the jail: only
# dev/accept.sh --compositor bardo, or a script run under packaging/harness/jail.sh, can use
# it) and layer-shell-host's headless KWin (kwin-virtual.sh). The KWin harness belongs to layer-shell-host;
# bardo-shell only wraps it.
#
# BARDO_SHELL_COMPOSITOR=bardo|kwin picks the compositor for `start_compositor`; unset, each script
# keeps its own choice: the compositor by default, kwin where it runs headless KWin (its
# `start_compositor --default kwin ...`).
# One set of helpers answers for both: start_compositor / stop_compositor, set_scale,
# require_scale, output_scale, output_size, shot, maximised_height and nested (a command in the
# compositor's environment); input.sh has start_input / input / key / click.
BARDO_SHELL_REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../shell" && pwd)"
BARDO_SHELL_TREE="$(cd "$BARDO_SHELL_REPO/../.." && pwd)"   # the bardo tree
BARDO_LAYER_SHELL_HOST_DEV="${BARDO_LAYER_SHELL_HOST_DEV:-$BARDO_SHELL_REPO/../layer-shell/scripts/dev}"
if [ ! -x "$BARDO_LAYER_SHELL_HOST_DEV/kwin-virtual.sh" ]; then
  echo "dev: layer-shell-host's harness not found at $BARDO_LAYER_SHELL_HOST_DEV (set BARDO_LAYER_SHELL_HOST_DEV)" >&2
  exit 1
fi
# The build every script runs: BARDO_SHELL_PROFILE=debug (the default) or release, from cargo's target
# directory (CARGO_TARGET_DIR when set, else the tree's target/). Both carry
# `--features debug` (the fake DDC, `bardo-shell debug ...`), which `build` passes. The latency budgets
# (a warm launcher open, a banner's first frame, ...) are release budgets: a debug build is
# several times slower, so `budgets_apply` is false there (and on a host whose load is past its
# core count) and a script prints the measurement but does not fail it. Run the suite with
# BARDO_SHELL_PROFILE=release on a quiet host for the full verdict.
BARDO_SHELL_PROFILE="${BARDO_SHELL_PROFILE:-debug}"
# shellcheck disable=SC2034 # BARDO_SHELL_CARGO_PROFILE is read by daemon.sh, run.sh and lock.sh
case "$BARDO_SHELL_PROFILE" in
  debug) BARDO_SHELL_CARGO_PROFILE=() ;;
  release) BARDO_SHELL_CARGO_PROFILE=(--release) ;;
  *) echo "dev: BARDO_SHELL_PROFILE must be debug or release, not '$BARDO_SHELL_PROFILE'" >&2; exit 1 ;;
esac
BARDO_SHELL_TARGET="${CARGO_TARGET_DIR:-$BARDO_SHELL_TREE/target}"
BARDO_SHELL_OUT="$BARDO_SHELL_TARGET/$BARDO_SHELL_PROFILE"
BARDO_SHELL_BIN="${BARDO_SHELL_BIN:-$BARDO_SHELL_OUT/bardo-shell}"
# A budget also needs a host that is not oversubscribed: the figures include `bardo-shell debug capture`
# and key round trips, which stretch several-fold when the one-minute load average is past the
# core count (another build or an agent suite running), and then say nothing about the shell.
budgets_apply() {
  [ "$BARDO_SHELL_PROFILE" = release ] || return 1
  awk -v cores="$(nproc)" '{ exit !($1 < cores) }' /proc/loadavg
}
budgets_why() {
  [ "$BARDO_SHELL_PROFILE" = release ] || { echo "a $BARDO_SHELL_PROFILE build; run with BARDO_SHELL_PROFILE=release"; return; }
  echo "host load $(cut -d' ' -f1 /proc/loadavg) is past its $(nproc) cores"
}
HOST_REPO="$(cd "$BARDO_LAYER_SHELL_HOST_DEV/../.." && pwd)"
KWIN_ENV="$BARDO_LAYER_SHELL_HOST_DEV/.kwin-env"
SIZE="${SIZE:-1920x1080}"
WL=
BUS=
# The compositor: the compositor crate of this tree and the
# tools dev/bootstrap.sh built there OUTSIDE the jail (the compositor, bardo-compositorctl, compositor-probe,
# fake-logind in dev/out/tools/bin; nothing inside the jail calls cargo). BARDO_COMPOSITOR_ENV is the env
# file of the session start_compositor made.
BARDO_COMPOSITOR_REPO="${BARDO_COMPOSITOR_REPO:-$BARDO_SHELL_TREE/crates/compositor}"
BARDO_COMPOSITOR_TOOLS="${BARDO_COMPOSITOR_TOOLS:-$BARDO_COMPOSITOR_REPO/dev/out/tools}"
BARDO_COMPOSITOR_ENV=
# The compositor's own bus (Debug1 lives on it): BUS is the script's, which a script that starts a bus per
# group (launcher.sh) replaces, so bardo-compositorctl keeps its own address.
BARDO_COMPOSITOR_BUS=
# The compositor start_compositor started (the compositor or kwin).
COMPOSITOR=
# Where a script's logs and captures go: the target directory and dev/out in a checkout, but the
# jail's own scratch dir inside one (A8: state never lands in the checkout, so jails run in parallel).
if [ -n "${JAIL_SCRATCH:-}" ]; then
  BARDO_SHELL_OUT_ROOT="$JAIL_SCRATCH/bardo/shell"
  BARDO_SHELL_SHOTS="$BARDO_SHELL_OUT_ROOT/shots"
else
  BARDO_SHELL_OUT_ROOT="$BARDO_SHELL_TARGET"
  BARDO_SHELL_SHOTS="$BARDO_SHELL_REPO/dev/out"
fi

# The headless KWin, at $SIZE (or $1), on its private bus; sets WL (its socket, absolute) and BUS
# (its private bus) from its env file, read without sourcing it into this shell: `kwin-virtual.sh
# stop` kills every process whose environment names the bus, which must not include the script.
# Never borrows a KWin someone else started, refuses a socket that is not the headless KWin's,
# and refuses a bus that would be the real one. Needs OUT_DIR. KWIN_VIRTUAL_* variables exported
# beforehand reach kwin-virtual.sh.
start_kwin() {
  [ ! -e "$KWIN_ENV" ] || { echo "$KWIN_ENV exists: another headless KWin is running" >&2; return 1; }
  "$BARDO_LAYER_SHELL_HOST_DEV/kwin-virtual.sh" start "${1:-$SIZE}" >"$OUT_DIR/kwin.log" 2>&1 || return 1
  local runtime socket
  runtime=$(sed -n 's/^export XDG_RUNTIME_DIR=//p' "$KWIN_ENV")
  socket=$(sed -n 's/^export WAYLAND_DISPLAY=//p' "$KWIN_ENV")
  BUS=$(sed -n "s/^export DBUS_SESSION_BUS_ADDRESS='\\(.*\\)'\$/\\1/p" "$KWIN_ENV")
  KWIN_SCOPE=$(sed -n 's/^\(export \)\{0,1\}KWIN_SCOPE=//p' "$KWIN_ENV")
  case "$socket" in /*) WL="$socket" ;; *) WL="$runtime/$socket" ;; esac
  case "$WL" in
    *wayland-kwinvirt-*) ;;
    *) echo "refusing $WL: not the headless KWin's socket" >&2; stop_kwin; return 1 ;;
  esac
  [ -S "$WL" ] && [ -n "$BUS" ] || { echo "no socket or bus in $KWIN_ENV" >&2; stop_kwin; return 1; }
  [ "$BUS" != "${DBUS_SESSION_BUS_ADDRESS:-}" ] || { echo "KWin's bus is the session's" >&2; stop_kwin; return 1; }
  require_private_system_bus "$BUS" || { stop_kwin; return 1; }
}

# Ends exactly the KWin start_kwin started; a no-op when none is running.
stop_kwin() {
  [ -f "$KWIN_ENV" ] || return 0
  "$BARDO_LAYER_SHELL_HOST_DEV/kwin-virtual.sh" stop >>"$OUT_DIR/kwin.log" 2>&1
}

# Sets scale $1 on the headless KWin's one output (the BenQ 4K at 2x of the user's session).
set_kwin_scale() {
  local output
  output=$(in_env kscreen-doctor -j 2>/dev/null | python3 -c '
import json, sys
print(json.load(sys.stdin)["outputs"][0]["name"])') || return 1
  in_env kscreen-doctor "output.$output.scale.$1" >>"$OUT_DIR/kwin.log" 2>&1
}

# The headless KWin's scale of its one output, from kscreen-doctor's JSON.
kwin_scale() {
  in_env kscreen-doctor -j 2>/dev/null | python3 -c '
import json, sys
print(json.load(sys.stdin)["outputs"][0].get("scale"))'
}

# Sets scale $1 (set_kwin_scale) and waits up to 3 s for kscreen-doctor to read it back; fails
# when the output does not end up at that scale.
require_kwin_scale() {
  set_kwin_scale "$1" || return 1
  for _ in $(seq 30); do
    python3 -c 'import sys; sys.exit(0 if abs(float(sys.argv[1]) - float(sys.argv[2])) < 0.01 else 1)' \
      "$(kwin_scale)" "$1" 2>/dev/null && return 0
    sleep 0.1
  done
  echo "the output's scale is $(kwin_scale), not $1" >&2
  return 1
}

# The nested output's logical size, "W H".
nested_output_size() {
  nested wayland-info | sed -n 's/.*logical_width: \([0-9]*\), logical_height: \([0-9]*\).*/\1 \2/p' | head -1
}

# True when a `bardo-shell run` of the person's draws on the compositor socket $WL (its WAYLAND_DISPLAY,
# read from /proc): only a daemon on the same output is in this script's screenshots. Daemons on
# other compositors (another agent's nested session, the real one) are none of its business.
daemon_on_wl() {
  local pid
  for pid in $(pgrep -u "$(id -u)" -f 'bardo-shell run'); do
    tr '\0' '\n' <"/proc/$pid/environ" 2>/dev/null | grep -qx "WAYLAND_DISPLAY=$WL" && return 0
  done
  return 1
}

# Waits up to 10 minutes for no other bardo-shell daemon on this script's compositor ($WL): it would be
# in the screenshots. Fails when one is still there.
await_no_process() {
  local waited=0
  while daemon_on_wl; do
    [ "$waited" = 0 ] && echo "another bardo-shell daemon is running; waiting for it to finish" >&2
    waited=$((waited + 1))
    [ "$waited" -gt 600 ] && { echo "it is still running after 10 min" >&2; return 1; }
    sleep 1
  done
}

# ---------------------------------------------------------------- the compositor, whichever it is

# The compositor `start_compositor` uses: $BARDO_SHELL_COMPOSITOR, else the script's own default $1.
compositor_kind() {
  local kind="${BARDO_SHELL_COMPOSITOR:-$1}"
  case "$kind" in
    bardo|kwin) echo "$kind" ;;
    *) echo "dev: BARDO_SHELL_COMPOSITOR must be bardo or kwin, not '$kind'" >&2; return 1 ;;
  esac
}

# start_compositor [--default bardo|kwin] [SIZE]: brings the compositor up and sets WL (its
# socket, absolute) and BUS (the private bus standing in for the session and the system bus).
#   kwin      layer-shell-host's headless KWin at SIZE (default $SIZE) on its private bus;
#   The compositor  dev/up.sh of the compositor checkout in this jail: a headless compositor on the
#             wall clock at SIZE, its private bus (also fake-logind's), WL and BUS from its env file.
# Needs OUT_DIR. Fails (exit code 2 for "not here") when the compositor is asked for outside the jail.
start_compositor() {
  local default=bardo
  if [ "${1:-}" = --default ]; then default="$2"; shift 2; fi
  COMPOSITOR=$(compositor_kind "$default") || return 1
  case "$COMPOSITOR" in
    compositor) start_compositor "$@" ;;
    kwin) start_kwin "$@" ;;
  esac
}

# Ends exactly what start_compositor started; a no-op when none is running.
stop_compositor() {
  case "$COMPOSITOR" in
    compositor) stop_compositor ;;
    kwin) stop_kwin ;;
  esac
  COMPOSITOR=
}

# The compositor tool NAME in the bootstrapped tools directory.
compositor_bin() {
  local path="$BARDO_COMPOSITOR_TOOLS/bin/$1"
  [ -x "$path" ] || { echo "compositor: $path is missing (run $BARDO_COMPOSITOR_REPO/dev/bootstrap.sh outside the jail)" >&2; return 3; }
  echo "$path"
}

# A compositor tool in the session's environment (its bus and its socket; usable before scratch_new).
compositor_run() {
  local tool="$1" bin
  shift
  bin=$(compositor_bin "$tool") || return 3
  env WAYLAND_DISPLAY="$WL" XDG_RUNTIME_DIR="$(dirname "$WL")" DBUS_SESSION_BUS_ADDRESS="${BARDO_COMPOSITOR_BUS:-$BUS}" "$bin" "$@"
}
bardo-compositorctl() { compositor_run bardo-compositorctl "$@"; }

# The programs these scripts run on the compositor's socket and the one power each takes, as
# "POWER PROGRAM". The compositor asks before a program captures the screen, reads the clipboard, types
# and so on, so the floor the session starts with allows exactly these (read off
# accept/clipboard.sh, accept/screenshot.sh, accept/launcher.sh and demo.sh; dev/shot.sh shoots
# KWin or the host, which compositor does not judge). A floor row names a program by its image.
# The compositor's test builds also take a row naming a program built under this checkout
# (privacy/injected.rs): BARDO_COMPOSITOR_FLOOR_BUILT below. The daemon itself is named as the shell by
# desktop-callers.toml beside the floor instead.
BARDO_COMPOSITOR_FLOOR_ROWS=(
  "screen grim"        # the output shot in screenshot.sh and demo.sh, and demo.sh's recorder
  "clipboard wl-copy"  # clipboard.sh and launcher.sh put made-up text on the selection
  "clipboard wl-paste" # and read it back
)

# Programs this checkout builds and the power each takes, as "POWER PATH": lock.sh's stand-in
# locker proves the session is free after an unlock.
BARDO_COMPOSITOR_FLOOR_BUILT=(
  "lock examples/session_locker"
)

# Where start_compositor puts the floor: inside the script's own output directory, so it goes with
# the run and nothing is written anywhere else.
compositor_floor_dir() { echo "$OUT_DIR/compositor-floor"; }

# Writes the session's floor (privacy.toml in an empty folder only this user can enter, the shape
# compositor-core's floor_codec reads) and prints the folder. A program that is not installed is left
# out: the script that needs it says so itself. Defaults stay "ask".
write_compositor_floor() {
  local dir row power tool path
  dir=$(compositor_floor_dir)
  [ -n "$OUT_DIR" ] || { echo "write_compositor_floor: OUT_DIR is not set" >&2; return 1; }
  rm -rf "$dir" && mkdir -m 700 "$dir" || return 1
  {
    echo "# The dev session's floor (bardo-shell dev/lib/compositor.sh): the programs the dev scripts run."
    for row in "${BARDO_COMPOSITOR_FLOOR_ROWS[@]}"; do
      power=${row%% *}
      tool=${row#* }
      path=$(command -v "$tool" 2>/dev/null) && path=$(readlink -f "$path") || continue
      printf '\n[[rule]]\npower = "%s"\nsubject = { program = "%s" }\nanswer = "allow"\n' "$power" "$path"
    done
    for row in "${BARDO_COMPOSITOR_FLOOR_BUILT[@]}"; do
      power=${row%% *}
      path=$(readlink -f "$BARDO_SHELL_OUT/${row#* }" 2>/dev/null) && [ -x "$path" ] || continue
      printf '\n[[rule]]\npower = "%s"\nsubject = { program = "%s" }\nanswer = "allow"\n' "$power" "$path"
    done
    # `shot` is the compositor's own bardo-compositorctl: once the daemon is the shell (desktop-callers.toml),
    # an unnamed capture is put to it as a question, which nobody here answers.
    path=$(compositor_bin bardo-compositorctl 2>/dev/null) && path=$(readlink -f "$path") \
      && printf '\n[[rule]]\npower = "screen"\nsubject = { program = "%s" }\nanswer = "allow"\n' "$path"
  } >"$dir/privacy.toml" || return 1
  chmod 600 "$dir/privacy.toml" || return 1
  write_compositor_callers "$dir" || return 1
  echo "$dir"
}

# The daemons this checkout built, as the shell: the jail has no /etc/bardo, so a test the compositor
# reads its list of the desktop's own programs from desktop-callers.toml in the injected folder
# (the compositor clients/dev_callers.rs). Without it the shell is a stranger, and the compositor refuses its
# session lock. Both daemons are named: lock.sh runs the one built without `debug` too.
write_compositor_callers() {
  local dir="$1" bin path
  {
    echo "# The dev session's shell (bardo-shell dev/lib/compositor.sh): this checkout's daemons."
    for bin in "$BARDO_SHELL_BIN" "$BARDO_SHELL_OUT/bardo-shell-nodebug"; do
      [ -x "$bin" ] || continue
      path=$(readlink -f "$bin") || continue
      printf '\n[[peer]]\napp = "org.bardo.Shell"\nrole = "shell"\nexe = ["%s"]\n' "$path"
    done
  } >"$dir/desktop-callers.toml" || return 1
  chmod 600 "$dir/desktop-callers.toml"
}

# Starts the compositor through its own dev/up.sh and sets WL, BUS, BARDO_COMPOSITOR_ENV. $1 is a size "WxH"
# (default $SIZE); KWIN_VIRTUAL_OUTPUTS=2 (widgets.sh) means two outputs side by side. The clock
# is the wall clock: bardo-shell's animations and timers run on real time, and a stepped clock would
# stand still until a script advanced it. Refuses a bus that would be the session's.
start_compositor() {
  local size="${1:-$SIZE}" spec up_out env_file floor
  mkdir -p "$OUT_DIR"
  spec="HEADLESS-1:$size@60000:120:0,0"
  if [ "${KWIN_VIRTUAL_OUTPUTS:-1}" = 2 ]; then
    spec="$spec;HEADLESS-2:$size@60000:120:${size%x*},0"
  fi
  [ -z "$BARDO_COMPOSITOR_ENV" ] || { echo "start_compositor: a compositor session is already up ($BARDO_COMPOSITOR_ENV)" >&2; return 1; }
  # --system-dir is read by the compositor's debug builds (the jail's) as a floor owned by this user.
  floor=$(write_compositor_floor) || { echo "start_compositor: could not write the session's floor" >&2; return 1; }
  up_out=$("$BARDO_COMPOSITOR_REPO/dev/up.sh" --clock wall --outputs "$spec" --system-dir "$floor" 2>"$OUT_DIR/compositor-up.log")
  local rc=$?
  if [ "$rc" != 0 ]; then
    cat "$OUT_DIR/compositor-up.log" >&2
    rm -rf "$floor"
    echo "start_compositor: dev/up.sh exited $rc (2: not in the jail; 3: run dev/bootstrap.sh)" >&2
    return "$rc"
  fi
  env_file=$(tail -n1 <<<"$up_out")
  [ -f "$env_file" ] || { echo "start_compositor: no env file '$env_file'" >&2; return 1; }
  BARDO_COMPOSITOR_ENV="$env_file"
  WL=$(source "$env_file" && echo "$WAYLAND_DISPLAY")
  BUS=$(source "$env_file" && echo "$DBUS_SESSION_BUS_ADDRESS")
  BARDO_COMPOSITOR_BUS=$BUS
  [ -S "$WL" ] && [ -n "$BUS" ] || { echo "no socket or bus in $env_file" >&2; stop_compositor; return 1; }
  [ "$BUS" != "${DBUS_SESSION_BUS_ADDRESS:-}" ] || { echo "compositor's bus is the session's" >&2; stop_compositor; return 1; }
  require_private_system_bus "$BUS" || { stop_compositor; return 1; }
  # The compositor's pointer starts on the output's corner (0, 0), where the hot corner and the bar's first
  # item would take it as a dwell and a hover. Park it on the left edge, halfway down: clear of the
  # corners, the bar, the dock, the right-hand notification column and the centred panels and
  # sheets (a launcher row under a resting pointer takes the hover and moves the selection).
  bardo-compositorctl debug inject "move $(( ${size%x*} / 8 )) $(( ${size#*x} / 2 ))" >>"$OUT_DIR/compositor-up.log" 2>&1 \
    || { echo "start_compositor: could not park the pointer (bardo-compositorctl debug inject)" >&2; stop_compositor; return 1; }
}

# Ends exactly the compositor start_compositor started (dev/down.sh stops each process by the pid in
# the env file, never by name); a no-op when none is running.
stop_compositor() {
  [ -n "$BARDO_COMPOSITOR_ENV" ] || return 0
  "$BARDO_COMPOSITOR_REPO/dev/down.sh" "$BARDO_COMPOSITOR_ENV" >>"$OUT_DIR/compositor-up.log" 2>&1
  rm -rf "$(compositor_floor_dir)"
  BARDO_COMPOSITOR_ENV=
  BARDO_COMPOSITOR_BUS=
}

# A line for a step that cannot be proven on this compositor yet: `expected_skip WHY` prints it
# (dev/accept.sh lists these) and returns 2, the "could not run" verdict of a step.
expected_skip() {
  echo "expected skip: $*" >&2
  return 2
}

# Runs "$@" in the compositor's environment: the script's own socket and bus.
nested() {
  env WAYLAND_DISPLAY="$WL" XDG_RUNTIME_DIR="$(dirname "$WL")" DBUS_SESSION_BUS_ADDRESS="$BUS" "$@"
}

# The output's logical size, "W H".
output_size() {
  case "$COMPOSITOR" in
    bardo)
      bardo-compositorctl debug output list --json | python3 -c '
import json, sys
o = json.load(sys.stdin)[0]
print(*o["logical"])' ;;
    *) nested_output_size ;;
  esac
}

# The first output's name and scale (a decimal), by compositor.
output_scale() {
  case "$COMPOSITOR" in
    bardo)
      bardo-compositorctl debug output list --json | python3 -c '
import json, sys
print(json.load(sys.stdin)[0]["scale"] / 120)' ;;
    *) kwin_scale ;;
  esac
}

# Sets scale $1 on the one output.
set_scale() {
  case "$COMPOSITOR" in
    bardo)
      local name
      name=$(bardo-compositorctl debug output list --json | python3 -c '
import json, sys
print(json.load(sys.stdin)[0]["name"])') || return 1
      bardo-compositorctl debug output set "$name" --scale "$1" >>"$OUT_DIR/compositor-up.log" 2>&1 ;;
    *) set_kwin_scale "$1" ;;
  esac
}

# Sets scale $1 (set_scale) and waits up to 3 s for the output to read it back; fails when it does not
# end up at that scale.
require_scale() {
  set_scale "$1" || return 1
  for _ in $(seq 30); do
    python3 -c 'import sys; sys.exit(0 if abs(float(sys.argv[1]) - float(sys.argv[2])) < 0.01 else 1)' \
      "$(output_scale)" "$1" 2>/dev/null && return 0
    sleep 0.1
  done
  echo "the output's scale is $(output_scale), not $1" >&2
  return 1
}

# Shoots the whole output into $1 (a PNG): `bardo-compositorctl shot --output NAME` for the compositor (the last
# presented frame of that output, the size answered as JSON; `--window` and `--masked` are P1-C1
# and not used here). Returns 2 (expected skip) when this compositor build has no capture at all;
# KWin's own scripts use spectacle themselves.
shot() {
  case "$COMPOSITOR" in
    bardo)
      mkdir -p "$(dirname "$1")"
      local rc
      # The headless compositor's first output (dev/up.sh) is HEADLESS-1.
      bardo-compositorctl shot --output "${BARDO_COMPOSITOR_OUTPUT:-HEADLESS-1}" "$1" >/dev/null
      rc=$?
      [ "$rc" != 2 ] || expected_skip "this compositor build has no screenshot (bardo-compositorctl shot exits 2)"
      return "$rc" ;;
    *) nested wayland-info >/dev/null 2>&1 || { echo "no compositor" >&2; return 1; }
       mkdir -p "$(dirname "$1")"
       nested grim "$1" >/dev/null ;;
  esac
}

# The height of a window that asks to be maximised on the output. KWin: layer-shell-host's
# `probe_toplevel` (built on first use) stands in for foot, asks to be maximised and prints each
# configure as JSON into $OUT_DIR/probe.log; prints the last maximised configure's height.
# The compositor: `compositor-probe toplevel --maximised`, which prints the same facts as JSON lines;
# returns 2 (an expected skip) while the probe has no such flag (it ignores unknown flags and
# prints no `maximised` field) or the compositor's P0 floating windows are never configured maximised.
PROBE="${BARDO_SHELL_PROBE:-$BARDO_SHELL_TARGET/release/examples/probe_toplevel}"
maximised_height() {
  [ "$COMPOSITOR" != bardo ] || { maximised_height_compositor; return; }
  [ -x "$PROBE" ] || (cd "$BARDO_SHELL_TREE" && CARGO_TARGET_DIR="$BARDO_SHELL_TARGET" cargo build --release -p layer-shell-host --features debug-inject \
    --example probe_toplevel >&2) || return 1
  local log="$OUT_DIR/probe.log" probe
  # The probe reads commands on stdin, which must be pollable: a pipe, not /dev/null.
  sleep 20 | nested "$PROBE" >"$log" 2>&1 &
  probe=$!
  for _ in $(seq 100); do
    grep -q '"maximized":true' "$log" && break
    sleep 0.1
  done
  kill "$probe" 2>/dev/null; wait "$probe" 2>/dev/null
  grep '"maximized":true' "$log" | tail -1 | python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["height"])'
}

maximised_height_compositor() {
  local log="$OUT_DIR/probe.log" height
  in_env timeout 25 "$(compositor_bin compositor-probe)" toplevel --maximised >"$log" 2>&1 </dev/null || true
  height=$(grep -E '"maxim(ised|ized)":true' "$log" | tail -1 | python3 -c '
import json, sys
print(json.loads(sys.stdin.read())["height"])' 2>/dev/null)
  if [ -z "$height" ]; then
    expected_skip "no maximised configure: compositor-probe toplevel has no --maximised (it ignores the flag) and P0 windows are floating; the window manager lands in P1/P2"
    return 2
  fi
  echo "$height"
}
