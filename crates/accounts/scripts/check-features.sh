#!/usr/bin/env bash
# The feature sets the consumers of porter's crates build, each one clippy with -D warnings.
#
# The workspace gate builds `--workspace --all-features`, which unifies every feature into one
# build and so never sees the build a consumer makes on its own. A consumer that builds with one
# feature and no other once left two methods unused, and the gate passed (porter 5bdda62; fixed
# in b12886e). Each set below is checked here, alone, so the same break fails this script.
#
# The sets come from the consumers' manifests at origin/master (account-client and the other
# porter crates each consumer takes with default-features = false). The comment above each set
# names the consumer it comes from. Keep the list as data: add a set when a consumer adds one.
#
# Output: one line per build, `ok <set>` or `FAIL <set>`, with the first compiler errors of a
# failed build and the path of its log (the logs go to $CARGO_TARGET_DIR/check-features/, so
# nothing is written into the tree). The exit status is the verdict.
set -uo pipefail
cd "$(dirname "$0")/.."

# Each entry: "<set> :: <cargo arguments>". Every build is `cargo clippy <arguments>
# --lib --bins -- -D warnings`: the targets a consumer compiles. Test targets are left out on
# purpose: a test target's helpers are dead code in a default build (accountd's tests/it/ fails
# -D warnings with default features on master, and no consumer compiles it).
BUILDS=(
  # The daemon alone with its default features: `-p` alone, so no other package's features are
  # unified in.
  "accountd (default) :: -p account-daemon"

  # account-client and its two transports, in the sets their consumers name. No feature is on by
  # default: mailo takes the hosted transport alone, the sync service's syncd the hosted transport with its relays,
  # The Settings app, the shell, the actions service and the memory service the bus transport.
  "account-client :: -p account-client"
  "account-client-dbus :: -p account-client-dbus"
  "account-client-inproc (no features: mailo) :: -p account-client-inproc --no-default-features"
  "account-client-inproc (relays: the sync service syncd) :: -p account-client-inproc --features relays"

  # The other porter crates a consumer takes with default-features = false (mailo's workspace
  # block, which mail-core, mail-app and mail-runtime name), in the features each one turns on.
  "account-secrets (testing: mailo mail-core, mail-app) :: -p account-secrets --no-default-features --features testing"
  "account-secrets (oo7: mailo mail-runtime, Linux) :: -p account-secrets --no-default-features --features oo7"
  "account-secrets (keyring: mailo mail-runtime, macOS and Windows) :: -p account-secrets --no-default-features --features keyring"
  "account-discover (no features: mailo mail-core, mail-runtime) :: -p account-discover --no-default-features"
  "http-core (no features: mailo mail-core, mail-runtime) :: -p http-core --no-default-features"
  "account-oauth (no features: mailo) :: -p account-oauth --no-default-features"
  "account-oauth (io: mailo mail-runtime) :: -p account-oauth --no-default-features --features io"
  "account-proxy (io: mailo mail-runtime, its link tests) :: -p account-proxy --no-default-features --features io"
)

logs="${CARGO_TARGET_DIR:-target}/check-features"
mkdir -p "$logs"
fail=0
n=0
for build in "${BUILDS[@]}"; do
  n=$((n + 1))
  set_name=${build%% :: *}
  read -r -a args <<<"${build#* :: }"
  log="$logs/$n.log"
  if cargo clippy "${args[@]}" --lib --bins -- -D warnings >"$log" 2>&1; then
    echo "ok $set_name"
  else
    echo "FAIL $set_name"
    echo "  log: $log"
    grep -m 5 -E '^(error|warning)' "$log" | sed 's/^/  /'
    fail=1
  fi
done
exit "$fail"
