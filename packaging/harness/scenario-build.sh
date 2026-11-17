#!/usr/bin/env bash
# Builds, OUTSIDE the jail, every artefact the scenarios use (plan A7: nothing inside the jail calls
# cargo or the network), from the bardo tree this folder is in, into this folder's git-ignored build/. Every build is a
# `cargo build -p ...` run at the tree's root with its own target directory under build/ (test features differ from a
# release's, so they never share one).
#
#   build/tools/bin/{bardo-compositor,bardo-compositorctl,compositor-probe,compositor-shell-exec,fake-logind} bardo-compositor's dev/bootstrap.sh --build-only
#   build/tools/bin/foot                                                 the host's (install foot from your distro), else, on Fedora only, its rpm (dnf download, unpacked)
#   build/shell-target/debug/bardo-shell                                         bardo-shell with --features debug (the fake DDC)
#   build/portal-target/debug/bardo-portal                                  bardo-portal (the portal backend), for portal-bardo; needs libpipewire-devel and clang
#   build/shell-host-target/debug/examples/demo_*                        layer-shell-host's real-input demos
#   build/detent-target/debug/bardo-settings                                     bardo-settings with ui-blitz/debug-probe, for the accounts-* and bardo-settings-on-compositor-at-1.5 scenarios
#   build/shell-probe-target/debug/bardo-shell                                   bardo-shell with --features debug,ui-blitz/debug-probe (the accounts-* scenarios; the probe is linked but writes nothing for bardo-shell's surfaces)
#   build/account-target/debug/bardo-accountd                                   the accounts service's bardo-accountd, a test-proc-root build
#   build/sync-target/debug/bardo-syncd                                       the sync service's bardo-syncd, a test-proc-root build (the sync daemon left the accounts service)
#   build/sync-target/debug/sync-rig-{servers,client}                  the sync service's test rig: the OAuth issuer + Graph fake, an app client for bardo-syncd
#   build/inference-target/debug/bardo-inferd                                    the inference service's bardo-inferd, a test-proc-root build
#   build/inference-target/debug/inference-rig-client                        the inference service's rig: an app opening an bardo-inferd session
#   build/account-target/debug/account-rig-{servers,client,secrets}        the accounts service's test rig: fakes, an app client, a fake Secret Service
#
#   scenario-build.sh --accounts   only what the accounts-* scenarios need (bardo-compositor, bardo-settings, bardo-shell with the probe, the accounts service and its rig, the sync service and its rig, the inference service)
#   scenario-build.sh --w2         only what scenarios/w2-excalidraw.sh needs (bardo-compositor, bardo-shell, and the two below):
#   build/cua-target/debug/bardo-computer-use                                          bardo-computer-use (the screen helper) with --features fixture-callers: a plain build identifies nobody
#   build/actions-target/debug/{agent-bringup,accept-*}                  action-accept's test builds (the assistant's services on a fake /proc, bardo-do)
#
# Nothing is written into the tree.
# Exit codes: 0 built, 2 run inside a jail, 3 a host tool is missing or a build failed.
set -euo pipefail
only_accounts=0 only_w2=0
case "${1:-}" in
  "") ;;
  --accounts) only_accounts=1 ;;
  --w2) only_w2=1 ;;
  *) echo "usage: scenario-build.sh [--accounts|--w2]" >&2; exit 64 ;;
esac
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=lib/all.sh
source "$ROOT/harness/lib/all.sh"
# shellcheck source=../install/distro.sh
source "$ROOT/install/distro.sh"
# shellcheck source=../install/packages.sh
source "$ROOT/install/packages.sh"
if [ -z "$(jail_violations)" ]; then
  echo "scenario-build: this builds (cargo, git, the network): run it outside the jail" >&2
  exit 2
fi
source "$ROOT/harness/lib/scenario.sh"
mkdir -p "$SCEN_BUILD"
log="$SCEN_BUILD/build.log"
: >"$log"
fail() { echo "scenario-build: $1 (see $log)" >&2; exit 3; }
# stage NAME CMD...: runs CMD with its output in the log.
stage() { local name=$1; shift; echo "scenario-build: $name"; "$@" >>"$log" 2>&1 || fail "$name failed"; }

# built_at TARGET: notes in build/TARGET/BUILT_AT the tree's HEAD it was built from ("bardo <sha>"), so a scenario can
# refuse a build older than the tree it is meant to test (scenario_fresh in harness/lib/scenario.sh).
built_at() {
  local target=$SCEN_BUILD/$1
  mkdir -p "$target"
  printf 'bardo %s\n' "$(git -C "$SCEN_TREE" rev-parse HEAD)" >"$target/BUILT_AT"
}

build_compositor() {
  (cd "$SCEN_SIB/compositor" && CARGO_TARGET_DIR="$SCEN_BUILD/compositor-target" BARDO_COMPOSITOR_TOOLS="$SCEN_BUILD/tools" \
    dev/bootstrap.sh --build-only) && built_at compositor-target
}

fetch_foot() {
  local bin="$SCEN_BUILD/tools/bin/foot" dir="$SCEN_BUILD/foot-root"
  [ ! -x "$bin" ] || return 0
  if command -v foot >/dev/null; then ln -sf "$(command -v foot)" "$bin"; return 0; fi
  # Only the dnf family can fetch the rpm and unpack it; every other family installs foot from its archive.
  [ "$(distro_family)" = fedora ] || return 1
  command -v dnf >/dev/null && command -v rpm2cpio >/dev/null && command -v cpio >/dev/null || return 1
  rm -rf "$dir"; mkdir -p "$dir/rpms"
  (cd "$dir/rpms" && timeout 300 dnf download --arch x86_64 foot fcft)
  local rpm
  for rpm in "$dir"/rpms/*.rpm; do (cd "$dir" && rpm2cpio "$rpm" | cpio -idm 2>/dev/null); done
  [ -x "$dir/usr/bin/foot" ] || return 1
  printf '#!/usr/bin/env bash\nexport LD_LIBRARY_PATH="%s/usr/lib64${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"\nexec "%s/usr/bin/foot" "$@"\n' \
    "$dir" "$dir" >"$bin"
  chmod +x "$bin"
}

# bardo-settings is built with Bardo's ui-blitz `debug-probe` feature (a feature of a direct dependency, so bardo-settings's own manifest
# is untouched): with BARDO_DEBUG_PROBE=<dir> set it lists its controls by role and name (harness/lib/accounts.sh,
# detent_press). Never a release build.
build_detent() {
  (cd "$SCEN_TREE" && CARGO_TARGET_DIR="$SCEN_BUILD/detent-target" cargo build -p settings-app --features ui-blitz/debug-probe,dev-callers)
}

build_shell() {
  (cd "$SCEN_TREE" && CARGO_TARGET_DIR="$SCEN_BUILD/shell-target" cargo build -p shell-app --features debug) &&
    built_at shell-target
}

# bardo-portal, --locked (the tree's Cargo.lock is never changed).
build_portal() {
  (cd "$SCEN_TREE" && CARGO_TARGET_DIR="$SCEN_BUILD/portal-target" cargo build -p portal-daemon --features debug --locked)
}

# The demos layer-shell-host's accept scripts run, built the way those scripts build them (two cargo
# invocations: the default features, then --features debug-inject, whose binaries the second set of
# scripts need). The scripts look in $CARGO_TARGET_DIR/debug/examples, which the scenario sets to this build dir.
build_shell_host() {
  local target="$SCEN_BUILD/shell-host-target"
  mkdir -p "$target"
  (cd "$SCEN_TREE" && CARGO_TARGET_DIR="$target" cargo build -p layer-shell-host \
    --example demo_hover --example demo_scroll --example demo_ime --example demo_swipe \
    --example demo_switcher --example demo_drag --example demo_lock)
  (cd "$SCEN_TREE" && CARGO_TARGET_DIR="$target" cargo build -p layer-shell-host --features debug-inject \
    --example demo_popup_keys --example demo_window --example probe_toplevel)
}

# The accounts service's bardo-accountd and test rig (account-rig). Test builds: `test-proc-root` makes each daemon read caller identities from
# a fixture /proc tree (BARDO_ACCOUNTD_PROC_ROOT; the sync service's bardo-syncd, below, reads BARDO_SYNCD_PROC_ROOT and the inference service's bardo-inferd BARDO_INFERD_PROC_ROOT),
# because nothing in the jail runs in a systemd scope. A release build ignores the variables; the features are never default.
build_accounts() {
  (cd "$SCEN_TREE" && CARGO_TARGET_DIR="$SCEN_BUILD/account-target" cargo build -p account-daemon --features test-proc-root) || return 1
  (cd "$SCEN_TREE" && CARGO_TARGET_DIR="$SCEN_BUILD/account-target" cargo build -p account-rig)
}

# The sync service's bardo-syncd and test rig. bardo-syncd is a test build like the accounts service's bardo-accountd: `test-proc-root` makes it read caller identities from a
# fixture /proc tree (BARDO_SYNCD_PROC_ROOT). One invocation per package: `-p a -p b --features a/f,b/f` once left a daemon without its
# feature (the binary then ignores the variable and no caller is known). sync-rig gives sync-rig-servers (the issuer and Graph)
# and sync-rig-client.
build_sync() {
  (cd "$SCEN_TREE" && CARGO_TARGET_DIR="$SCEN_BUILD/sync-target" cargo build -p sync-daemon --features test-proc-root \
    && CARGO_TARGET_DIR="$SCEN_BUILD/sync-target" cargo build -p sync-rig)
}

# The inference service's bardo-inferd. A test build like the accounts service's daemons: `test-proc-root` makes it read caller identities from a fixture /proc tree
# (BARDO_INFERD_PROC_ROOT).
build_inference() {
  (cd "$SCEN_TREE" && CARGO_TARGET_DIR="$SCEN_BUILD/inference-target" cargo build -p inference-daemon --features test-proc-root \
    && CARGO_TARGET_DIR="$SCEN_BUILD/inference-target" cargo build -p inference-rig --bin inference-rig-client)
}

# bardo-computer-use, the screen helper. The fixture-callers feature makes it read caller identities from a fake
# /proc (BARDO_COMPUTER_USE_TEST_PROC_ROOT): nothing in the jail runs in a systemd scope. A test build, never shipped.
build_cuad() {
  (cd "$SCEN_TREE" && CARGO_TARGET_DIR="$SCEN_BUILD/cua-target" nice cargo build -p computer-use-daemon --features fixture-callers) &&
    built_at cua-target
}

# The assistant's services as action-accept builds (fixture identities, scripted stand-ins), agent-bringup and bardo-do's
# test twin.
build_actions() {
  (cd "$SCEN_TREE" && CARGO_TARGET_DIR="$SCEN_BUILD/actions-target" nice cargo build -p action-accept --bins) &&
    built_at actions-target
}

# bardo-shell with the debug probe: the accounts-* scenarios read the accounts sheet through it.
build_shell_probe() {
  (cd "$SCEN_TREE" && CARGO_TARGET_DIR="$SCEN_BUILD/shell-probe-target" cargo build -p shell-app --features debug,ui-blitz/debug-probe)
}

# foot is the scenarios' terminal: say how to get it before the long builds, not after.
if [ "$only_accounts" = 0 ] && [ "$only_w2" = 0 ] && [ ! -x "$SCEN_BUILD/tools/bin/foot" ] && ! command -v foot >/dev/null && [ "$(distro_family)" != fedora ]; then
  echo "scenario-build: foot (a terminal the scenarios start) is not installed: $(install_hint foot | tr '\n' ' ')" >&2
  exit 3
fi

stage "bardo-compositor (bootstrap --build-only)" build_compositor
if [ "$only_w2" = 1 ]; then
  stage "bardo-shell (--features debug)" build_shell
  stage "bardo-computer-use (--features fixture-callers)" build_cuad
  stage "action-accept (the assistant's services, test builds)" build_actions
  echo "scenario-build: done for W2 ($SCEN_BUILD)"
  exit 0
fi
if [ "$only_accounts" = 0 ]; then
  stage "foot" fetch_foot
  stage "bardo-shell (--features debug)" build_shell
  stage "layer-shell-host demos" build_shell_host
  stage "bardo-portal" build_portal
fi
stage "bardo-settings (with the debug probe)" build_detent
stage "bardo-shell with the debug probe" build_shell_probe
stage "accounts: bardo-accountd (test-proc-root) and account-rig" build_accounts
stage "sync: bardo-syncd (test-proc-root) and sync-rig" build_sync
stage "inference: bardo-inferd (test-proc-root) and inference-rig-client" build_inference
echo "scenario-build: done ($SCEN_BUILD)"
