#!/usr/bin/env bash
# bardo-shell's idle-cost acceptance, on the compositor ONLY (brought up and ended by start_compositor). A release+debug build is started on a private bus (both session and system
# bus, nothing else on it) with scratch HOME/XDG and the fake DDC, left alone for WARMUP seconds,
# then sampled over DURATION seconds from /proc/<pid>:
#
#   1. CPU: utime+stime over the window, as a percentage of one core, must stay under
#      MAX_CPU_PCT (default 1.0). A shell with nothing to do and nothing on the screen moving
#      must not spin: before the fix it burned about 48 %.
#   2. Threads: the highest thread count seen must stay under MAX_THREADS (default 120: the
#      tokio workers, one per core, and a few dozen others). The blocking pool is capped
#      (`max_blocking_threads`), so a loop of `spawn_blocking` calls cannot grow it without
#      bound. The nested daemon has no GPU adapter and renders on Mesa's llvmpipe, which starts
#      one thread pool per core per device: LP_NUM_THREADS=2 keeps those out of the count.
#   3. RSS: reported only (MAX_RSS_MB, when set, makes it a limit).
#   4. Frames: over FRAMES_WINDOW seconds (at most 10, inside one minute, so the bar's clock owes
#      no frame) more of the same idle, no surface draws a frame. Every
#      frame of a still shell is GPU work for nothing (on a small integrated GPU at 4K a wallpaper
#      frame alone is tens of megabytes); the step names each surface that drew, and how often.
#
# SAFETY: the daemon gets its own XDG_RUNTIME_DIR/HOME/XDG_*, the private bus as both buses, the
# fake DDC; it only touches processes it started itself (kill by its own pid).
#
# Usage: dev/accept/idle-cpu.sh
#   DURATION=60 WARMUP=20 MAX_CPU_PCT=1.0 MAX_THREADS=120 FRAMES_WINDOW=10
#   BARDO_SHELL_PROFILE=release|debug (default release)
set -uo pipefail
# Idle cost is a release measurement (a debug build's CPU says nothing): release unless asked.
export BARDO_SHELL_PROFILE="${BARDO_SHELL_PROFILE:-release}"
# shellcheck source=dev/lib/all.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/all.sh"
OUT_DIR="$BARDO_SHELL_OUT_ROOT/accept-idle-cpu"
mkdir -p "$OUT_DIR"
DURATION="${DURATION:-60}"
WARMUP="${WARMUP:-20}"
INTERVAL="${INTERVAL:-5}"
MAX_CPU_PCT="${MAX_CPU_PCT:-1.0}"
MAX_THREADS="${MAX_THREADS:-120}"
MAX_RSS_MB="${MAX_RSS_MB:-}"
FRAMES_WINDOW="${FRAMES_WINDOW:-10}"
CSV="$OUT_DIR/idle.csv"

# utime+stime in clock ticks of every thread of $1 (the process's own stat line).
ticks_of() {
  awk '{ rest = $0; sub(/^.*\) /, "", rest); split(rest, f, " "); print f[12] + f[13] }' "/proc/$1/stat"
}
threads_of() { awk '/^Threads:/ { print $2 }' "/proc/$1/status"; }
rss_mb_of() { awk '/^VmRSS:/ { printf "%d\n", $2 / 1024 }' "/proc/$1/status"; }

step_idle_cost() {
  clear_of_the_minute
  sleep "$WARMUP"
  kill -0 "$DAEMON" 2>/dev/null || { echo "the daemon died during warm-up (see $OUT_DIR/daemon.log)" >&2; return 1; }
  local hz t0 t1 ticks0 ticks max_threads=0 threads rss max_rss=0 elapsed=0 pct
  hz=$(getconf CLK_TCK)
  ticks0=$(ticks_of "$DAEMON")
  t0=$(date +%s.%N)
  echo "elapsed_s,ticks,threads,rss_mb" >"$CSV"
  while [ "$elapsed" -lt "$DURATION" ]; do
    sleep "$INTERVAL"
    elapsed=$((elapsed + INTERVAL))
    kill -0 "$DAEMON" 2>/dev/null || { echo "the daemon died while sampled" >&2; return 1; }
    ticks=$(ticks_of "$DAEMON"); threads=$(threads_of "$DAEMON"); rss=$(rss_mb_of "$DAEMON")
    echo "$elapsed,$ticks,$threads,$rss" >>"$CSV"
    [ "$threads" -le "$max_threads" ] || max_threads=$threads
    [ "$rss" -le "$max_rss" ] || max_rss=$rss
  done
  t1=$(date +%s.%N)
  pct=$(python3 -c 'import sys
hz, t0, t1, a, b = map(float, sys.argv[1:])
print(f"{100.0 * (b - a) / hz / (t1 - t0):.2f}")' "$hz" "$t0" "$t1" "$ticks0" "$ticks")
  echo "idle over ${DURATION}s: cpu ${pct}% of a core (max ${MAX_CPU_PCT}), threads max ${max_threads} (max ${MAX_THREADS}), rss ${max_rss} MB; samples in $CSV"
  local result=0
  python3 -c 'import sys; sys.exit(0 if float(sys.argv[1]) <= float(sys.argv[2]) else 1)' "$pct" "$MAX_CPU_PCT" \
    || { echo "FAIL cpu ${pct}% > ${MAX_CPU_PCT}%" >&2; result=1; }
  [ "$max_threads" -le "$MAX_THREADS" ] || { echo "FAIL threads ${max_threads} > ${MAX_THREADS}" >&2; result=1; }
  [ -z "$MAX_RSS_MB" ] || [ "$max_rss" -le "$MAX_RSS_MB" ] || { echo "FAIL rss ${max_rss} MB > ${MAX_RSS_MB}" >&2; result=1; }
  return "$result"
}

# Every surface's frame count, one "surface frames" line each.
all_frames() {
  bardo-shell debug stats | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    print(row["surface"], row["frames"])'
}

step_idle_frames() {
  local before after drew
  # The bar's clock turns over once a minute, which is a frame it owes: stay inside one minute.
  clear_of_the_minute
  before=$(all_frames) || { echo "could not read the surfaces' frame counts" >&2; return 1; }
  sleep "$FRAMES_WINDOW"
  after=$(all_frames) || { echo "could not read the surfaces' frame counts" >&2; return 1; }
  drew=$(python3 -c '
import sys
before = dict(line.split() for line in sys.argv[1].splitlines())
for line in sys.argv[2].splitlines():
    surface, frames = line.split()
    drawn = int(frames) - int(before.get(surface, 0))
    if drawn:
        print(f"{surface} {drawn}")' "$before" "$after")
  echo "idle frames over ${FRAMES_WINDOW}s on $(echo "$after" | wc -l) surfaces: ${drew:-none drew}" | tr '\n' ' '
  echo
  [ -z "$drew" ] || { echo "FAIL surfaces drew while idle (surface frames): $(echo "$drew" | tr '\n' ';')" >&2; return 1; }
}

cleanup() {
  local result=0
  stop_daemon || result=1
  stop_private_bus
  stop_compositor
  scratch_drop
  return "$result"
}

start_compositor || exit 1
nested wayland-info >/dev/null 2>&1 || { echo "no nested compositor (start_compositor failed)" >&2; exit 1; }
await_no_process || exit 1
build || exit 1
IN_ENV_EXTRA+=(LP_NUM_THREADS=2)
scratch_new
start_private_bus || { scratch_drop; exit 1; }
launch_daemon || { cleanup; exit 1; }
run_steps step_idle_cost step_idle_frames
status=$STATUS
cleanup || status=1
echo "logs in $OUT_DIR"
exit "$status"
