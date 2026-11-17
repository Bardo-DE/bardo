#!/usr/bin/env bash
# The portable core (Bardo design/36 section 1, rule 3 and section 4): everything except the
# desktop itself builds without the desktop. Each core below must `cargo check` with the
# features named, and its normal dependency tree must reach no D-Bus (zbus, zvariant). The
# account-client build an accounts-only consumer uses (no features) must also reach no inference:
# no stoker.
#
# Called from check-boundary.sh, which the lane gate runs. Output says each check and each
# tree; the exit status is the verdict.
set -uo pipefail
cd "$(dirname "$0")/.."

fail=0

# What no portable core reaches.
BUS='zbus|zvariant'
# What the accounts-only builds also never reach: inference, and stoker through it (computer-use-action is
# stoker's crate, by sibling path).
INFER='computer-use-action'

# check <label> <crate> <forbidden regex> [cargo feature flags...]
check() {
  local label=$1 crate=$2 forbidden=$3
  shift 3
  if cargo check -p "$crate" "$@" >/dev/null 2>&1; then
    echo "check ok: $label"
  else
    echo "FAIL check: $label does not build"
    fail=1
  fi
  local tree
  if ! tree=$(cargo tree -p "$crate" "$@" -e normal --prefix none 2>/dev/null); then
    echo "FAIL tree: cargo tree cannot resolve $label"
    fail=1
    return
  fi
  local leaked
  # `name vX.Y.Z` is the first two fields; a path dependency also names its directory.
  leaked=$(awk '{print $1}' <<<"$tree" | grep -xE "$forbidden" | sort -u | tr '\n' ' ')
  if grep -q '/stoker/' <<<"$tree" && [[ "$forbidden" == *computer-use-action* ]]; then
    leaked="$leaked(a stoker path) "
  fi
  if [ -n "$leaked" ]; then
    echo "LEAK: $label reaches ${leaked% }"
    fail=1
  else
    echo "tree ok: $label reaches none of ${forbidden//|/ }"
  fi
}

check "account-client" account-client "$BUS|$INFER"
check "account-client-inproc" account-client-inproc "$BUS|$INFER"
check "account-ids" account-ids "$BUS"
check "account-core" account-core "$BUS"
check "account-provider" account-provider "$BUS"
check "account-secrets --no-default-features" account-secrets "$BUS" --no-default-features
check "account-oauth --no-default-features" account-oauth "$BUS" --no-default-features
check "account-families --no-default-features" account-families "$BUS" --no-default-features

# The client API is the one an app on macOS or Windows links, so it must compile there, not only
# on a host that happens to have unix descriptors: the Linux-only code (a memfd, a bus) is in
# account-client-dbus. Every target of these that rustup has installed is checked; a target that is
# not installed is said, and skipped (this script installs nothing).
CROSS='x86_64-pc-windows-msvc x86_64-pc-windows-gnu aarch64-apple-darwin x86_64-apple-darwin'
installed=$(rustup target list --installed 2>/dev/null)
for target in $CROSS; do
  if ! grep -qx "$target" <<<"$installed"; then
    echo "skip cross: $target is not installed"
    continue
  fi
  if cargo check --target "$target" -p account-client -p account-sheet -p account-wire >/dev/null 2>&1; then
    echo "check ok: account-client account-sheet account-wire on $target"
  else
    echo "FAIL check: account-client account-sheet account-wire do not build on $target"
    fail=1
  fi
done

exit "$fail"
