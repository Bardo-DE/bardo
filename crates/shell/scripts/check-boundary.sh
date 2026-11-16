#!/usr/bin/env bash
# Crate boundaries, mechanically enforced.
#
# `cargo tree -i <dep>` exits 101 when the dependency is absent, which is precisely the state
# we want. Checking the exit status would therefore fail whenever the boundary holds, so we
# check for OUTPUT instead: any line naming the dependency is a leak.
#
# Each rule is "<crate>: <forbidden deps...>". Forbidden names are exact package names; a
# family such as blitz-* is spelled out member by member.
set -uo pipefail
cd "$(dirname "$0")/.."

# ARCHITECTURE.md section 1 is the crate map; this file is its mechanical form, in two parts.
#
# RULES: what a crate reaches through ANY path (`cargo tree -i`, transitive).
# shell-launcher, shell-model and shell-ipc are pure: no UI, no D-Bus, no Wayland, no runtime, no
# audio, no design system. shell-settings is data and UI-free, but it names ui-settings (its dioxus
# feature off), whose file I/O brings zbus and tokio, so only the UI is forbidden to it.
# shell-services and shell-search are effectful but UI-free: their state reaches surfaces only
# through watch channels read by shell-ui-kit's hooks, so neither links a renderer. shell-services
# names ui-settings (its dioxus feature off) for Night Shift's SettingsWriter edits.
# shell-testkit is std only and names no bardo-shell crate.
# shell-kit (the shell-only parts over quire's `ds`) names no other bardo-shell crate and no service,
# bus, runtime or compositor client: it is what the surfaces and shell-ui-kit draw with.
# shell-ui-kit is what every surface shares, so it reaches no surface crate and not the binary.
# `bardo-shell` (the whole process) takes the viewer's light tier, its history, its vocabulary and its
# D-Bus requests, and never its media player: it reaches neither libmpv binding.
# The surface crates never depend on one another and none depends on `bardo-shell`; the daemon is the one
# crate that names them all.
PURE="dioxus dioxus-core zbus wayland-client tokio libpulse-binding layer-shell-host ui-components ui-shell shell-kit ui-blitz ui-harness blitz-dom"
UI_FREE="dioxus dioxus-core blitz-dom layer-shell-host ui-components ui-shell shell-kit ui-blitz ui-harness"
SURFACES=(shell-overlays shell-notify-ui shell-dock shell-launcher-ui shell-bar)
# The other surfaces, and the binary, for one surface crate.
others() {
  local self="$1" c
  for c in "${SURFACES[@]}" shell-app; do [ "$c" = "$self" ] || printf '%s ' "$c"; done
}
RULES=(
  "shell-launcher: $PURE"
  "shell-model: $PURE"
  "shell-settings: $UI_FREE"
  "shell-ipc: $PURE"
  "shell-services: $UI_FREE"
  "shell-search: $UI_FREE"
  "shell-kit: shell-launcher shell-model shell-settings shell-ipc shell-services shell-search shell-testkit shell-ui-kit ${SURFACES[*]} shell-app zbus tokio wayland-client libpulse-binding layer-shell-host ui-blitz blitz-dom"
  "shell-testkit: shell-launcher shell-model shell-settings shell-ipc shell-services shell-search shell-kit shell-ui-kit ${SURFACES[*]} shell-app"
  "shell-ui-kit: ${SURFACES[*]} shell-app"
  "shell-app: mpv-wgpu-player rsmpv"
)
for surface in "${SURFACES[@]}"; do
  RULES+=("$surface: $(others "$surface")")
done
fail=0

for rule in "${RULES[@]}"; do
  crate="${rule%%:*}"
  read -r -a forbidden <<<"${rule#*:}"
  # A crate that cargo cannot find would make every check below pass vacuously.
  if ! cargo tree -p "$crate" --depth 0 >/dev/null 2>&1; then
    echo "ERROR: cargo tree cannot resolve $crate; the boundary was not checked"
    fail=1
    continue
  fi
  leaked=0
  for dep in "${forbidden[@]}"; do
    if cargo tree -p "$crate" -i "$dep" -e normal,build 2>/dev/null | grep -q .; then
      echo "LEAK: $crate depends on $dep"
      cargo tree -p "$crate" -i "$dep" -e normal,build 2>/dev/null | head -20
      leaked=1
      fail=1
    fi
  done
  if [ "$leaked" -eq 0 ]; then
    echo "boundary holds: $crate reaches none of ${forbidden[*]}"
  fi
done

# The allowed edges between our own crates and quire and layer-shell-host (ARCHITECTURE.md section 1):
# each crate's DIRECT normal and build dependencies that are path dependencies, and nothing else.
# A dependency on a crate not listed is a leak; so is one the crate no longer has, so the table
# stays exact. Dev dependencies are outside it (a test may reach further than the crate does).
EDGES=(
  "shell-launcher: memory-core action-core action-vocab ui-core provenance"
  "shell-testkit:"
  "shell-kit: ui-components ui-core ui-shell"
  "shell-model: memory-core desktop-protocols companion-model companion-wire computer-use-action computer-use-bus action-core action-manifest permission-core action-vocab action-wire ui-core ui-helpers key-remap-proto account-capability account-core account-ids inference-core account-sheet provenance shell-launcher agent-voice-wire"
  "shell-settings: ui-core ui-settings key-remap-proto shell-launcher shell-model"
  "shell-ipc: ui-core shell-launcher shell-model"
  "shell-services: memory-core memory-dbus anyview-core anyview-platform desktop-protocols companion-model companion-wire computer-use-action computer-use-bus computer-use-dbus desktop-client action-client action-core action-dbus action-manifest permission-core action-provider action-vocab action-wire ui-core ui-helpers ui-settings inference-client key-remap-proto account-client account-client-dbus sync-bus account-core account-dbus account-ids inference-core account-sheet inference-tailnet account-tailscale account-wire provenance shell-launcher shell-model agent-voice-dbus agent-voice-wire"
  "shell-search: memory-core anyview-core anyview-store companion-model companion-wire computer-use-action action-core action-manifest permission-core action-vocab ui-core ui-settings provenance shell-launcher shell-model shell-services"
  "shell-ui-kit: companion-wire computer-use-action action-core action-manifest permission-core action-vocab ui-components ui-behaviour ui-blitz ui-core ui-harness ui-settings key-remap-client account-capability account-core provenance layer-shell-host shell-launcher shell-model shell-search shell-services shell-settings shell-kit"
  "shell-bar: ui-components ui-blitz ui-settings layer-shell-host shell-launcher shell-model shell-services shell-settings shell-kit shell-ui-kit"
  "shell-dock: ui-components ui-blitz layer-shell-host shell-launcher shell-model shell-services shell-settings shell-kit shell-ui-kit"
  "shell-launcher-ui: memory-core anyview-core anyview-fs anyview-image anyview-peek anyview-platform companion-model companion-wire action-core permission-core action-vocab ui-components ui-behaviour ui-blitz provenance layer-shell-host shell-launcher shell-model shell-search shell-services shell-settings shell-kit shell-ui-kit"
  "shell-notify-ui: ui-components ui-blitz ui-settings layer-shell-host shell-model shell-services shell-settings shell-kit shell-ui-kit"
  "shell-overlays: ui-components ui-behaviour ui-settings ui-shell account-capability account-core account-ids account-sheet layer-shell-host shell-launcher shell-model shell-services shell-settings shell-kit shell-ui-kit"
  "shell-app: ui-components ui-blitz ui-core ui-extensions ui-lint ui-settings provenance layer-shell-host shell-verbs shell-bar shell-dock shell-ipc shell-launcher shell-launcher-ui shell-model shell-notify-ui shell-overlays shell-search shell-services shell-settings shell-kit shell-ui-kit"
)
for edge in "${EDGES[@]}"; do
  crate="${edge%%:*}"
  read -r -a allowed <<<"${edge#*:}"
  found=$(cargo tree -p "$crate" --depth 1 -e normal,build --prefix none --all-features 2>/dev/null \
    | grep '(/' | awk '{print $1}' | grep -vx "$crate" | sort -u | tr '\n' ' ')
  want=$(printf '%s\n' "${allowed[@]}" | grep . | sort -u | tr '\n' ' ')
  if [ "$found" != "$want" ]; then
    echo "EDGE: $crate depends on [${found% }], the table allows [${want% }]"
    fail=1
  else
    echo "edges hold: $crate depends on [${found% }]"
  fi
done

# Every workspace member has a row above, so a new crate cannot slip in unchecked.
for member in $(cargo metadata --no-deps --format-version 1 2>/dev/null | grep -oE '"name":"shell-(app|bar|dock|ipc|kit|launcher|launcher-ui|model|notify-ui|overlays|search|services|settings|testkit|ui-kit)"' | cut -d'"' -f4 | sort -u); do
  printf '%s\n' "${EDGES[@]}" | grep -q "^$member:" || { echo "ERROR: $member has no row in EDGES"; fail=1; }
done

exit "$fail"
