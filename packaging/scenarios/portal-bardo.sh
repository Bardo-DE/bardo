#!/usr/bin/env bash
# Scenario `portal-bardo`: bardo-portal is the portal of the Bardo desktop. On a headless bardo-compositor in the jail,
# apps' portal calls go through xdg-desktop-portal to bardo-portal, not to xdg-desktop-portal-wlr.
#
# The portal chain (private bus shared with bardo-compositor, pipewire, wireplumber, bardo-portal, xdg-desktop-portal with
# XDG_CURRENT_DESKTOP=bardo) is bardo-portal's own (bardo-portal/dev/lib/chain.sh); its acceptance scripts make the
# calls a browser or OBS makes, against the frontend, and are what the steps run:
#   routing     session/bardo-portals.conf (what the desktop installs) names bardo-portal for every interface
#               bardo-portal's bardo.portal lists, keeps the file chooser with gtk, and says what bardo-portal's
#               dist/bardo-portals.conf says (the chain in the next steps reads that copy)
#   screenshot  Screenshot returns the address of a PNG of the output, at the output's size
#   screencast  a ScreenCast session on the output starts and a PipeWire consumer receives frames
#   settings    org.freedesktop.appearance is read through the frontend; SettingChanged follows the file
# A step's script exiting 2 is a SKIP (bardo-portal's own rule for "not runnable"); the screencast step also
# needs the shell's source chooser (org.bardo.Shell1) or its stand-in, which that script provides.
# The window cast, remote desktop and chooser scripts are not run here (they need a client window, EIS and a
# bardo-compositor peer-unit override); they stay bardo-portal's own.
#
# budget: 180
# Exit: 0 pass | 1 a step failed | 2 outside the jail | 3 artefacts missing (harness/scenario-build.sh).
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/../harness/lib/all.sh"
source "$(dirname "${BASH_SOURCE[0]}")/../harness/lib/scenario.sh"
require_jail
BARDO_PORTAL_DIR="$SCEN_SIB/portal"
BARDO_PORTAL_BIN="$SCEN_BUILD/transom-target/debug/bardo-portal"
scenario_need "$SCEN_TOOLS/bin/bardo-compositor" "$SCEN_TOOLS/bin/bardo-compositorctl" "$SCEN_TOOLS/bin/fake-logind" \
  "$SCEN_SIB/shell/dev/lib/all.sh" "$BARDO_PORTAL_BIN" "$BARDO_PORTAL_DIR/dist/bardo.portal" "$BARDO_PORTAL_DIR/dist/bardo-portals.conf"
for tool in pipewire wireplumber gdbus gst-launch-1.0 /usr/libexec/xdg-desktop-portal; do
  command -v "$tool" >/dev/null || { echo "scenario: $tool is missing (dnf install pipewire wireplumber gstreamer1 xdg-desktop-portal)" >&2; exit 3; }
done

export BARDO_SHELL_COMPOSITOR=bardo-compositor BARDO_SHELL_PREBUILT=1 BARDO_SHELL_PROFILE=debug
export CARGO_TARGET_DIR="$SCEN_BUILD/shell-target" BARDO_COMPOSITOR_TOOLS="$SCEN_TOOLS"
# bardo-portal (built under build/) is the portal (a row in the caller list) and takes the screen (a floor row); both name its built image.
floor_shim portal-bardo bardo-portal xdg-desktop-portal fake-shell
# shellcheck source=/dev/null
source "$SCEN_SIB/shell/dev/lib/all.sh"
OUT_DIR=$(scenario_out portal-bardo)

trap 'stop_compositor' EXIT

# Non-comment, non-blank lines of a portals.conf, sorted.
conf_lines() { grep -v -e '^[[:space:]]*#' -e '^[[:space:]]*$' "$1" | sort; }

step_routing() {
  local conf="$SCEN_ROOT/session/bardo-portals.conf" portal="$BARDO_PORTAL_DIR/dist/bardo.portal" iface bad=0
  for iface in $(sed -n 's/^Interfaces=//p' "$portal" | tr ';' ' '); do
    grep -qx "$iface=bardo" "$conf" || { echo "$conf does not route $iface to bardo" >&2; bad=1; }
  done
  grep -qx 'org.freedesktop.impl.portal.FileChooser=gtk' "$conf" || { echo "the file chooser is not left to gtk" >&2; bad=1; }
  grep -qx 'default=bardo;gtk' "$conf" || { echo "the default is not bardo;gtk" >&2; bad=1; }
  ! grep -q 'wlr' "$conf" || { echo "$conf still names wlr" >&2; bad=1; }
  diff <(conf_lines "$conf") <(conf_lines "$BARDO_PORTAL_DIR/dist/bardo-portals.conf") >&2 \
    || { echo "the desktop's portals.conf and bardo-portal's differ" >&2; bad=1; }
  return "$bad"
}

# bardo-compositor's env file is plain NAME=value and bardo-portal's chain sources it without exporting, so the
# chain gets a copy that exports every name (the bus and the display reach bardo-portal and the portal).
step_boot() {
  start_compositor || return 1
  CHAIN_ENV="$OUT_DIR/chain.env"
  { echo 'set -a'; cat "$BARDO_COMPOSITOR_ENV"; echo 'set +a'; } >"$CHAIN_ENV"
}

# accept NAME: bardo-portal's acceptance script NAME against this bardo-compositor; its output is kept.
accept() {
  local rc
  BARDO_PORTAL_BIN="$BARDO_PORTAL_BIN" BARDO_PORTAL_DEV_CALLERS="$FLOOR_DIR/desktop-callers.toml" BARDO_PORTAL_DESKTOP="$SCEN_ROOT" BARDO_PORTAL_COMPOSITOR_ENV="$CHAIN_ENV" \
    BARDO_PORTAL_OUTPUT=HEADLESS-1 BARDO_PORTAL_OUTPUT_SIZE="$SIZE" BARDO_PORTAL_STATE="$OUT_DIR/chain-$1" \
    bash "$BARDO_PORTAL_DIR/dev/accept/$1.sh" >"$OUT_DIR/$1.log" 2>&1
  rc=$?
  cat "$OUT_DIR/$1.log" >&2
  return "$rc"
}
step_screenshot() { accept screenshot; }
step_screencast() { accept screencast-output; }
step_settings() { accept settings; }

run_steps step_routing step_boot step_settings step_screenshot step_screencast
if [ "$STATUS" != 0 ]; then
  for f in "$OUT_DIR"/chain-*/bardo-portal.log "$OUT_DIR"/chain-*/portal.log; do
    [ -f "$f" ] || continue
    echo "--- $f (tail)" >&2; tail -20 "$f" >&2
  done
fi
exit "$STATUS"
