#!/bin/sh
# The edges of the dependency graph:
#   input-broker-wire stays pure: no bus, async runtime, caller identity, socket or device library.
#   input-broker-client is what palmrest and keycap link, so it must not drag in the bus, an async
#   runtime, the caller-identity crates or the daemon's libraries.
#   turnstile is a root service: it may read /proc through peephole, but must never link the bus or an
#   async runtime.
set -eu
cd "$(dirname "$0")/.."

# reaches CRATE PATTERN: the dependencies of CRATE (normal ones, build scripts excluded) whose name
# matches the extended regular expression PATTERN. A failing `cargo tree` stops the script instead of
# reading as "nothing found".
reaches() {
    tree=$(cargo tree -p "$1" -e normal --prefix none)
    echo "$tree" | grep -E "^($2) " || true
}

bad=$(reaches input-broker-wire 'zbus|tokio|async-std|smol|peephole[a-z-]*|caller-identity-[a-z]*|rustix|libc|input-linux[a-z-]*|listenfd|nix')
[ -z "$bad" ] || { echo "check-boundary: input-broker-wire reaches: $bad"; exit 1; }

bad=$(reaches input-broker-client 'zbus|tokio|async-std|smol|peephole[a-z-]*|caller-identity-[a-z]*|input-linux[a-z-]*|listenfd|nix')
[ -z "$bad" ] || { echo "check-boundary: input-broker-client reaches: $bad"; exit 1; }

bad=$(reaches input-broker-daemon 'zbus|tokio|async-std|smol|caller-identity-dbus')
[ -z "$bad" ] || { echo "check-boundary: input-broker-daemon reaches: $bad"; exit 1; }

echo "check-boundary: ok"
