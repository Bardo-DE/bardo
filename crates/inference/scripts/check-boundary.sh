#!/usr/bin/env bash
# Crate boundaries, mechanically enforced (ARCHITECTURE.md section 1 is the table; this is its
# mechanical form). Cut from porter's script (porter e9e7cdcd) for the crates that live here.
#
# `cargo tree -i <dep>` exits 101 when the dependency is absent, which is precisely the state
# we want. Checking the exit status would therefore fail whenever the boundary holds, so we
# check for OUTPUT instead: any line naming the dependency is a leak.
set -uo pipefail
cd "$(dirname "$0")/.."

# RULES: what a crate reaches through ANY path (transitive, default features). The pure crates
# never reach a bus, a runtime, an HTTP client or a keyring. inference-client reaches none with its
# default (empty) feature set; inference-tailnet reaches tokio and hyper only through its `io`
# feature; inference-dbus is the bus vocabulary, so it reaches zbus but no client, keyring or
# settings library.
EFFECTS="zbus zvariant tokio reqwest hyper ureq oo7 keyring secret-service interprocess latchkey ui-settings"
RULES=(
  "inference-core: $EFFECTS"
  "inference-bridge: $EFFECTS"
  "inference-router: $EFFECTS inferd"
  "inference-turns: zbus zvariant ureq oo7 keyring secret-service interprocess latchkey ui-settings inferd"
  "inference-tailnet: $EFFECTS"
  "inference-client: $EFFECTS"
  "inference-probe: $EFFECTS"
  "inference-dbus: reqwest hyper ureq oo7 keyring secret-service ui-settings"
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

# ONE CLIENT: reqwest is named by account-http-reqwest (porter's crate) and by no crate of this
# workspace; inferd takes `ReqwestHttp` from it.
named=$(grep -lE '^reqwest[ .=]' crates/*/Cargo.toml || true)
if [ -n "$named" ]; then
  echo "LEAK: reqwest is named in this workspace: $named"
  fail=1
else
  echo "boundary holds: no crate here names reqwest"
fi

# The inferd library reads the environment in no function but `from_env` (the daemon's
# `Config::from_env` and the directory helpers it calls), so a program that runs the daemon inside
# itself gives it a `Config` and nothing else. The binary (main.rs) and the test-only files are
# outside the rule; `std::env::temp_dir` is a scratch name in tests, not a read of a variable.
# The awk remembers the last `fn` it saw, which is the function the next line is in.
env_reads=$(find crates/inferd/src -name '*.rs' \
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
  echo "LEAK: the inferd library reads the environment outside a from_env function"
  fail=1
else
  echo "pure: inferd reads the environment only in from_env functions"
fi

# The inference libraries (inference-router, inference-turns) read no environment variable at all: what a
# daemon or an app knows is passed in. Their test helpers are outside the rule, as above.
lib_env_reads=$(find crates/porter-router/src crates/porter-turns/src -name '*.rs' ! -name tests.rs ! -name testkit.rs ! -path '*/testkit/*' -print0 \
  | xargs -0 grep -nE 'env::(var|var_os|vars|vars_os|args|args_os|current_dir|home_dir)\b' | grep -vE '^[^:]*:[0-9]+:[[:space:]]*//' || true)
if [ -n "$lib_env_reads" ]; then
  echo "$lib_env_reads"
  echo "LEAK: an inference library reads the environment"
  fail=1
else
  echo "pure: the inference libraries read no environment variable"
fi

# DELETED HELPERS: hand-rolled code that a crate or one shared function replaced must not come
# back. Each rule names the code and where its one home is (a path prefix that is allowed to hold
# it, if any); the allowlist is otherwise empty, and every crate is covered.
ratchet() {
  local what="$1" pattern="$2" home="${3:-}" hits
  hits=$(grep -rniE --include='*.rs' -e "$pattern" crates | grep -v "^${home:-/nonexistent}" || true)
  if [ -n "$hits" ]; then
    echo "$hits"
    echo "LEAK: $what is back; use ${4:-the crate that replaced it}"
    fail=1
  else
    echo "deleted: $what is not in any crate"
  fi
}
ratchet "a hand-rolled percent encoder or decoder" 'fn percent_(en|de)code\b' "" "porter_ids::percent"
ratchet "hand-rolled civil-date arithmetic" '(days_from_civil|civil_from_days|719_468)' "" "chrono"
ratchet "a hand-written MD5 (its first round constant)" '0xd76aa478' "" "md-5"
ratchet "a hand-written scratch directory type" '^[[:space:]]*(pub(\([a-z]+\))? )?struct Scratch\b' "" "test_kit::Scratch"

# The allowed edges to the crates of ours and of the repos we pin (this workspace, porter, stoker,
# quire): each crate's DIRECT normal and build dependencies (all features), and nothing else. A
# dependency not listed is a leak; so is one the crate no longer has, so the table stays exact.
# Dev dependencies are outside it. steward depends on porter by git rev; porter never depends on
# steward.
EDGES=(
  "inferd: computer-use-action computer-use-parse computer-use-session computer-use-vendors ui-settings engine-manifest engine-probe engine-spec engine-supervisor inference-probe model-catalog model-extract model-http model-openai-compat model-provider model-replay model-wire inference-bridge account-callers account-capability account-client account-client-dbus account-core account-daemon-support account-dbus account-fs account-http account-http-reqwest account-ids inference-core account-match account-provider inference-router account-settings account-store inference-tailnet account-tailscale inference-turns account-wire speech-host-client speech-provider inference-dbus model-vision-prep"
  "inference-client: model-http model-openai-compat model-provider model-wire inference-bridge account-callers account-capability account-core account-dbus account-fs account-ids inference-core account-match inference-tailnet account-tailscale inference-turns account-wire inference-dbus"
  "inference-probe: account-capability account-http account-ids account-provider"
  "inference-bridge: model-catalog model-openai-compat model-provider account-capability account-core account-ids inference-core account-match model-vision-prep"
  "inference-core: computer-use-action model-provider account-capability account-core account-ids account-match"
  "inference-router: computer-use-action engine-probe engine-spec engine-supervisor model-catalog model-http model-openai-compat model-provider account-capability account-core account-ids inference-core account-match inference-tailnet account-tailscale speech-provider model-vision-prep"
  "inference-tailnet: account-fs account-ids account-tailscale"
  "inference-turns: computer-use-action computer-use-parse computer-use-session computer-use-vendors engine-supervisor model-extract model-http model-openai-compat model-provider inference-bridge account-capability account-core account-ids inference-core account-match inference-router model-vision-prep"
  "inference-dbus: account-core account-dbus account-dbus-macros account-ids account-match account-wire"
  "inference-rig: inference-client account-capability account-core account-ids inference-core account-match account-rig"
)
for edge in "${EDGES[@]}"; do
  crate="${edge%%:*}"
  read -r -a allowed <<<"${edge#*:}"
  found=$(cargo tree -p "$crate" --depth 1 -e normal,build --prefix none --all-features 2>/dev/null \
    | grep -E '\((/|https://github.com/Bardo-DE/(porter|stoker|quire))' | awk '{print $1}' | grep -vx "$crate" | sort -u | tr '\n' ' ')
  want=$(printf '%s\n' "${allowed[@]}" | grep . | sort -u | tr '\n' ' ')
  if [ "$found" != "$want" ]; then
    echo "EDGE: $crate depends on [${found% }], the table allows [${want% }]"
    fail=1
  else
    echo "edges hold: $crate depends on [${found% }]"
  fi
done

# Every workspace member has a row above, so a new crate cannot slip in unchecked.
for member in $(sed -n 's#^  "crates/\(.*\)",$#\1#p' Cargo.toml); do
  printf '%s\n' "${EDGES[@]}" | grep -q "^$member:" || { echo "ERROR: $member has no row in EDGES"; fail=1; }
done

# inferd's test-only knobs are never on in a default build: neither `test-proc-root`
# (`INFERD_PROC_ROOT`, which account-callers' fixtures and peephole's `fake` come with) nor
# account-secrets' `test-keys` (a file key store that keeps credentials as plain text) may be among
# the features cargo resolves for inferd without any flag.
if cargo tree -p inference-daemon --depth 0 -f '{p} {f}' 2>/dev/null | grep -q 'test-proc-root'; then
  echo "LEAK: inferd enables test-proc-root by default"
  fail=1
else
  echo "test-only: inferd's default features do not include test-proc-root"
fi
if cargo tree -p inference-daemon -e normal --prefix none -f '{p} {f}' 2>/dev/null | grep -q 'test-keys'; then
  echo "LEAK: inferd's default build enables test-keys"
  fail=1
else
  echo "test-only: inferd's default build does not include test-keys"
fi
if cargo tree -p inference-daemon -e normal --prefix none -f '{p} {f}' 2>/dev/null | grep -E '^(account-callers|peephole) ' | grep -Eq 'test-proc-root|fake'; then
  echo "LEAK: inferd's default build enables account-callers test-proc-root or peephole fake"
  fail=1
else
  echo "test-only: inferd's default build has no caller fixtures"
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
  [ -f "crates/$crate/src/lib.rs" ] && [ ! -f "crates/$crate/src/daemon.rs" ] || continue
  found=$(find "crates/$crate/src" -name '*.rs' ! -name 'tests.rs' ! -name '*_tests.rs' ! -name 'testing.rs' \
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

# The portable cores build without the desktop and reach no D-Bus.
./scripts/check-portable.sh || fail=1

exit "$fail"
