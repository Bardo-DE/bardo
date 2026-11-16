# What every scenario's bardo-compositor is told about the programs it will see (design 3.8, the owner's "ask per app").
#
# bardo-compositor asks the person the first time a program watches the screen, types, reads the clipboard, lists
# windows, uses an input method, locks the screen or changes the displays. Nobody answers inside a scenario, so
# a scenario starts its bardo-compositor with `--system-dir DIR` (a debug build reads it as the administrator's floor,
# owned by the running user) and a floor of `allow` rows for exactly the programs its steps run. This file is the
# one place that writes the floor; no scenario writes privacy.toml itself.
#
#   floor_make NAME [TOOL...]   writes the floor for NAME (privacy.toml) and the desktop's caller list for it
#                               (desktop-callers.toml); sets FLOOR_DIR. No TOOL is a floor with no rows, which is
#                               right for a scenario whose programs use no power and are no desktop program
#   floor_shim NAME [TOOL...]   floor_make, then points BARDO_COMPOSITOR_REPO and BARDO_COMPOSITOR_DIR at a stand-in checkout
#                               whose dev/up.sh adds `--system-dir` and whose dev/down.sh is bardo-compositor's own. For
#                               scripts that start bardo-compositor themselves (bardo-shell's and layer-shell-host's libraries, whose
#                               pins may predate the flag): they call "$BARDO_COMPOSITOR_REPO/dev/up.sh" and never see
#                               the difference. The flag is added last, so this floor is the one bardo-compositor reads
#
# A TOOL is a program the scenario starts and bardo-compositor will see connect: a power tool from the system (grim, fcitx5,
# ...), or a program built under build/ (bardo-compositorctl, compositor-probe, bardo-portal, bardo-shell, bardo-settings, the layer-shell-host demos).
# It is named by its real path (`readlink -f`), which is what bardo-compositor reads from the process. Two things come out:
#
#   privacy.toml          an `allow` row per power the TOOL takes (floor_powers). bardo-compositor reads the floor of a
#                         debug-inject build from `--system-dir DIR` and lets a row name a program run from a
#                         build folder (the person can write it), which in a real install it never does
#   desktop-callers.toml  a row per TOOL that is one of the desktop's own programs (floor_role: the shell, Settings,
#                         the portal), in the shape of dist/desktop-callers.toml with no `unit`: bardo-compositor reads it
#                         from the same folder, in the same build, in place of /etc/bardo/desktop-callers.toml,
#                         which the jail has not got. Without it bardo-shell, bardo-settings and bardo-portal are nobody, and the
#                         shell cannot connect
#
# Display changes: a program that changes the outputs (wlr-randr and the like) is asked by bardo-compositor whether to keep
# the new setup and the change goes back after 15 s (bardo_shell_v1 version 6). No scenario does that: the scale
# and size changes they make go through `bardo-compositorctl debug output set`, the compositor's own test control. A
# scenario that does change outputs through a program must answer Keep through the shell or expect the revert.

# floor_powers TOOL: the powers TOOL takes on bardo-compositor's socket, space separated (none for a program that takes no
# power a row can name); fails for a tool not listed.
floor_powers() {
  case "$1" in
    grim | wf-recorder | wayshot | bardo-portal) echo screen ;;
    bardo-compositorctl) echo screen windows ;; # `shot`: the capture, and the toplevel list it binds to name the windows
    wl-copy | wl-paste | cliphist) echo clipboard ;;
    wtype) echo keys ;;
    fcitx5) echo typing_help keys ;; # the input method, and the virtual keyboard it forwards keys through
    wlr-randr | kanshi) echo displays ;;
    bardo-computer-use) echo screen keys windows ;; # the screen helper: looks at windows, types and clicks in them (W2)
    wlrctl | lswt | compositor-probe) echo windows ;; # compositor-probe: `workspaces` (ext_workspace_manager)
    swaylock | demo_lock) echo lock ;;
    bardo-shell | bardo-settings | demo_* | compositor-shell-exec) ;; # the desktop's programs, the demos and their launcher: the role below, no power
    bardo-accountd | bardo-inferd | bardo-syncd | xdg-desktop-portal | bardo-compositor | fake-shell) ;; # services the desktop's programs check by role (bardo-shell, bardo-settings, bardo-portal)
    *) return 1 ;;
  esac
}

# floor_role TOOL: the role TOOL plays in the desktop's caller list (the roles bardo-compositor checks: shell, settings,
# portal, agent_tier; and the services bardo-shell, bardo-settings and bardo-portal check by role: accounts, inference, sync,
# portal_frontend, compositor); nothing
# for a program that is none. The layer-shell-host demos stand in for the shell: they map the shell's
# surfaces (an exclusive keyboard, an overlay), which bardo-compositor gives only to the shell's connection. The ones that
# need it start through compositor-shell-exec (layer-shell-host's comp_as_shell), which asks Compositor1.Connect("shell")
# for that connection, so it is the program that must hold the role. demo_lock is a screen locker, a power a floor
# row allows, and is not the shell.
floor_role() {
  case "$1" in
    bardo-shell | compositor-shell-exec | demo_hover | demo_scroll | demo_ime | demo_swipe | demo_switcher | demo_drag | demo_popup_keys) echo shell ;;
    bardo-settings) echo settings ;;
    bardo-portal) echo portal ;;
    bardo-accountd) echo accounts ;;
    bardo-inferd) echo inference ;;
    bardo-syncd) echo sync ;;
    xdg-desktop-portal) echo portal_frontend ;;
    bardo-compositor) echo compositor ;;
    bardo-computer-use) echo agent_tier ;; # the screen helper: bardo-compositor leases it a window (Compositor1.LeaseWindow)
    fake-shell) echo shell ;; # bardo-portal's dev/lib fake shell (python3), which owns org.bardo.Shell in portal-bardo
    *) return 1 ;;
  esac
}

# floor_app ROLE: the app id dist/desktop-callers.toml gives ROLE's row.
floor_app() {
  case "$1" in
    shell) echo org.bardo.Shell ;;
    settings) echo org.bardo.Settings ;;
    portal) echo org.bardo.Portal ;;
    accounts) echo org.bardo.Accounts ;;
    inference) echo org.bardo.Inference ;;
    sync) echo org.bardo.Sync ;;
    portal_frontend) echo org.freedesktop.portal.Desktop ;;
    compositor) echo org.bardo.Compositor ;;
    agent_tier) echo org.bardo.Agent ;;
  esac
}

# floor_image TOOL: the real path of the program TOOL names (built under build/ for ours, else the one on PATH);
# fails when it is not there (not built, not installed).
floor_image() {
  local path
  case "$1" in
    bardo-compositorctl | compositor-probe | compositor-shell-exec) path="${SCEN_TOOLS:?floor_image: scenario.sh is not sourced}/bin/$1" ;;
    bardo-portal) path="${SCEN_BUILD:?floor_image: scenario.sh is not sourced}/transom-target/debug/bardo-portal" ;;
    bardo-shell) path="${FLOOR_SILL_TARGET:-${SCEN_BUILD:?floor_image: scenario.sh is not sourced}/shell-target}/debug/bardo-shell" ;; # FLOOR_SILL_TARGET: the accounts rig runs bardo-shell at its pin
    bardo-accountd) path="${SCEN_BUILD:?floor_image: scenario.sh is not sourced}/porter-target/debug/bardo-accountd" ;;
    bardo-syncd) path="${SCEN_BUILD:?floor_image: scenario.sh is not sourced}/ferry-target/debug/bardo-syncd" ;; # ferry's, since sync left porter
    bardo-inferd) path="${SCEN_BUILD:?floor_image: scenario.sh is not sourced}/steward-target/debug/bardo-inferd" ;;
    xdg-desktop-portal) path=/usr/libexec/xdg-desktop-portal ;;
    bardo-compositor) path="${SCEN_TOOLS:?floor_image: scenario.sh is not sourced}/bin/bardo-compositor" ;;
    bardo-computer-use) path="${SCEN_BUILD:?floor_image: scenario.sh is not sourced}/cua-target/debug/bardo-computer-use" ;;
    fake-shell) path=$(command -v python3 2>/dev/null) || return 1 ;;
    bardo-settings) path="${SCEN_BUILD:?floor_image: scenario.sh is not sourced}/detent-target/debug/bardo-settings" ;;
    demo_*) path="${SCEN_BUILD:?floor_image: scenario.sh is not sourced}/shell-host-target/debug/examples/$1" ;;
    *) path=$(command -v "$1" 2>/dev/null) || return 1 ;;
  esac
  [ -e "$path" ] && readlink -f "$path"
}

# floor_rows TOOL...: one allow row per power of each TOOL that is installed or built (one that is not is left
# out: the step that needs it says so). Fails for a tool floor_powers does not know.
floor_rows() {
  local tool power path
  for tool in "$@"; do
    floor_powers "$tool" >/dev/null || { echo "floor_rows: no powers are known for $tool (add it to floor_powers)" >&2; return 2; }
    path=$(floor_image "$tool") || continue
    for power in $(floor_powers "$tool"); do
      printf '\n[[rule]]\npower = "%s"\nsubject = { program = "%s" }\nanswer = "allow"\n' "$power" "$path"
    done
  done
}

# floor_callers TOOL...: the desktop's caller list for these programs: one [[peer]] per role, naming the images of
# every TOOL that plays it and is built (dist/desktop-callers.toml's shape, without a `unit`). Empty when no TOOL
# plays a role.
floor_callers() {
  local role tool path exes
  for role in shell settings portal portal_frontend compositor agent_tier accounts inference sync; do
    exes=
    for tool in "$@"; do
      [ "$(floor_role "$tool" 2>/dev/null)" = "$role" ] || continue
      path=$(floor_image "$tool") || continue
      exes="$exes${exes:+, }\"$path\""
    done
    [ -z "$exes" ] || printf '\n[[peer]]\napp = "%s"\nrole = "%s"\nexe = [%s]\n' "$(floor_app "$role")" "$role" "$exes"
  done
}

# floor_make NAME [TOOL...]: the floor in $JAIL_SCRATCH/compositor-floor/NAME (a folder only this user can enter, with
# privacy.toml and desktop-callers.toml in it, each mode 600 and ours: the shape bardo-compositor's test build reads); sets
# FLOOR_DIR.
FLOOR_DIR=
floor_make() {
  local name=$1 dir rows callers
  shift
  dir="${JAIL_SCRATCH:?floor_make: not in the jail}/compositor-floor/$name"
  rows=$(floor_rows "$@") || return 1
  callers=$(floor_callers "$@") || return 1
  rm -rf "$dir" && mkdir -p "$(dirname "$dir")" && mkdir -m 700 "$dir" || return 1
  { echo "# The floor of scenario $name: the programs its steps run (harness/lib/compositor-floor.sh)."; echo "$rows"; } >"$dir/privacy.toml" || return 1
  { echo "# The desktop's own programs in scenario $name, by the images the harness built (harness/lib/compositor-floor.sh)."; echo "$callers"; } >"$dir/desktop-callers.toml" || return 1
  chmod 600 "$dir/privacy.toml" "$dir/desktop-callers.toml" || return 1
  FLOOR_DIR=$dir
}

# floor_sill_callers: bardo-shell (a `debug` build) takes the floor's desktop-callers.toml as the person's own rows
# (`bardo-shell run --dev-callers FILE`, peephole's Proof::Dev), so the services and the portal it checks by role are known.
# Call it after sourcing bardo-shell's dev/lib (which clears DEV_CALLERS_FLAGS) and before launch_daemon. bardo-settings (feature
# dev-callers) takes the same file as `bardo-settings --dev-callers FILE` and bardo-portal (feature debug) as BARDO_PORTAL_DEV_CALLERS.
floor_sill_callers() {
  DEV_CALLERS_FLAGS=(--dev-callers "${FLOOR_DIR:?floor_sill_callers: call floor_make or floor_shim first}/desktop-callers.toml")
}

# floor_shim NAME [TOOL...]: floor_make, then the stand-in checkout (see the top of this file).
floor_shim() {
  local real="${SCEN_SIB:?floor_shim: scenario.sh is not sourced}/compositor" repo
  floor_make "$@" || return 1
  repo="$FLOOR_DIR.repo"
  rm -rf "$repo" && mkdir -p "$repo/dev" || return 1
  printf '#!/usr/bin/env bash\nexec %q/dev/up.sh "$@" --system-dir %q\n' "$real" "$FLOOR_DIR" >"$repo/dev/up.sh" || return 1
  printf '#!/usr/bin/env bash\nexec %q/dev/down.sh "$@"\n' "$real" >"$repo/dev/down.sh" || return 1
  chmod 700 "$repo/dev/up.sh" "$repo/dev/down.sh" || return 1
  export BARDO_COMPOSITOR_REPO=$repo BARDO_COMPOSITOR_DIR=$repo
}
