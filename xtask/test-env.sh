#!/bin/sh
# Runs one test binary in a throwaway home: `cargo xtask gate` sets this as the target runner for the
# test step, so every test process starts here (`test-env.sh <test binary> <args>`).
#
# A test that forgets to isolate itself must not be able to read or write the person's files or reach
# the running desktop. So: HOME and every XDG folder point into a fresh folder under
# $BARDO_TEST_ROOT (made by xtask, removed after the step), the runtime folder is private (0700),
# and the session and system buses point at nothing. Tests start their own private bus and
# compositor; they never need the real ones.
set -eu
root=${BARDO_TEST_ROOT:?BARDO_TEST_ROOT is not set: run tests through cargo xtask gate}
dir=$(mktemp -d "$root/t.XXXXXXXX")
mkdir -p "$dir/home" "$dir/config" "$dir/data" "$dir/state" "$dir/cache" "$dir/run"
chmod 0700 "$dir/run"
export HOME="$dir/home"
export XDG_CONFIG_HOME="$dir/config"
export XDG_DATA_HOME="$dir/data"
export XDG_STATE_HOME="$dir/state"
export XDG_CACHE_HOME="$dir/cache"
export XDG_RUNTIME_DIR="$dir/run"
export TMPDIR="$dir"
export DBUS_SESSION_BUS_ADDRESS="unix:path=$dir/run/no-session-bus"
export DBUS_SYSTEM_BUS_ADDRESS="unix:path=$dir/run/no-system-bus"
unset DISPLAY WAYLAND_DISPLAY WAYLAND_SOCKET SSH_AUTH_SOCK GNUPGHOME
exec "$@"
