#!/usr/bin/env bash
# Crate boundaries, mechanically enforced.
#
# `cargo tree -i <dep>` exits 101 when the dependency is absent, which is precisely the state
# we want. Checking the exit status would therefore fail whenever the boundary holds, so we
# check for OUTPUT instead: any line naming the dependency is a leak.
set -uo pipefail
cd "$(dirname "$0")/.."

# ARCHITECTURE.md section 1 is the crate map; this file is its mechanical form, in two parts.
#
# RULES: what a crate reaches through ANY path (`cargo tree -i`, transitive).
# settings-app-model and settings-app-files are pure: no UI, no renderer, no runtime. They name ui-settings
# for its schema types and its lenient-read types, and ui-settings builds zbus for its portal
# watch, so zbus cannot be in the forbidden list until ui-settings splits its schema module out
# (FINDINGS.md "ui-settings drags zbus into a pure crate"). settings-app-testkit is std only.
# settings-app-intents answers the desktop's intents with no window: it reaches no UI crate.
PURE="dioxus dioxus-core tokio blitz-dom ds ui-blitz ui-harness ui-lint"
RULES=(
  "settings-app-model: $PURE"
  "settings-app-files: $PURE"
  "settings-app-callers: dioxus dioxus-core blitz-dom ds ui-blitz ui-harness ui-lint settings-app-model settings-app-files settings-app-intents settings-app-ui bardo-settings"
  "settings-app-testkit: $PURE zbus ui-settings settings-app-model settings-app-files settings-app-ui bardo-settings"
  "settings-app-link: dioxus dioxus-core blitz-dom ds ui-blitz ui-harness ui-lint settings-app-system settings-app-client settings-app-ui settings-app-intents bardo-settings"
  "settings-app-system: dioxus dioxus-core blitz-dom ds ui-blitz ui-harness ui-lint settings-app-client settings-app-ui settings-app-intents bardo-settings"
  "settings-app-client: dioxus dioxus-core blitz-dom ds ui-blitz ui-harness ui-lint settings-app-system settings-app-ui settings-app-intents bardo-settings"
  "settings-app-ui: bardo-settings settings-app-intents"
  "settings-app-intents: dioxus dioxus-core blitz-dom ds ui-blitz ui-harness ui-lint settings-app-ui bardo-settings"
)
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

# DIRECT: what a crate may not NAME in its own [dependencies], though it reaches them through the
# crate under it. The window reaches the computer only through settings-app-client, so the compiler
# refuses a use of the files, the services, porter, the memory service, the actions service or the bus in a view
# (ARCHITECTURE.md "The fence"). The EDGES row below is the exact list of what it may name.
DIRECT=(
  "settings-app-ui: settings-app-files settings-app-link settings-app-system sync-bus account-client account-core account-dbus inference-core account-provider memory-core action-core action-settings action-dbus action-provider agent-install agent-registry account-ids account-capability account-match account-wire account-sheet inference-tailnet account-tailscale action-vocab action-manifest action-wire action-gate-wire permission-core companion-model zbus"
)
for rule in "${DIRECT[@]}"; do
  crate="${rule%%:*}"
  read -r -a forbidden <<<"${rule#*:}"
  named=$(cargo tree -p "$crate" --depth 1 -e normal,build --prefix none --all-features 2>/dev/null | awk '{print $1}')
  leaked=0
  for dep in "${forbidden[@]}"; do
    if printf '%s\n' "$named" | grep -qx "$dep"; then
      echo "LEAK: $crate names $dep in its own dependencies"
      leaked=1
      fail=1
    fi
  done
  if [ "$leaked" -eq 0 ]; then
    echo "fence holds: $crate names none of ${forbidden[*]}"
  fi
done

# The allowed edges between our own crates and Bardo (ARCHITECTURE.md section 1): each crate's
# DIRECT normal and build dependencies that are path dependencies, and nothing else. A
# dependency on a crate not listed is a leak; so is one the crate no longer has, so the table
# stays exact. Dev dependencies are outside it (a test may reach further than the crate does).
EDGES=(
  "settings-app-testkit:"
  "settings-app-callers: ui-settings"
  "settings-app-model: memory-core action-core action-manifest action-settings action-vocab ui-helpers ui-settings account-capability account-core account-ids inference-core account-sheet inference-tailnet account-tailscale"
  "settings-app-files: agent-registry settings-app-model agent-install action-settings ui-extensions ui-helpers ui-settings"
  "settings-app-link: settings-app-model account-tailscale account-wire compositor-dbus inference-dbus memory-core settings-dbus settings-types sync-bus base-time"
  "settings-app-client: settings-app-files settings-app-link settings-app-model ui-settings inference-core account-provider"
  "settings-app-intents: settings-app-callers settings-app-client settings-app-files settings-app-model action-core action-dbus action-manifest action-provider action-vocab action-wire base-time ui-settings account-core account-ids prov"
  "settings-app-system: settings-app-link memory-core anyview-core anyview-fs anyview-image anyview-platform desktop-protocols desktop-client settings-app-callers settings-app-files settings-app-model base-time ui-settings engine-install engine-manifest engine-probe engine-spec inference-client model-catalog account-callers account-client account-client-dbus account-core account-dbus inference-tailnet account-tailscale account-wire inference-dbus sync-bus"
  "settings-app-ui: settings-app-client settings-app-model ds ui-blitz ui-helpers ui-lint ui-settings"
  "bardo-settings: settings-app-callers settings-app-client settings-app-files settings-app-intents settings-app-link settings-app-model settings-app-system settings-app-ui ui-blitz ui-settings"
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
for member in $(cargo metadata --no-deps --format-version 1 2>/dev/null | grep -o '"name":"bardo-settings[a-z-]*"' | cut -d'"' -f4 | sort -u); do
  printf '%s\n' "${EDGES[@]}" | grep -q "^$member:" || { echo "ERROR: $member has no row in EDGES"; fail=1; }
done

exit "$fail"
