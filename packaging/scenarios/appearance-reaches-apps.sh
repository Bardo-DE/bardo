#!/usr/bin/env bash
# Scenario `appearance-reaches-apps` (plan §4.14): an appearance change through ui-settings reaches
# every running app, live, on a headless bardo-compositor in the jail.
#
# The apps are the bardo-shell daemon's surfaces (bar and Dock): each is a quire ds root that follows
# `bardo/appearance.toml` through ui-settings' watch. The change is made the way a settings app
# makes it, by writing the file in the scratch XDG_CONFIG_HOME (never a real one); the proof is each
# surface's own capture (`bardo-shell debug capture`), compared by pixels. Quire apps outside bardo-shell need
# a screenshot of bardo-compositor (P1-C1): that is the scenario `appearance-reaches-bardo-apps`.
#
#   baseline  theme = "light": every surface captured
#   dark      theme = "dark" is written: every surface changes (>= 0.4% of its pixels) with no restart
#   light     the file returns to light: every surface returns to its baseline (< 0.2% differ)
#   alive     the same daemon pid answers `ping` throughout
#
# Each wait is on the capture, polled up to 20 s (wait_look, expect_all: bardo-shell's dev/lib/pixels.sh), never a fixed sleep.
# budget: 120
# Exit: 0 pass | 1 a step failed | 2 outside the jail | 3 artefacts missing (harness/scenario-build.sh).
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/../harness/lib/all.sh"
source "$(dirname "${BASH_SOURCE[0]}")/../harness/lib/scenario.sh"
require_jail
scenario_need "$SCEN_TOOLS/bin/bardo-compositor" "$SCEN_TOOLS/bin/bardo-compositorctl" "$SCEN_TOOLS/bin/fake-logind" \
  "$SCEN_BUILD/sill-target/debug/bardo-shell"

export BARDO_SHELL_COMPOSITOR=bardo-compositor BARDO_SHELL_PREBUILT=1 BARDO_SHELL_PROFILE=debug
export CARGO_TARGET_DIR="$SCEN_BUILD/sill-target" BARDO_COMPOSITOR_TOOLS="$SCEN_TOOLS"
floor_shim appearance-reaches-apps bardo-shell   # bardo-shell takes no power a floor row can allow (no rows, nothing waits on a question); the caller list names it as the shell
# shellcheck source=/dev/null
source "$SCEN_SIB/shell/dev/lib/all.sh"
OUT_DIR=$(scenario_out appearance-reaches-apps)
CHANGED=0.004 SETTLED=0.002
SURFACES=() DAEMON_PID=

cleanup() {
  stop_daemon
  stop_compositor
  scratch_drop
}
trap cleanup EXIT

# The appearance file, as the settings app writes it.
write_theme() {
  mkdir -p "$CONFIG/bardo"
  printf '[appearance]\ntheme = "%s"\n' "$1" >"$CONFIG/bardo/appearance.toml"
}

step_baseline() {
  local name surface
  start_compositor || return 1
  scratch_new
  build || return 1
  write_theme light
  # The OSD is not among the surfaces: it hides after a fixed 1.5 s (the Settings prune made its hold a constant), so it
  # cannot stay up across the three captures. The bar and the Dock are drawn all the time.
  floor_sill_callers
  launch_daemon || return 1
  DAEMON_PID=$DAEMON
  name=$(output_name) || return 1
  SURFACES=("bardo-shell-bar-$name" "bardo-shell-dock-$name")
  # Settled: the bar and the Dock finish their first animations before the baseline is taken.
  for surface in "${SURFACES[@]}"; do await_frames_settled "$surface" 20 || { echo "$surface never settled" >&2; return 1; }; done
  for surface in "${SURFACES[@]}"; do capture "$surface" "$OUT_DIR/base-$surface.png" || return 1; done
}

step_dark() { write_theme dark; expect_all base different dark; }
step_light() { write_theme light; expect_all base same light; }

run_steps step_baseline step_dark step_light
if [ "$STATUS" != 0 ]; then
  echo "--- daemon.log (tail)" >&2; tail -20 "$OUT_DIR/daemon.log" >&2
  echo "--- captures in the scratch dir: $OUT_DIR" >&2
fi
exit "$STATUS"
