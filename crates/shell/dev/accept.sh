#!/usr/bin/env bash
# The acceptance runner: every dev/accept/NAME.sh on a compositor, one verdict per script.
#
#   dev/accept.sh [--compositor the compositor|kwin] [--jobs N] [--build] [--out DIR]
#                 [--timeout SECONDS] [NAME...]
#
# NAME is a script's name (`bar`, `bar.sh` or its path); with none, every dev/accept/*.sh.
#
#   --compositor the compositor   each script runs in its OWN jail (packaging/harness/jail.sh) on a headless
#                           The compositor that the script starts itself (dev/lib/compositor.sh's
#                           start_compositor, through the compositor's dev/up.sh): N jails at a
#                           time (--jobs, default 4), each with its own scratch dir OUT/NAME/ for its
#                           runtime dir, buses, logs and captures. The checkout is read-only in the jail,
#                           and nothing in it calls cargo or the network (A7): the daemon, the examples
#                           and the compositor's tools are built OUTSIDE first (--build does it; without it a
#                           missing artefact is an error, exit 3).
#   --compositor kwin       the scripts run directly, one at a time (they share the one headless
#                           KWin), as before.
#   (no --compositor)       each script keeps its own default (the compositor, or kwin where it ran
#                           headless KWin).
#
# Each script's combined output is OUT/NAME.log (default OUT: $BARDO_SHELL_TARGET/accept-runs/<run>). The runner
# prints one PASS|SKIP|FAIL line per script and, under it, the steps that skipped and why ("expected
# skip: ..." lines, and a launcher's PENDING lines) or failed. A script's own exit code is its verdict:
# 0 passed, 2 passed but for steps that could not run on this compositor yet (SKIP), 1 failed.
#
# Exit: 0 every script passed | 1 one failed | 2 none failed but one skipped | 3 environment incomplete
# (an artefact is missing: run with --build, outside the jail) | 4 a script overran its bound.
#
# Environment: BARDO_SHELL_PROFILE (debug, the default, or release), CARGO_TARGET_DIR, BARDO_COMPOSITOR_REPO (the
# The compositor crate, default crates/compositor of this tree), BARDO_COMPOSITOR_TARGET (its cargo target dir, for --build),
# BARDO_COMPOSITOR_TOOLS, BARDO_SHELL_JAIL (the jail script).
set -uo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
# shellcheck source=dev/lib/compositor.sh
source "$root/dev/lib/compositor.sh"

BARDO_ACTIONS_TARGET="${BARDO_ACTIONS_TARGET:-$BARDO_SHELL_TARGET/docket-target}"
compositor="" parallel=4 build=no out="" bound=${BARDO_ACCEPT_SCRIPT_TIMEOUT:-900} names=()
while [ $# -gt 0 ]; do
  case "$1" in
    --compositor) compositor=${2:?--compositor needs a value}; shift 2 ;;
    --jobs) parallel=${2:?--jobs needs a value}; shift 2 ;;
    --build) build=yes; shift ;;
    --out) out=${2:?--out needs a value}; shift 2 ;;
    --timeout) bound=${2:?--timeout needs a value}; shift 2 ;;
    -h|--help) sed -n '2,32p' "$0"; exit 0 ;;
    -*) echo "accept.sh: unknown flag $1" >&2; exit 64 ;;
    *) names+=("$1"); shift ;;
  esac
done
case "$compositor" in ""|compositor|kwin) ;; *) echo "accept.sh: --compositor is compositor or kwin" >&2; exit 64 ;; esac
case "$parallel" in ''|*[!0-9]*|0) echo "accept.sh: --jobs wants a positive number" >&2; exit 64 ;; esac

# The scripts to run, as names.
if [ ${#names[@]} -eq 0 ]; then
  for path in "$root"/dev/accept/*.sh; do names+=("$(basename "$path" .sh)"); done
else
  for i in "${!names[@]}"; do
    names[i]=$(basename "${names[i]}" .sh)
    [ -f "$root/dev/accept/${names[i]}.sh" ] || { echo "accept.sh: no script dev/accept/${names[i]}.sh" >&2; exit 64; }
  done
fi
[ "$parallel" -le ${#names[@]} ] || parallel=${#names[@]}
if [ "$compositor" != compositor ] && [ "$parallel" -gt 1 ]; then
  echo "accept.sh: scripts on ${compositor:-their own compositors} share one compositor: running them one at a time"
  parallel=1
fi

if [ -z "$out" ]; then out="$BARDO_SHELL_TARGET/accept-runs/$(date +%Y%m%dT%H%M%S)-$$"; fi
mkdir -p "$out"
out=$(cd "$out" && pwd)

# --- the artefacts, built outside the jail (A7)
prebuild() {
  local flags=("${BARDO_SHELL_CARGO_PROFILE[@]}")
  {
    echo "== the daemon without the debug feature (lock.sh's last step asks a build without it)"
    (cd "$root" && cargo build "${flags[@]}" -p shell-app) || return 1
    cp -f "$BARDO_SHELL_OUT/bardo-shell" "$BARDO_SHELL_OUT/bardo-shell-nodebug" || return 1
    echo "== the daemon with --features debug, the testkit's examples, the launcher demo"
    (cd "$root" && cargo build "${flags[@]}" -p shell-app --features debug \
      && cargo build "${flags[@]}" -p shell-testkit --examples \
      && cargo build "${flags[@]}" -p shell-services --bins \
      && cargo build "${flags[@]}" -p shell-launcher-ui --example launcher_demo \
      && cargo build --release -p shell-bar --example bar_demo) || return 1
    echo "== layer-shell-host's probe_toplevel (the maximised-window probe)"
    (cd "$BARDO_SHELL_TREE" && CARGO_TARGET_DIR="$BARDO_SHELL_TARGET" cargo build --release -p layer-shell-host \
      --features debug-inject --example probe_toplevel) || return 1
    echo "== docket (confirm.sh's accept-actiond and bardo-do), into its own target dir"
    (cd "$BARDO_SHELL_TREE" && CARGO_TARGET_DIR="$BARDO_ACTIONS_TARGET" cargo build -p action-accept --bin accept-actiond && CARGO_TARGET_DIR="$BARDO_ACTIONS_TARGET" cargo build -p action-cli --bins) || return 1
    echo "== compositor's tools"
    (cd "$BARDO_COMPOSITOR_REPO" && CARGO_TARGET_DIR="${BARDO_COMPOSITOR_TARGET:-$BARDO_SHELL_TARGET}" dev/bootstrap.sh --build-only) || return 1
  } >"$out/build.log" 2>&1 || { echo "accept.sh: the build failed (see $out/build.log)" >&2; return 1; }
}
if [ "$build" = yes ]; then
  [ "$compositor" = compositor ] || echo "accept.sh: --build prepares the jailed (compositor) run; building anyway"
  prebuild || exit 3
fi
if [ "$compositor" = compositor ]; then
  for tool in bardo-compositor bardo-compositorctl compositor-probe fake-logind; do
    [ -x "$BARDO_COMPOSITOR_TOOLS/bin/$tool" ] || { echo "accept.sh: $BARDO_COMPOSITOR_TOOLS/bin/$tool is missing: run dev/accept.sh --build (outside the jail)" >&2; exit 3; }
  done
  for artefact in "$BARDO_SHELL_OUT/bardo-shell" "$BARDO_SHELL_OUT/bardo-shell-nodebug"; do
    [ -x "$artefact" ] || { echo "accept.sh: $artefact is missing: run dev/accept.sh --build (outside the jail)" >&2; exit 3; }
  done
  jail="${BARDO_SHELL_JAIL:-$BARDO_SHELL_TREE/packaging/harness/jail.sh}"
  [ -x "$jail" ] || { echo "accept.sh: the jail script $jail is missing" >&2; exit 3; }
  jail_env=(
    --env BARDO_SHELL_COMPOSITOR=bardo-compositor --env BARDO_SHELL_PREBUILT=1 --env "BARDO_SHELL_PROFILE=$BARDO_SHELL_PROFILE"
    --env "CARGO_TARGET_DIR=$BARDO_SHELL_TARGET" --env "BARDO_ACTIONS_TARGET=$BARDO_ACTIONS_TARGET"
    --env "BARDO_SHELL_PROBE=$BARDO_SHELL_TARGET/release/examples/probe_toplevel" --env "BARDO_COMPOSITOR_REPO=$BARDO_COMPOSITOR_REPO" --env "BARDO_COMPOSITOR_TOOLS=$BARDO_COMPOSITOR_TOOLS"
  )
  [ -z "${BARDO_COMPOSITOR_START_TENTHS:-}" ] || jail_env+=(--env "BARDO_COMPOSITOR_START_TENTHS=$BARDO_COMPOSITOR_START_TENTHS")
  [ -z "${BARDO_LAYER_SHELL_HOST_DEV:-}" ] || jail_env+=(--env "BARDO_LAYER_SHELL_HOST_DEV=$BARDO_LAYER_SHELL_HOST_DEV")
fi

# run_one NAME: one script, in its own jail on the compositor or directly otherwise; leaves
# "<exit> <seconds>" in OUT/NAME.result.
run_one() {
  local name=$1 start end rc
  start=$(date +%s)
  if [ "$compositor" = compositor ]; then
    timeout --kill-after=10 "$bound" "$jail" --scratch "$out/$name" "${jail_env[@]}" -- \
      "$root/dev/accept/$name.sh" >"$out/$name.log" 2>&1 </dev/null
  else
    BARDO_SHELL_COMPOSITOR=$compositor timeout --kill-after=10 "$bound" "$root/dev/accept/$name.sh" \
      >"$out/$name.log" 2>&1 </dev/null
  fi
  rc=$?
  end=$(date +%s)
  [ "$rc" != 124 ] || rc=4
  echo "$rc $((end - start))" >"$out/$name.result"
}

passed=0 failed=0 skipped=0 worst=0 reported=()
# The lines that say why a script did not simply pass: skipped steps and their reasons, failed steps.
why() {
  grep -E '^(SKIP|FAIL) (step|check)|expected skip|PENDING - |^step [0-9A-Za-z_-]+: (SKIP|FAIL)' "$out/$1.log" \
    | sed 's/^/    /' | sort -u -k2 | head -${2:-30}
}
report_done() {
  local name rc secs seen
  for name in "${names[@]}"; do
    [ -f "$out/$name.result" ] || continue
    for seen in "${reported[@]:-}"; do [ "$seen" != "$name" ] || continue 2; done
    read -r rc secs <"$out/$name.result"
    reported+=("$name")
    case "$rc" in
      0) passed=$((passed + 1)); echo "PASS $name (${secs}s)" ;;
      2) skipped=$((skipped + 1)); echo "SKIP $name (${secs}s): passed but for what could not run here yet"
         why "$name"; [ "$worst" != 0 ] || worst=2 ;;
      *)
        failed=$((failed + 1))
        echo "FAIL $name (exit $rc, ${secs}s) log: $out/$name.log"
        why "$name" 12
        tail -8 "$out/$name.log" | sed 's/^/    | /'
        case "$rc" in
          3) [ "$worst" = 1 ] || worst=3 ;;
          1) worst=1 ;;
          *) [ "$worst" = 1 ] || [ "$worst" = 3 ] || worst=4 ;;
        esac ;;
    esac
  done
}

started=$(date +%s)
for name in "${names[@]}"; do
  while [ "$(jobs -rp | wc -l)" -ge "$parallel" ]; do
    wait -n || true
    report_done
  done
  run_one "$name" &
done
while [ "$(jobs -rp | wc -l)" -gt 0 ]; do
  wait -n || true
  report_done
done
wait || true
report_done
echo "accept: $passed passed, $skipped skipped, $failed failed in $(( $(date +%s) - started ))s (logs: $out)"
exit "$worst"
