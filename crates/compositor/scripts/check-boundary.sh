#!/usr/bin/env bash
# casement's boundary rows (20-FREEZE-PLAN §2.4, B1..B20; 21-FREEZE-AMENDMENTS A1, A4),
# mechanically. ARCHITECTURE.md section 1 is the table; this is its mechanical form.
#
# "Reaches" rows read each crate's normal-edge dependency graph once (`cargo tree -e normal`)
# and look for forbidden package names in it: a check for OUTPUT, never for exit status
# (`cargo tree -i` exits 101 when a dependency is absent, the state we want).
# "Names" rows grep the source. A row about a crate that is not a member yet passes and says so;
# it bites the moment that crate lands.
set -uo pipefail
cd "$(dirname "$0")/.."
fail=0
bad() { echo "LEAK: $*"; fail=1; }
ok() { echo "holds: $*"; }

members=$(cargo metadata --no-deps --format-version 1 2>/dev/null \
  | grep -o '"name":"[^"]*","version"' | sed 's/"name":"\([^"]*\)".*/\1/' | sort -u)
if [ -z "$members" ]; then
  echo "ERROR: cargo metadata found no members; the boundary was not checked"
  exit 1
fi
is_member() { grep -qx "$1" <<<"$members"; }

# Every package a crate reaches through normal edges, one name per line.
reached() {
  cargo tree -p "$1" -e normal --prefix none --format '{p}' 2>/dev/null | awk '{print $1}' | sort -u
}

# reaches_none <crate> <label> <extended regex of package names>
reaches_none() {
  local crate="$1" label="$2" pattern="$3" found
  if ! is_member "$crate"; then ok "$label ($crate is not a member yet)"; return; fi
  found=$(reached "$crate" | grep -E "^($pattern)$" | tr '\n' ' ')
  if [ -n "$found" ]; then bad "$label: $crate reaches ${found% }"; else ok "$label"; fi
}

# names_only <label> <extended regex> <allowed path regex> [search roots...]: the pattern
# appears in Rust code only under paths matching the allowed regex. Comment lines (`//`, `///`,
# `//!`) are not code: a doc may name what a row forbids (B20 checks comments on its own).
names_only() {
  local label="$1" pattern="$2" allowed="$3"; shift 3
  local roots=("$@") hits
  [ ${#roots[@]} -eq 0 ] && roots=(*/)
  hits=$(grep -rnE --include='*.rs' "$pattern" "${roots[@]}" 2>/dev/null \
    | grep -vE '^[^:]+:[0-9]+:[[:space:]]*//' | grep -vE "^($allowed)" || true)
  if [ -n "$hits" ]; then bad "$label"; printf '%s\n' "$hits" | head -20; else ok "$label"; fi
}

# The graph rows of B1: what the pure model may never reach.
PURE_FORBIDDEN='smithay|wayland-[a-z-]+|calloop|calloop-[a-z-]+|zbus|tokio|drm|drm-[a-z-]+|gbm|gbm-[a-z-]+|input|input-sys|udev|libudev-sys|libseat|libseat-sys|dioxus|dioxus-[a-z-]+|blitz[a-z-]*|stylo|stylo_[a-z_]+|ds|ui-shell|ui-blitz|ui-settings|parley|fontique|vello[a-z_]*|wgpu|anyrender[a-z_]*'

# B1 compositor-core: the forbidden list, taffy included.
reaches_none compositor-core "B1 compositor-core is pure" "$PURE_FORBIDDEN|taffy"
# B1 (allowed ours): compositor-core's direct dependencies of our own are only these.
OURS='compositor-[a-z-]+|ui-[a-z-]+|key-remap-[a-z-]+|mouse-gestures-[a-z-]+|account-[a-z-]+|provenance'
ALLOWED_OURS='desktop-protocols|ui-core|ui-style|ui-motion|ui-behaviour|key-remap-core|mouse-gestures-proto|account-core|provenance'
direct=$(cargo tree -p compositor-core -e normal --depth 1 --prefix none --format '{p}' 2>/dev/null \
  | awk 'NR > 1 {print $1}' | grep -E "^($OURS)$" | grep -vE "^($ALLOWED_OURS)$" | tr '\n' ' ')
if [ -n "$direct" ]; then bad "B1 compositor-core depends on ${direct% } (not in the allowed list)"; else ok "B1 compositor-core's own dependencies are allowed ones"; fi

# B2 compositor-layout: B1's list except taffy; only taffy_bridge/ names taffy.
reaches_none compositor-layout "B2 compositor-layout is pure" "$PURE_FORBIDDEN"
names_only "B2 only compositor-layout/src/taffy_bridge/ names taffy" '\btaffy::|use taffy\b|extern crate taffy' \
  'compositor-layout/src/taffy_bridge/'

# B3 compositor-motion: B1's list (taffy arrives through layout, so the source never names it).
reaches_none compositor-motion "B3 compositor-motion is pure" "$PURE_FORBIDDEN"
names_only "B3 compositor-motion names no taffy" '\btaffy::|use taffy\b' '$^' compositor-motion

# B4 compositor-render: no session/device backends, no second graphics stack.
names_only "B4 compositor-render names no drm/udev/session/libinput backend" \
  'smithay::backend::(drm|udev|session|libinput)' '$^' compositor-render
reaches_none compositor-render "B4 compositor-render reaches no second graphics stack" \
  'wgpu|vello[a-z_]*|anyrender[a-z_]*|blitz[a-z-]*|stylo|parley|fontique'

# B5 compositor-backend names no casement_wayland and no smithay::wayland module but dmabuf.
names_only "B5 compositor-backend names no compositor_wayland" '\bcompositor_wayland\b' '$^' compositor-backend
hits=$(grep -rnE --include='*.rs' 'smithay::wayland::[a-z_]+' compositor-backend 2>/dev/null \
  | grep -vE '^[^:]+:[0-9]+:[[:space:]]*//' | grep -oE 'smithay::wayland::[a-z_]+' | grep -v 'smithay::wayland::dmabuf' || true)
if [ -n "$hits" ]; then bad "B5 compositor-backend names a smithay::wayland module"; printf '%s\n' "$hits"; else ok "B5 compositor-backend names only smithay::wayland::dmabuf"; fi

# B6 compositor-wayland: no device backends.
names_only "B6 compositor-wayland names no device backend" 'smithay::backend::(drm|gbm|udev|libinput|session)' '$^' compositor-wayland
reaches_none compositor-wayland "B6 compositor-wayland reaches neither drm nor gbm" 'drm|gbm'

# B7 the thin-adapter allow-list (11 §5.3).
ADAPTERS='compositor-backend/src/tty/(kms_real|surface_real|session_real|udev_real|direct_real|writeback_real|render_hub_real|bridge_real|soft_real|real)\.rs|compositor-backend/src/tty/kms_real/makers\.rs|compositor-backend/src/gpu/(render_nodes|formats)_real\.rs|compositor-backend/src/input/libinput_real\.rs'
# The test kit's soft GPU (the SP4 route behind the `RenderNodes` fake) names DrmNode too: test
# support, not shipped, and not counted as adapter lines.
SOFT_GPU='compositor-testkit/src/gpu/[a-z_]+\.rs|compositor-testkit/tests/it/topology/[a-z_]+\.rs'
# The libinput crate is named `input`, as are casement's own `input` modules: its uses are
# caught by the crate's own items (`input::Libinput`, `input::event::`, ...), not the bare name.
names_only "B7 only the thin adapters name the device crates" \
  'smithay::backend::(drm|libinput|session::libseat|udev|egl::EGLDevice)|(^|[^:[:alnum:]_])(drm|gbm|libseat)::|(^|[^:[:alnum:]_])input::(Libinput|LibinputInterface|event|Device|DeviceCapability|AccelProfile)\b' "$ADAPTERS|$SOFT_GPU"
adapter_lines=$(find compositor-backend/src -type f 2>/dev/null | grep -E "^($ADAPTERS)$" | xargs -r cat | wc -l)
echo "B7 thin adapters: $adapter_lines lines (target <= 1500)"

# B8 (+ A1, SP3): only compositor-app/src/time/ reads a clock or arms a real timer.
names_only "B8 only compositor-app/src/time/ reads a clock" \
  '\b(Instant::now|SystemTime::now|Clock::now)\b|calloop::timer|\bClock::<Monotonic>::new' 'compositor-app/src/time/'

# A4: smithay's UdmabufAllocator::create_buffer over-allocates; ours replaces it.
names_only "A4 nothing calls UdmabufAllocator::create_buffer" 'UdmabufAllocator::create_buffer' '$^'

# B9 the test client reaches no smithay.
reaches_none compositor-testclient "B9 compositor-testclient reaches no smithay" 'smithay'

# B10 test and debug-inject code never reaches the release binary.
if is_member compositor-app; then
  features=$(cargo tree -p compositor-app -e features --prefix none 2>/dev/null | grep -E 'feature "(testing|debug-inject)"' || true)
  if [ -n "$features" ]; then bad "B10 the release casement enables testing or debug-inject"; else ok "B10 the release binary enables neither testing nor debug-inject"; fi
else
  ok "B10 (casement is not a member yet)"
fi

# B11 the binary graph reaches no UI or second graphics stack (vello_cpu only via compositor-chrome).
reaches_none compositor-app "B11 the casement binary reaches no UI stack" \
  'blitz[a-z-]*|stylo|selectors|html5ever|dioxus|dioxus-[a-z-]+|wgpu|anyrender[a-z_]*|vello|vello_hybrid'

# B12 no crate but casement-intents reaches the agent tier or the shell.
TIERS='action-[a-z-]+|agent-[a-z-]+|companion-[a-z-]+|permission-[a-z-]+|computer-use-[a-z-]+|memory-[a-z-]+|shell-(app|bar|dock|ipc|kit|launcher|launcher-ui|model|notify-ui|overlays|search|services|settings|testkit|ui-kit)|layer-shell-host|model-[a-z-]+|engine-supervisor|model-vision-prep|speech-provider'
for crate in $members; do
  [ "$crate" = casement-intents ] && continue
  reaches_none "$crate" "B12 $crate reaches no agent tier or shell crate" "$TIERS"
done

# B13 ui-settings only in compositor-ipc and casement (the test kit reaches it only through the
# `casement` crate it drives; it names no ui-settings item itself, checked below).
for crate in $members; do
  case "$crate" in compositor-ipc|compositor-app|compositor-testkit) continue ;; esac
  reaches_none "$crate" "B13 $crate reaches no ui-settings" 'ui-settings'
done

names_only "B13 the test kit names no ui-settings" '\bds_settings::' '$^' compositor-testkit

# B14 only compositor-agent names reis, the role hand-out or EIS fds.
names_only "B14 only compositor-agent names reis or the privileged hand-out" \
  '\breis::|ConnectionRole::(Shell|Agent)\b.*(hand_out|socket)|\bEisFd\b' 'compositor-agent/'

# B15 only compositor-chrome reaches the text stack. Its one consumer, `casement`, reaches it
# through the crate's dependency (so does the test kit, which drives `casement`); no other crate
# does, and no source outside compositor-chrome names it.
for crate in $members; do
  case "$crate" in compositor-chrome|compositor-app|compositor-testkit) continue ;; esac
  reaches_none "$crate" "B15 $crate reaches no text stack" 'parley|fontique|vello_cpu'
done
names_only "B15 only compositor-chrome names the text stack" \
  '\b(parley|fontique|vello_cpu)::|extern crate (parley|fontique|vello_cpu)' 'compositor-chrome/'

# B16 the excluded crates are never members.
for excluded in wlcs-compositor; do
  if is_member "$excluded"; then bad "B16 $excluded is a workspace member"; else ok "B16 $excluded is not a member"; fi
done

# B17 no hash order reaches the wire or a render order.
names_only "B17 no HashMap/HashSet in wayland, render, layout, motion" '\bHash(Map|Set)\b' '$^' \
  compositor-wayland compositor-render compositor-layout compositor-motion

# B18 no unsafe outside the reserved module (the workspace also denies unsafe_code).
names_only "B18 no unsafe outside compositor-render/src/gl_raw.rs" '\bunsafe\b' 'compositor-render/src/gl_raw\.rs'

# B19 casement-intents reaches no compositor stack.
reaches_none casement-intents "B19 casement-intents reaches no smithay, wayland or calloop" 'smithay|wayland-[a-z-]+|calloop'

# B20 clean-room tripwire: no use, file name or comment naming a GPL compositor.
TRIP='\b(niri|jay|pinnacle)\b'
hits=$(grep -rniE "$TRIP" . 2>/dev/null; find . -iregex ".*\(niri\|pinnacle\).*" 2>/dev/null)
if [ -n "$hits" ]; then bad "B20 a GPL compositor is named in code"; printf '%s\n' "$hits" | head -20; else ok "B20 no GPL compositor named in code"; fi

exit "$fail"
