#!/usr/bin/env bash
# The desktop gate (plan §4.14, §6.4). Every exit code is checked; nothing is piped or tailed.
#
#   desktop-gate.sh --self                 the packaging's own checks, nothing built
#   desktop-gate.sh --merge
#   desktop-gate.sh --phase P<n>
#
# --self    the jail's selftest (inside, 2 fabricated GPUs), its negative check (outside), install.sh against stub installers (gate/installer-check.sh), release/selftest.sh (a SKIP, exit 2, when minisign is missing), the distro helpers (gate/distro-check.sh), the three input and caller checks (gate/check-callers-table.sh: every role of peephole's list has a row in dist/desktop-callers.toml; gate/check-no-fake.sh; gate/check-no-uinput-access.sh; gate/check-loader-unset.sh: every trusted unit throws away peephole's loader variables) with their own test on generated trees, and every scenario in the jail (an unlisted SKIP fails). The table check reads peephole's list from the peephole crate of this tree.
# --merge   --self, plus the whole tree through ~/rs-wt/gate-lane-jailed.sh (it hands the bardo tree to its own gate), and the
#           check that no trace of the removed compositor stack is left in the tree.
# --phase   --merge, and additionally an entry of expected-skips.toml whose `closes` is P<n> or
#           earlier must be gone: the list is empty at that phase's exit; and bardo-shell's own acceptance
#           suite on bardo-compositor (crates/shell/dev/accept.sh --compositor casement, its skips
#           listed in shell-accept-skips.txt). The scenarios launcher-p95-on-compositor and
#           bardo-settings-on-casement-at-1.5 are the phase's other checks and run with every scenario.
#
# Exit codes (plan §6.1): 0 green; otherwise the worst step's code. Prints
# "DESKTOP GATE GREEN (<tier>)" only when every step passed.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
# shellcheck source=../harness/lib/all.sh
source "$HERE/../harness/lib/all.sh"
# shellcheck source=../install/distro.sh
source "$ROOT/install/distro.sh"
# shellcheck source=../install/packages.sh
source "$ROOT/install/packages.sh"
JAIL="$ROOT/harness/jail.sh"
TREE="$(cd "$ROOT/.." && pwd)"
LANE_GATE="${GATE_LANE_JAILED:-$HOME/rs-wt/gate-lane-jailed.sh}"

tier="" phase=""
while [ $# -gt 0 ]; do
  case "$1" in
    --self) tier=self; shift ;;
    --merge) tier=merge; shift ;;
    --phase) tier=phase; phase=${2:-}; [[ "$phase" =~ ^P[0-9]+$ ]] || { echo "usage: --phase P<n>" >&2; exit 64; }; shift 2 ;;
    *) sed -n '2,19p' "$0" >&2; exit 64 ;;
  esac
done
[ -n "$tier" ] || { sed -n '2,19p' "$0" >&2; exit 64; }
label=$tier; [ "$tier" != phase ] || label="phase $phase"

# Preflight: environment incomplete is exit 3 with the fix printed.
preflight() {
  command -v bwrap >/dev/null || { echo "missing bubblewrap: $(install_hint bwrap | tr '\n' ' ')"; return 3; }
  [ -c /dev/udmabuf ] || { echo "missing /dev/udmabuf: log in locally (uaccess) or $(as_root_hint modprobe udmabuf)"; return 3; }
  if [ "$tier" != self ]; then
    [ -x "$LANE_GATE" ] || { echo "missing $LANE_GATE"; return 3; }
    command -v cargo-nextest >/dev/null || { echo "missing cargo-nextest: cargo install cargo-nextest"; return 3; }
  fi
}

# "<scenario> <finding> <closes>" per entry of expected-skips.toml.
expected_skips() {
  awk '
    function flush() { if (s != "") print s, f, c; s = f = c = "" }
    /^\[\[skip\]\]/ { flush(); next }
    $1 == "scenario" { gsub(/"/, "", $3); s = $3 }
    $1 == "finding"  { gsub(/"/, "", $3); f = $3 }
    $1 == "closes"   { gsub(/"/, "", $3); c = $3 }
    END { flush() }' "$HERE/expected-skips.toml"
}

skip_entries_resolve() {
  local s f c rc=0
  while read -r s f c; do
    [ -x "$ROOT/scenarios/$s.sh" ] || { echo "expected-skips lists $s, which is not an executable scenario"; rc=1; }
    [ -n "$f" ] && [ -n "$c" ] || { echo "expected-skips entry $s needs a finding and a closing phase"; rc=1; }
  done < <(expected_skips)
  return "$rc"
}

phase_closed() { # the entry's phase is the current phase or earlier
  [ "$tier" = phase ] && [ "${1#P}" -le "${phase#P}" ]
}

# A scenario's step budget in seconds: its `# budget: N` header line, else 60.
scenario_budget() { sed -n 's/^# budget: *\([0-9][0-9]*\).*/\1/p' "$1" | head -n 1 | grep . || echo 60; }

run_scenario() {
  local name=$1 rc=0 f c
  "$JAIL" --fabricate-gpus 2 -- "$ROOT/scenarios/$name.sh" || rc=$?
  [ "$rc" = 2 ] || return "$rc"
  read -r _ f c < <(expected_skips | awk -v n="$name" '$1 == n') || true
  if [ -z "${f:-}" ]; then echo "  unexpected SKIP: $name is not in expected-skips.toml"; return 2; fi
  if phase_closed "$c"; then echo "  $name still skips ($f) but $c has closed: the list must be empty at its exit"; return 1; fi
  echo "  expected skip: $name ($f, closes $c)"
  return 0
}

# bardo-shell's own acceptance suite on bardo-compositor (plan 20 §5.2: "bardo-shell's 19 minus listed WM-state steps"), from
# this tree, every script in its own jail on a headless bardo-compositor. Everything is
# built outside the jail first (accept.sh --build: bardo-shell, the testkit's examples, shell-services' bins,
# docket's bardo-actiond, bardo-compositor's tools from this tree). Exit 0 and 2 (SKIP) are accepted; a SKIP
# only for a script listed in shell-accept-skips.txt whose phase has not closed; any FAIL is rerun once,
# alone and one at a time (a loaded host starves the timing-sensitive scripts), and says so.
sill_accept() {
  local work="$ROOT/build/shell-accept" shell_dir="$TREE/crates/shell" log rc=0 name why closes
  local -a failed=() skipped=()
  rm -rf "$work"; mkdir -p "$work"; log="$work/accept.log"
  export CARGO_TARGET_DIR="$work/target" BARDO_COMPOSITOR_TARGET="$work/casement-target" BARDO_COMPOSITOR_TOOLS="$work/casement-tools"
  "$shell_dir/dev/accept.sh" --compositor casement --build --out "$work/run" >"$log" 2>&1 || rc=$?
  cat "$log"
  [ "$rc" != 3 ] && [ "$rc" != 4 ] || { echo "  bardo-shell acceptance could not run (exit $rc)"; return "$rc"; }
  mapfile -t failed < <(sed -n 's/^FAIL \([a-z0-9-]*\) .*/\1/p' "$log")
  if [ "${#failed[@]}" -gt 0 ]; then
    echo "  RETRY ${failed[*]} (failed under the parallel run; one at a time now)"
    rc=0
    for name in "${failed[@]}"; do
      "$shell_dir/dev/accept.sh" --compositor casement --jobs 1 --out "$work/retry-$name" "$name" >"$work/retry-$name.log" 2>&1
      case $? in
        0) echo "  retry PASS $name" ;;
        2) echo "  retry SKIP $name"; sed -n 's/^SKIP \([a-z0-9-]*\) .*/\1/p' "$work/retry-$name.log" >>"$work/retry-skips" ;;
        *) echo "  retry FAIL $name"; cat "$work/retry-$name.log"; rc=1 ;;
      esac
    done
    [ "$rc" = 0 ] || return 1
  fi
  mapfile -t skipped < <({ sed -n 's/^SKIP \([a-z0-9-]*\) .*/\1/p' "$log"; cat "$work/retry-skips" 2>/dev/null; } | sort -u)
  rc=0
  for name in "${skipped[@]}"; do
    read -r _ closes why < <(awk -v n="$name" '!/^#/ && $1 == n' "$HERE/shell-accept-skips.txt") || true
    if [ -z "${closes:-}" ]; then echo "  unexpected SKIP: $name is not in shell-accept-skips.txt"; rc=1; continue; fi
    if [ "$closes" != never ] && phase_closed "$closes"; then echo "  $name still skips but $closes has closed"; rc=1; continue; fi
    echo "  expected skip: $name (closes $closes)"
  done
  return "$rc"
}

# The lane gate script names its logs gate-target.<step>.log beside the tree: keep them under build/lane-logs/ when it
# ends, so a failure's cause survives the next run.
lane_gate() {
  local rc=0 dest="$ROOT/build/lane-logs" f
  rm -f "$(dirname "$TREE")"/gate-target.*.log
  "$LANE_GATE" "$TREE" || rc=1
  rm -rf "$dest"; mkdir -p "$dest"
  for f in "$(dirname "$TREE")"/gate-target.*.log; do [ ! -e "$f" ] || mv "$f" "$dest/"; done
  [ "$rc" = 0 ] || echo "  logs kept in $dest/"
  return "$rc"
}

run_dir_init
echo "run $RUN_ID"

step preflight preflight || true
[ "$STEP_WORST" = 0 ] || { echo "DESKTOP GATE FAIL ($label)"; exit "$STEP_WORST"; }

step "selftest (jail, 2 fabricated GPUs)" "$JAIL" --fabricate-gpus 2 -- "$ROOT/harness/selftest.sh" || true
step "selftest negative (outside the jail)" "$ROOT/harness/selftest.sh" --negative || true
step "installer (dry run and stubs)" "$HERE/installer-check.sh" || true
step "release (pack, verify, hostile releases; SKIP without minisign)" "$ROOT/release/selftest.sh" || true
step "legacy-free check (generated trees)" "$HERE/legacy-free-selftest.sh" || true
step "distro helpers (family, package table, check mode)" "$HERE/distro-check.sh" || true
step "input and caller checks (generated trees)" "$HERE/input-checks-selftest.sh" || true
step "the table of the desktop's own programs: every role has a row" "$HERE/check-callers-table.sh" || true
step "every trusted unit and the portal frontend's drop-in unset all loader variables (peephole's list)" "$HERE/check-loader-unset.sh" || true
step "no stand-in (fake feature) in what is built or shipped" "$HERE/check-no-fake.sh" || true
step "nothing shipped opens /dev/uinput to the person or a group" "$HERE/check-no-uinput-access.sh" || true
step "expected-skips resolve" skip_entries_resolve || true
# The scenarios' artefacts are built outside the jail (plan A7); BARDO_GATE_NO_BUILD=1 skips it.
if [ -z "${BARDO_GATE_NO_BUILD:-}" ]; then step "scenario artefacts (built outside the jail)" "$ROOT/harness/scenario-build.sh" || true; fi
for s in "$ROOT"/scenarios/*.sh; do
  # A scenario with an `# opt-in:` header line is run on demand only (it needs a model, a browser...): the gate skips it.
  if grep -q '^# opt-in:' "$s"; then echo "  not run by the gate (opt-in): $(basename "$s" .sh)"; continue; fi
  STEP_BUDGET=$(scenario_budget "$s") step "scenario $(basename "$s" .sh)" run_scenario "$(basename "$s" .sh)" || true
done

if [ "$tier" != self ]; then
  step "no removed-compositor traces in the tree" "$HERE/check-legacy-free.sh" "$TREE" || true
  step "lane gate of the whole tree" lane_gate || true
  # Whatever failed before, this check still runs: a phase gate names every failure.
  if [ "$tier" = phase ]; then
    step "bardo-shell acceptance on bardo-compositor (dev/accept.sh --compositor casement)" sill_accept || true
  fi
fi

if [ "$STEP_WORST" = 0 ]; then echo "DESKTOP GATE GREEN ($label)"; else echo "DESKTOP GATE FAIL ($label, exit $STEP_WORST)"; fi
steps_exit
