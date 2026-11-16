#!/usr/bin/env bash
# Crate boundaries, mechanically enforced (ARCHITECTURE.md section 1 is the table; this is its
# mechanical form).
#
# `cargo tree -i <dep>` exits 101 when the dependency is absent, which is precisely the state
# we want. Checking the exit status would therefore fail whenever the boundary holds, so we
# check for OUTPUT instead: any line naming the dependency is a leak.
set -uo pipefail
cd "$(dirname "$0")/.."

# RULES: what a crate reaches through ANY path (transitive, default features). The pure crates
# never reach a bus, a runtime, an HTTP client or a keyring; account-client reaches zbus only
# through its `dbus` feature; account-dbus reaches tokio only through zbus's `tokio` feature.
# account-http is the pure seam and reaches none of them; account-proxy and account-oauth reach tokio
# only through their named I/O feature (`io`). reqwest is named by one crate, account-http-reqwest
# (see ONE CLIENT below), and reached through it by the families that send directly. account-families reaches none with default features;
# each of its family features is checked on its own in FAMILY_FORBIDS below.
# account-calendar reaches zbus only through its `eds` feature.
EFFECTS="zbus zvariant tokio reqwest hyper ureq oo7 keyring secret-service interprocess latchkey ui-settings"
RULES=(
  "account-ids: $EFFECTS toml"
  "account-capability: $EFFECTS toml"
  "account-match: $EFFECTS toml"
  "account-core: $EFFECTS toml"
  "account-wire: $EFFECTS toml"
  "account-sheet: $EFFECTS toml"
  "account-fs: $EFFECTS"
  "account-provider: $EFFECTS"
  "account-secrets: $EFFECTS"
  "account-service-core: $EFFECTS"
  "account-service-accounts: $EFFECTS"
  "account-service-grants: $EFFECTS"
  "account-service: $EFFECTS"
  "account-identities: zbus zvariant reqwest hyper ureq oo7 keyring secret-service interprocess latchkey ui-settings"
  "account-spaces: zbus zvariant reqwest hyper ureq oo7 keyring secret-service interprocess latchkey ui-settings"
  "account-settings: reqwest hyper ureq oo7 keyring secret-service interprocess latchkey"
  "account-store: zbus zvariant reqwest hyper ureq oo7 keyring secret-service interprocess latchkey ui-settings"
  "account-client: $EFFECTS account-dbus account-callers account-service account-service-core account-service-accounts account-service-grants account-secrets account-provider account-fs"
  "account-client-dbus: reqwest hyper ureq oo7 keyring secret-service interprocess latchkey ui-settings account-service account-service-core account-service-accounts account-service-grants account-secrets account-provider account-fs"
  "account-client-inproc: zbus zvariant hyper ureq oo7 keyring secret-service interprocess latchkey ui-settings account-dbus"
  "account-sheets: reqwest hyper ureq oo7 keyring secret-service interprocess latchkey ui-settings"
  "account-fake: $EFFECTS"
  "base-testkit: $EFFECTS"
  "account-http: $EFFECTS"
  "account-proxy: $EFFECTS"
  "account-oauth: $EFFECTS"
  "account-discover: $EFFECTS"
  "account-daemon-support: $EFFECTS"
  "account-tailscale: $EFFECTS"
  "account-dav-proto: $EFFECTS"
  "account-calendar: $EFFECTS"
  "account-nextcloud-proto: $EFFECTS"
  "account-generic-proto: $EFFECTS"
  "account-oauth-probe-proto: $EFFECTS"
  "account-families: $EFFECTS"
  "account-family-kit: $EFFECTS"
  "account-family-nextcloud: $EFFECTS"
  "account-family-generic: zbus zvariant hyper reqwest ureq oo7 keyring secret-service interprocess latchkey ui-settings"
  "account-family-oauth: zbus zvariant ureq oo7 keyring secret-service interprocess latchkey ui-settings"
  "account-dbus: reqwest hyper ureq oo7 keyring secret-service ui-settings"
  "account-callers: reqwest hyper ureq oo7 keyring secret-service ui-settings"
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

# SANS-IO PROTOCOL CRATES: every `*-proto` folder of this domain is sans-IO: request builders return
# `http::Request<Vec<u8>>`, parsers take `&http::Response<Vec<u8>>`, and the caller sends and
# reads the clock. So none reaches the HTTP seam (account-http), a runtime (tokio) or an HTTP client
# (reqwest), and none has an `async fn`. The crates are found by name, so a new one is checked
# without a row here; the crates they may depend on are in EDGES.
for dir in *-proto; do
  crate="$(basename "$dir")"
  if ! cargo tree -p "$crate" --depth 0 >/dev/null 2>&1; then
    echo "ERROR: cargo tree cannot resolve $crate; the sans-IO rule was not checked"
    fail=1
    continue
  fi
  clean=1
  for dep in account-http tokio reqwest; do
    if cargo tree -p "$crate" -i "$dep" -e normal,build 2>/dev/null | grep -q .; then
      echo "LEAK: $crate (sans-IO) depends on $dep"
      cargo tree -p "$crate" -i "$dep" -e normal,build 2>/dev/null | head -20
      clean=0
      fail=1
    fi
  done
  if grep -rn --include='*.rs' -E '\basync[[:space:]]+fn\b' "$dir/src"; then
    echo "LEAK: $crate (sans-IO) has an async fn"
    clean=0
    fail=1
  fi
  if [ "$clean" -eq 1 ]; then
    echo "sans-IO holds: $crate reaches none of account-http tokio reqwest and has no async fn"
  fi
done

# FAMILY FEATURES: account-families with each of its family features on, one at a time, and no
# other. Its default build is checked by RULES above (it reaches none of EFFECTS). A family
# reaches only what it really uses, so each feature forbids EFFECTS minus what it uses today:
#   generic:            tokio (account-proxy's `io`, for the relay's login)
#   oauth:              tokio, hyper, reqwest (account-oauth's `io`, account-http-reqwest)
#   api_key:            hyper, reqwest, tokio (account-http-reqwest)
#   tailnet:            hyper, tokio (account-tailscale's `io`)
#   nextcloud, agent_login: none
#   set:                every family a daemon maps (what oauth, generic, api_key and tailnet
#                       reach together), with the system resolver (account-discover's `io`)
# Every feature in account-families' Cargo.toml must have a row, so a new family cannot slip in
# unchecked; the direct porter-* edges of each feature are exact too (FAMILY_EDGES).
MS_FORBIDS="zbus zvariant reqwest ureq oo7 keyring secret-service interprocess latchkey ui-settings"
DIRECT_FORBIDS="zbus zvariant ureq oo7 keyring secret-service interprocess latchkey ui-settings"
declare -A FAMILY_FORBIDS=(
  [nextcloud]="$EFFECTS"
  [generic]="zbus zvariant hyper reqwest ureq oo7 keyring secret-service interprocess latchkey ui-settings"
  [oauth]="$DIRECT_FORBIDS"
  [api_key]="$DIRECT_FORBIDS"
  [agent_login]="$EFFECTS"
  [tailnet]="$MS_FORBIDS"
  [set]="$DIRECT_FORBIDS"
)
declare -A FAMILY_EDGES=(
  [nextcloud]="account-family-kit account-family-nextcloud account-core account-capability account-sheet account-ids account-http account-provider"
  [generic]="account-family-kit account-family-generic account-core account-capability account-sheet account-ids account-http account-provider"
  [oauth]="account-family-kit account-family-oauth account-core account-capability account-sheet account-ids account-http account-provider"
  [api_key]="account-family-kit account-core account-capability account-sheet account-ids account-http account-http-reqwest account-provider"
  [agent_login]="account-family-kit account-core account-capability account-sheet account-ids account-http account-provider"
  [tailnet]="account-family-kit account-core account-capability account-sheet account-ids account-http account-provider account-tailscale"
  [set]="account-family-kit account-family-nextcloud account-family-generic account-family-oauth account-core account-capability account-sheet account-ids account-discover account-http account-http-reqwest account-provider account-tailscale"
)
family_features=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+ *=/{sub(/ *=.*/,""); if ($0!="default") print}' account-families/Cargo.toml)
for feature in $family_features; do
  if [ -z "${FAMILY_FORBIDS[$feature]+set}" ] || [ -z "${FAMILY_EDGES[$feature]+set}" ]; then
    echo "ERROR: account-families feature $feature has no row in FAMILY_FORBIDS or FAMILY_EDGES; its boundary was not checked"
    fail=1
    continue
  fi
  read -r -a forbidden <<<"${FAMILY_FORBIDS[$feature]}"
  leaked=0
  for dep in "${forbidden[@]}"; do
    if cargo tree -p account-families --no-default-features --features "$feature" -i "$dep" -e normal,build 2>/dev/null | grep -q .; then
      echo "LEAK: account-families[$feature] depends on $dep"
      cargo tree -p account-families --no-default-features --features "$feature" -i "$dep" -e normal,build 2>/dev/null | head -20
      leaked=1
      fail=1
    fi
  done
  if [ "$leaked" -eq 0 ]; then
    echo "boundary holds: account-families[$feature] reaches none of ${forbidden[*]}"
  fi
  found=$(cargo tree -p account-families --no-default-features --features "$feature" --depth 1 -e normal,build --prefix none 2>/dev/null \
    | grep -E '\((/|https://github.com/Bardo-DE/(stoker|quire))' | awk '{print $1}' | grep -vx account-families | sort -u | tr '\n' ' ')
  want=$(printf '%s\n' ${FAMILY_EDGES[$feature]} | sort -u | tr '\n' ' ')
  if [ "$found" != "$want" ]; then
    echo "EDGE: account-families[$feature] depends on [${found% }], the table allows [${want% }]"
    fail=1
  else
    echo "edges hold: account-families[$feature] depends on [${found% }]"
  fi
done

# ONE CLIENT: reqwest is named by account-http-reqwest and by no other crate of this workspace. Every
# other crate that sends over a direct connection takes `ReqwestHttp` from it.
named=$(grep -lE '^reqwest[ .=]' */Cargo.toml | grep -v '^account-http-reqwest/' || true)
if [ -n "$named" ]; then
  echo "LEAK: reqwest is named outside account-http-reqwest: $named"
  fail=1
else
  echo "boundary holds: only account-http-reqwest names reqwest"
fi

# PURE FILES: source that parses what an edge read and never reads it itself. A dependency rule
# cannot see std, so these are checked by name: no file, process, socket or environment access.
STD_EFFECTS='std::(fs|io|env|process|net|os)\b|\b(File|Command|TcpStream|UnixStream)::'
PURE_FILES=(
  "account-core/src/identity.rs"
  "account-ids/src/clock.rs"
)
for file in "${PURE_FILES[@]}"; do
  if [ ! -f "$file" ]; then
    echo "ERROR: $file is missing; its purity was not checked"
    fail=1
  elif grep -nE "$STD_EFFECTS" "$file"; then
    echo "LEAK: $file reaches std's effects"
    fail=1
  else
    echo "pure: $file reaches none of std's effects"
  fi
done

# account-core's and account-ids' source is pure: the file writer is account-fs, so no file, process or
# environment access is named anywhere in either crate (a dependency rule cannot see std's own
# modules).
for pure in account-ids account-core; do
  if grep -rnE 'std::(fs|process|env)\b' "$pure/src"; then
    echo "LEAK: $pure's source reaches std's file, process or environment access"
    fail=1
  else
    echo "pure: $pure's source names no std::fs, std::process or std::env"
  fi
done

# The daemon's library reads the environment in no function but `from_env` (the daemon's
# `Config::from_env` and the directory helpers it calls), so a program that runs a daemon inside
# itself gives it a `Config` and nothing else. The binaries (main.rs) and the test-only files are
# outside the rule; `std::env::temp_dir` is a scratch name in tests, not a read of a variable.
# The awk remembers the last `fn` it saw, which is the function the next line is in.
env_reads=$(find account-daemon/src -name '*.rs' \
  ! -name main.rs ! -name tests.rs ! -name testing.rs ! -name testkit.rs ! -path '*/tests/*' -print0 \
  | xargs -0 awk '
    FNR == 1 { current = "" }
    /^[[:space:]]*(pub(\([a-z]+\))? )?(async )?(unsafe )?fn [A-Za-z_0-9]+/ {
      match($0, /fn [A-Za-z_0-9]+/)
      current = substr($0, RSTART + 3, RLENGTH - 3)
    }
    /env::(var|var_os|vars|vars_os|args|args_os|current_dir|home_dir)\>/ \
      && current != "from_env" && $0 !~ /^[[:space:]]*\/\// {
      print FILENAME ":" FNR ": " $0
    }')
if [ -n "$env_reads" ]; then
  echo "$env_reads"
  echo "LEAK: a daemon library reads the environment outside a from_env function"
  fail=1
else
  echo "pure: accountd reads the environment only in from_env functions"
fi

# account-daemon-support reads no environment variable at all: what a daemon or an app knows is passed in
# (account-daemon-support takes the lookup). Its test helpers are outside the rule, as above.
lib_env_reads=$(find account-daemon-support/src -name '*.rs' ! -name tests.rs ! -name testkit.rs ! -path '*/testkit/*' -print0 \
  | xargs -0 grep -nE 'env::(var|var_os|vars|vars_os|args|args_os|current_dir|home_dir)\b' | grep -vE '^[^:]*:[0-9]+:[[:space:]]*//' || true)
if [ -n "$lib_env_reads" ]; then
  echo "$lib_env_reads"
  echo "LEAK: account-daemon-support reads the environment"
  fail=1
else
  echo "pure: account-daemon-support reads no environment variable"
fi

# DELETED HELPERS: hand-rolled code that a crate or one shared function replaced must not come
# back. Each rule names the code and where its one home is (a path prefix that is allowed to hold
# it, if any); the allowlist is otherwise empty, and every crate is covered.
ratchet() {
  local what="$1" pattern="$2" home="${3:-}" hits
  hits=$(grep -rniE --include='*.rs' -e "$pattern" */ ../base/base-testkit | grep -v "^${home:-/nonexistent}" || true)
  if [ -n "$hits" ]; then
    echo "$hits"
    echo "LEAK: $what is back; use ${4:-the crate that replaced it}"
    fail=1
  else
    echo "deleted: $what is not in any crate"
  fi
}
ratchet "a hand-rolled percent encoder or decoder" 'fn percent_(en|de)code\b' "" "account_ids::percent"
ratchet "hand-rolled civil-date arithmetic" '(days_from_civil|civil_from_days|719_468)' "" "chrono"
ratchet "a hand-written MD5 (its first round constant)" '0xd76aa478' "" "md-5"
ratchet "an XOAUTH2 format string" '(\\x01|\\u\{1\})auth=Bearer|auth=Bearer \{|"auth=Bearer' "account-core/src/token.rs" "account_core::token"
ratchet "a hand-written scratch directory type" '^[[:space:]]*(pub(\([a-z]+\))? )?struct Scratch\b' "../base/base-testkit/" "base_testkit::Scratch"

# The allowed edges between our own crates: each crate's DIRECT normal and build path
# dependencies (all features), and nothing else. A dependency not listed is a leak; so is one
# the crate no longer has, so the table stays exact. Dev dependencies are outside it.
EDGES=(
  "account-ids:"
  "account-capability: account-ids"
  "account-match: account-capability account-ids"
  "account-core: account-ids account-capability"
  "account-wire: account-core account-match account-capability"
  "account-sheet: account-core account-tailscale account-capability account-ids account-wire"
  "account-daemon-support:"
  "account-fs:"
  "account-keyring-chunked:"
  "account-provider: account-core account-match account-capability account-sheet account-ids"
  "account-secrets: account-keyring-chunked account-core account-capability account-ids account-fs"
  "account-service-core: account-core account-match account-capability account-sheet account-wire account-ids account-provider account-secrets"
  "account-service-accounts: account-service-core account-core account-tailscale account-match account-capability account-sheet account-wire account-ids account-provider account-secrets"
  "account-service-grants: account-service-core account-service-accounts account-core account-match account-capability account-wire account-ids account-provider account-secrets account-sheet"
  "account-service: account-service-core account-service-accounts account-service-grants account-core account-match account-capability account-wire account-ids account-provider account-secrets account-sheet"
  "account-store: account-core account-capability account-fs"
  "account-identities: account-service-core account-core account-ids account-fs account-secrets account-wire account-sheet"
  "account-spaces: account-core account-ids account-fs"
  "account-settings: ui-settings account-service-core account-capability account-core account-ids account-provider account-sheet"
  "account-dbus-macros:"
  "account-callers: account-core"
  "account-dbus: account-core account-dbus-macros account-tailscale account-wire account-ids"
  "account-sheets: account-core account-capability account-sheet account-wire account-callers account-dbus account-provider account-service-core"
  "account-client: account-core account-tailscale account-match account-capability account-wire account-ids account-sheet"
  "account-client-dbus: account-client account-core account-match account-wire account-capability account-dbus account-ids account-tailscale"
  "account-client-inproc: account-capability account-client account-core account-ids account-match account-provider account-proxy account-secrets account-service account-service-accounts account-service-core account-service-grants account-sheet account-wire"
  "account-fake: account-core account-capability account-sheet account-wire account-ids account-provider account-secrets account-service account-service-core"
  "base-testkit:"
  "account-fake-servers: account-dav-proto account-core account-ids account-discover account-fake account-provider base-testkit"
  "account-http: account-core account-ids"
  "account-http-reqwest: account-ids account-http"
  "account-http-relay: account-client account-core account-wire account-ids account-http"
  "account-proxy: account-core account-capability account-ids"
  "account-oauth: account-core account-ids account-http account-provider"
  "account-discover: account-core account-capability account-ids account-http account-provider"
  "account-tailscale: account-ids"
  "account-dav-proto:"
  "account-calendar: account-dav-proto account-http account-ids"
  "account-nextcloud-proto: account-core account-capability account-ids"
  "account-oauth-probe-proto: account-capability account-ids"
  "account-generic-proto: account-dav-proto account-core account-capability account-ids account-provider account-sheet"
  "account-families: account-family-kit account-family-nextcloud account-family-generic account-family-oauth account-core account-capability account-sheet account-ids account-discover account-http account-http-reqwest account-provider account-tailscale"
  "account-family-kit: account-core account-provider account-sheet account-http"
  "account-family-nextcloud: account-family-kit account-nextcloud-proto account-dav-proto account-core account-capability account-sheet account-provider account-http"
  "account-family-generic: account-family-kit account-generic-proto account-core account-capability account-sheet account-provider account-http account-discover account-proxy"
  "account-family-oauth: account-family-kit account-oauth-probe-proto account-core account-capability account-sheet account-ids account-provider account-http account-http-reqwest account-oauth"
  "account-daemon: ui-settings account-core account-match account-capability account-sheet account-wire account-ids account-daemon-support account-callers account-dbus account-families account-family-oauth account-fs account-identities account-provider account-proxy account-secrets account-service account-service-accounts account-service-core account-settings account-sheets account-spaces account-store account-tailscale"
  "account-rig: account-fake-secret-service account-client account-core account-capability account-wire account-ids account-callers account-fake account-fake-servers base-testkit account-client-dbus"
  "account-fake-secret-service:"
)
for edge in "${EDGES[@]}"; do
  crate="${edge%%:*}"
  read -r -a allowed <<<"${edge#*:}"
  found=$(cargo tree -p "$crate" --depth 1 -e normal,build --prefix none --all-features 2>/dev/null \
    | grep -E '\((/|https://github.com/Bardo-DE/(stoker|quire))' | awk '{print $1}' | grep -vx "$crate" | sort -u | tr '\n' ' ')
  want=$(printf '%s\n' "${allowed[@]}" | grep . | sort -u | tr '\n' ' ')
  if [ "$found" != "$want" ]; then
    echo "EDGE: $crate depends on [${found% }], the table allows [${want% }]"
    fail=1
  else
    echo "edges hold: $crate depends on [${found% }]"
  fi
done

# HIDDEN CRATES: the bus and the service are a daemon's, and a client's transport's. Each crate
# below may be depended on (normal or build edge) only by the crates named, so an app-facing
# crate (account-client, the families, the drive crates, ...) can never come to name one. The
# transport crates are the only client-side names: account-client-dbus holds the bus, and
# account-client-inproc the service. account-rig (test tooling) uses account-dbus's sync group; its
# row goes with it.
HIDDEN=(
  "account-dbus: account-daemon account-client-dbus account-sheets"
  "account-callers: account-daemon account-rig account-sheets"
  "account-service-core: account-daemon account-client-inproc account-fake account-identities account-service account-service-accounts account-service-grants account-settings account-sheets"
  "account-service-accounts: account-daemon account-client-inproc account-service account-service-grants"
  "account-service-grants: account-client-inproc account-service"
  "account-service: account-daemon account-client-inproc account-fake"
  "account-identities: account-daemon"
  "account-spaces: account-daemon"
  "account-settings: account-daemon"
  "account-store: account-daemon"
  "account-sheets: account-daemon"
)
for row in "${HIDDEN[@]}"; do
  hidden="${row%%:*}"
  read -r -a allowed <<<"${row#*:}"
  users=$(cargo tree --workspace -i "$hidden" --depth 1 -e normal,build --prefix none 2>/dev/null \
    | awk '{print $1}' | grep -vx "$hidden" | sort -u | tr '\n' ' ')
  want=$(printf '%s\n' "${allowed[@]}" | sort -u | tr '\n' ' ')
  if [ "$users" != "$want" ]; then
    echo "HIDDEN: $hidden is depended on by [${users% }], only [${want% }] may"
    fail=1
  else
    echo "hidden holds: $hidden is named by [${users% }] alone"
  fi
done

# CLIENT TESTS: a client crate builds, and is semver-checked, without building a daemon. No
# `[dev-dependencies]` section (target-specific ones too) of a account-client* crate names a daemon
# or a service crate; the tests that need accountd are accountd's (account-daemon/tests/it/client).
client_dev=""
for manifest in account-client*/Cargo.toml; do
  named=$(awk '/^\[/ { dev = ($0 ~ /dev-dependencies\]$/) } dev && /^(account-daemon|sync-daemon|sync-[a-z-]*|drive-[a-z-]*|account-service[a-z-]*)[ .=]/ { print $1 }' "$manifest")
  if [ -n "$named" ]; then
    client_dev="$client_dev$manifest: $(echo $named)"$'\n'
  fi
done
if [ -n "$client_dev" ]; then
  printf '%s' "$client_dev"
  echo "LEAK: a account-client* crate has a daemon or a service crate as a dev-dependency"
  fail=1
else
  echo "boundary holds: no account-client* crate has a daemon or a service crate as a dev-dependency"
fi

# `account_dbus::accounts` is named only where accountd's interfaces are served or called: the
# daemons, the sheet crate and the bus transport of the client.
accounts_users=$(grep -rlE --include='*.rs' 'account_dbus::(accounts|\{[^}]*accounts)' */ \
  | grep -vE '^(account-daemon|account-sheets|account-client-dbus|account-dbus)/' || true)
if [ -n "$accounts_users" ]; then
  echo "$accounts_users"
  echo "LEAK: account_dbus::accounts is named outside the daemons and the client's bus transport"
  fail=1
else
  echo "boundary holds: account_dbus::accounts is named by the daemons and the client's bus transport alone"
fi

# accountd's test-only knob (`ACCOUNTD_PROC_ROOT`, feature `test-proc-root`) is never on in a
# default build: the features cargo resolves for accountd without any flag must not name it.
if cargo tree -p account-daemon --depth 0 -f '{p} {f}' 2>/dev/null | grep -q 'test-proc-root'; then
  echo "LEAK: accountd enables test-proc-root by default"
  fail=1
else
  echo "test-only: accountd's default features do not include test-proc-root"
fi

# accountd's file key store (`ACCOUNTD_KEYS=file:`, feature `test-keys`, account-secrets'
# `FileSecrets`) keeps credentials as plain text: neither crate's default build, nor accountd's
# whole normal dependency tree, may have the feature on.
for crate in account-daemon account-secrets; do
  if cargo tree -p "$crate" -e normal --prefix none -f '{p} {f}' 2>/dev/null | grep -q 'test-keys'; then
    echo "LEAK: $crate's default build enables test-keys"
    fail=1
  else
    echo "test-only: $crate's default build does not include test-keys"
  fi
done

# account-callers' test-only caller fixtures (feature `test-proc-root`, which turns on peephole's
# `fake`) are never in a daemon's normal dependency tree.
for crate in account-daemon; do
  if cargo tree -p "$crate" -e normal --prefix none -f '{p} {f}' 2>/dev/null | grep -E '^(account-callers|caller-identity-linux) ' | grep -Eq 'test-proc-root|fake'; then
    echo "LEAK: $crate's default build enables account-callers test-proc-root or peephole fake"
    fail=1
  else
    echo "test-only: $crate's default build has no caller fixtures"
  fi
done

# THE SYNC REPO IS NOT A DEPENDENCY: porter is accounts only. syncd and everything that is sync
# (the storage replicas, the drive, the mount helper, the Graph fake, the bus interfaces of
# Sync1 and Storage1) are ferry's, in a repo of its own that pins porter by git rev. No manifest
# here names one of ferry's crates, and no dependency tree of the workspace (normal, build or dev)
# reaches one.
FERRY_CRATES='sync-daemon|sync-fakes|replica-[a-z]+|sync-[a-z-]+|drive-[a-z-]+'
named=$(grep -lE "^($FERRY_CRATES)[ .=]" */Cargo.toml || true)
reached=$(cargo tree --workspace -e normal,build,dev --prefix none 2>/dev/null \
  | awk '{print $1}' | grep -xE "$FERRY_CRATES" | sort -u | tr '\n' ' ')
if [ -n "$named" ] || [ -n "$reached" ]; then
  echo "LEAK: a manifest names [$(echo $named)] or the workspace reaches [${reached% }], which are ferry's"
  fail=1
else
  echo "boundary holds: nothing in this workspace names or reaches syncd, a storage replica or a ferry crate"
fi

# Every workspace member has a row above, so a new crate cannot slip in unchecked.
for member in $(sed -n 's#^    "crates/accounts/\(.*\)",$#\1#p' ../../Cargo.toml); do
  printf '%s\n' "${EDGES[@]}" | grep -q "^$member:" || { echo "ERROR: $member has no row in EDGES"; fail=1; }
done

# account-fake is built over account-core, account-provider, account-secrets and account-service, so none
# of them (nor anything below them) may take it, even as a dev dependency: a dev-dependency
# cycle makes the crate's own tests compile it twice. Their tests use `base-testkit`, which knows
# nothing of porter; tests that need the fakes live in account-fake's own tests.
for below in account-core account-provider account-secrets account-service-core account-service-accounts account-service-grants account-service; do
  if cargo tree -p "$below" -e normal,build,dev --prefix none 2>/dev/null | grep -qx 'account-fake v.*'; then
    echo "LEAK: $below reaches account-fake, which depends on it (a dev-dependency cycle)"
    fail=1
  else
    echo "no cycle: $below does not reach account-fake"
  fi
done

# A test-only crate is never a dependency (normal, build or dev) of another crate of ours:
# `cargo tree -i` prints the crate itself on its first line and each dependent below it.
for testonly in account-fake-servers; do
  dependents=$(cargo tree -p "$testonly" -i "$testonly" -e normal,build,dev --prefix none 2>/dev/null \
    | grep '(/' | awk '{print $1}' | grep -vx "$testonly" | tr '\n' ' ')
  if [ -n "$dependents" ]; then
    echo "LEAK: $testonly is a dependency of [${dependents% }]"
    fail=1
  else
    echo "test-only: nothing depends on $testonly"
  fi
done

# account-rig is test tooling (the rig a jailed scenario drives porter with): no other crate of
# ours names it, not even as a dev dependency, it is never published, and nothing in dist/ (the
# units, the packaging) mentions it, so it is never installed.
for manifest in */Cargo.toml; do
  [ "$manifest" = "account-rig/Cargo.toml" ] && continue
  if grep -q 'account-rig' "$manifest"; then
    echo "LEAK: $manifest names account-rig"
    fail=1
  fi
done
if ! grep -q '^publish.workspace = true' account-rig/Cargo.toml \
  || ! grep -q '^publish = false' ../../Cargo.toml; then
  echo "LEAK: account-rig is publishable"
  fail=1
fi
if grep -rq 'account-rig' dist; then
  echo "LEAK: dist/ names account-rig"
  fail=1
else
  echo "test-only: nothing depends on account-rig, and dist/ does not name it"
fi

# LIBRARY ERRORS CAN GROW: every `pub enum` whose name ends in `Error`, in a library crate, is
# `#[non_exhaustive]`, so a variant added later breaks no caller's `match`. A library crate is
# one a shipped program links: the programs are the binaries dist/ names, the crates they reach
# through normal and build dependencies are the shipped ones, and of those a daemon (a crate
# with a `daemon` module) holds its own errors for itself. Test tooling is never reached from a
# shipped program, so it is outside the rule by what it is.
shipped_programs=$(cargo metadata --format-version 1 --no-deps 2>/dev/null \
  | jq -r '.packages[] | .name as $p | .targets[] | select(.kind | index("bin")) | "\($p) \(.name)"' \
  | while read -r package program; do grep -rqF -- "$program" dist && echo "$package"; done | sort -u)
shipped=$(for package in $shipped_programs; do
  cargo tree -p "$package" -e normal,build --prefix none 2>/dev/null | grep -E '\(/' | awk '{print $1}'
done | sort -u)
grow_leaks=""
for crate in $shipped; do
  [ -f "$crate/src/lib.rs" ] && [ ! -f "$crate/src/daemon.rs" ] || continue
  found=$(find "$crate/src" -name '*.rs' ! -name 'tests.rs' ! -name '*_tests.rs' ! -name 'testing.rs' \
    ! -name 'testkit.rs' ! -path '*/tests/*' ! -path '*/testkit/*' ! -path '*/testing/*' -print0 \
    | xargs -0 awk '
      FNR == 1 { exhaustive = 1 }
      /^[[:space:]]*#\[non_exhaustive\]/ { exhaustive = 0; next }
      /^[[:space:]]*pub enum [A-Za-z0-9_]*Error([^A-Za-z0-9_]|$)/ {
        if (exhaustive) print FILENAME ":" FNR ": " $0
        exhaustive = 1
        next
      }
      /^[[:space:]]*(pub(\([a-z]+\))? )?(fn|struct|enum|impl|trait|mod|use|const|static|type|union|extern|unsafe|async)[[:space:]]/ { exhaustive = 1 }
      /^}/ { exhaustive = 1 }')
  [ -n "$found" ] && grow_leaks="$grow_leaks$found
"
done
if [ -n "$grow_leaks" ]; then
  printf '%s' "$grow_leaks"
  echo "LEAK: a public error enum of a library crate is not #[non_exhaustive]"
  fail=1
else
  echo "grows: every public *Error enum of the libraries is #[non_exhaustive] ($(echo $shipped | wc -w) shipped crates looked at)"
fi

# The portable cores build without the desktop and reach no D-Bus; the accounts-only client
# reaches no inference either (quire design/36).
./scripts/check-portable.sh || fail=1

exit "$fail"
