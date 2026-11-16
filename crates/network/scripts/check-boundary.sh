#!/usr/bin/env bash
# Crate boundaries, mechanically enforced. Both crates are proxies and the types their values
# decode to: they reach zbus and nothing that talks to the network, to a TLS stack, or to a
# second D-Bus stack (libdbus through the `dbus` crate).
#
# `cargo tree -i <dep>` exits 101 when the dependency is absent, which is precisely the state we
# want, so this checks the OUTPUT of `cargo tree -p <crate>` instead: any line naming a forbidden
# dependency is a leak.
set -uo pipefail
cd "$(dirname "$0")/.."

FORBIDDEN="dbus libdbus-sys reqwest hyper ureq rustls openssl openssl-sys"
fail=0

for crate in networkmanager-proxy bluez-proxy; do
  # A crate that cargo cannot find would make every check below pass vacuously.
  if ! cargo tree -p "$crate" --depth 0 >/dev/null 2>&1; then
    echo "ERROR: cargo tree cannot resolve $crate; the boundary was not checked"
    fail=1
    continue
  fi
  tree=$(cargo tree -p "$crate" --all-features --edges normal --prefix none --format '{p}' 2>/dev/null)
  for dep in $FORBIDDEN; do
    if grep -qE "^$dep v" <<<"$tree"; then
      echo "ERROR: $crate reaches $dep"
      fail=1
    fi
  done
done

[ "$fail" -eq 0 ] && echo "boundary OK"
exit "$fail"
