#!/usr/bin/env bash
# bardo-portal's boundary rows, mechanically. ARCHITECTURE.md section 1 is the table; this is its
# mechanical form. Rows:
#   T1 portal-core reaches no zbus, tokio, pipewire, wayland, image or tracing
#   T2 only portal-daemon/src/capture/ names the wayland crates
#   T3 only portal-daemon/src/screencast/pipewire.rs names the pipewire crate (a path through bardo-portal's own `pipewire` module, `self::pipewire::` or `screencast::pipewire::`, is not the crate)
#   T4 only portal-daemon/src/screenshot.rs (P2-T1) names the image crate
#   T5 no clock, no environment read and no file I/O in portal-core
#   T6 the environment is read only in portal-daemon/src/main.rs
#   T7 no unsafe anywhere
#   T8 no other portal backend's name in code or comments (they are facts at most, never sources)
#   T9 no bool fields or parameters in portal-core's public items
set -uo pipefail
cd "$(dirname "$0")/.."
fail=0
bad() { echo "LEAK: $*"; fail=1; }
ok() { echo "holds: $*"; }

reached() {
  cargo tree -p "$1" -e normal --prefix none --format '{p}' 2>/dev/null | awk '{print $1}' | sort -u
}

# T1: a check on OUTPUT (cargo tree exits non-zero in states we do not mind).
tree=$(reached portal-core)
if [ -z "$tree" ]; then
  echo "ERROR: cargo tree found nothing for portal-core; the boundary was not checked"
  exit 1
fi
found=$(printf '%s\n' "$tree" | grep -E '^(zbus[a-z_-]*|zvariant[a-z_-]*|tokio[a-z_-]*|pipewire[a-z_-]*|libspa[a-z_-]*|wayland-[a-z-]+|image|tracing[a-z_-]*|async-[a-z-]+)$' | tr '\n' ' ')
if [ -n "$found" ]; then bad "T1 portal-core reaches ${found% }"; else ok "T1 portal-core is pure"; fi

# names_only <label> <extended regex> <allowed path regex> [roots...]: the pattern appears in
# Rust code only under paths matching the allowed regex. Comment lines are not code.
names_only() {
  local label="$1" pattern="$2" allowed="$3"; shift 3
  local roots=("$@") hits
  [ ${#roots[@]} -eq 0 ] && roots=(*/)
  hits=$(grep -rnE --include='*.rs' "$pattern" "${roots[@]}" 2>/dev/null \
    | grep -vE '^[^:]+:[0-9]+:[[:space:]]*//' | grep -vE "^($allowed)" || true)
  if [ -n "$hits" ]; then bad "$label"; printf '%s\n' "$hits" | head -20; else ok "$label"; fi
}

names_only "T2 only capture/ names the wayland crates" '\bwayland_(client|protocols|backend)\b' 'portal-daemon/src/capture/'
names_only "T3 only screencast/pipewire.rs names pipewire" '(^|[^:[:alnum:]_])(pipewire|libspa)::' 'portal-daemon/src/screencast/pipewire\.rs'
names_only "T4 only screenshot.rs names image" '\bimage::' 'portal-daemon/src/screenshot\.rs'
names_only "T5 portal-core reads no clock, environment or file" \
  '\b(Instant::now|SystemTime::now|std::env::|env::var|std::fs::|File::open|File::create)\b' '$^' portal-core
names_only "T6 only main.rs reads the environment" '\b(std::env::|env::var)' 'portal-daemon/src/main\.rs' portal-daemon/src
names_only "T7 no unsafe" '\bunsafe\b' '$^'

# T8 is checked on comments too: another backend is a fact to read about, never code to copy.
hits=$(grep -rniE --include='*.rs' 'xdg-desktop-portal-(wlr|gnome|kde|luminous|hyprland|gtk)|xdpw' */ 2>/dev/null || true)
if [ -n "$hits" ]; then bad "T8 another backend is named"; printf '%s\n' "$hits" | head; else ok "T8 no other backend named"; fi

# T9: a `bool` in portal-core's public signatures or fields (a predicate's return type is fine).
hits=$(grep -rnE --include='*.rs' 'pub [a-z_]+: bool|[(,][[:space:]]*[a-z_]+: bool[,)]' portal-core/src 2>/dev/null || true)
if [ -n "$hits" ]; then bad "T9 a bool field or parameter in portal-core"; printf '%s\n' "$hits" | head; else ok "T9 no bool fields or parameters in portal-core"; fi

exit $fail
