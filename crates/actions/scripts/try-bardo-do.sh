#!/usr/bin/env bash
# Try `bardo-do` by hand against a real bardo-actiond, on a private bus that is not your session's:
# a scratch dbus-daemon, a scratch HOME and XDG directories, bardo-actiond, a fake mail app
# (org.bardo.Mail) and a fake shell sheet that asks on THIS terminal. Nothing of your real
# session bus or ~/.config is named.
#
#   scripts/try-bardo-do.sh          # starts it all; prints what to run in a second terminal
#
# In the second terminal (the script prints the exact `source` line):
#   bardo-do apps
#   bardo-do mail --list
#   bardo-do mail thread.read t2                 # a read: runs, no sheet
#   bardo-do mail thread.archive t2 --dry-run    # the preview, no sheet
#   bardo-do mail thread.archive t2              # a write: the sheet appears HERE; answer y, t or n
#   bardo-do undo --last
#   bardo-do memory forget memory.fact:f1        # hidden: exit 3, no sheet
# Ctrl-C here stops everything.
set -euo pipefail
cd "$(dirname "$0")/.."

cargo build -p action-daemon -p action-cli --bins --examples
bin="$(cargo metadata --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')/debug"

world="$(mktemp -d)"
pids=()
cleanup() {
  for pid in "${pids[@]}"; do kill "$pid" 2>/dev/null || true; done
  rm -rf "$world"
}
trap cleanup EXIT

mkdir -p "$world/data/bardo/intents" "$world/config" "$world/bin"
cp action-fake/fixtures/manifests/org.bardo.Mail.intents.toml "$world/data/bardo/intents/org.bardo.Mail.toml"
cp manifests/org.bardo.Memory.toml "$world/data/bardo/intents/org.bardo.Memory.toml"
cat > "$world/bus.conf" <<CONF
<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <type>session</type>
  <listen>unix:path=$world/bus.sock</listen>
  <auth>EXTERNAL</auth>
  <policy context="default">
    <allow send_destination="*"/>
    <allow receive_sender="*"/>
    <allow own="*"/>
  </policy>
</busconfig>
CONF

dbus-daemon --config-file="$world/bus.conf" --nofork --print-address=1 > "$world/address" &
pids+=($!)
for _ in $(seq 50); do [ -s "$world/address" ] && break; sleep 0.1; done
bus="$(head -n1 "$world/address")"

# What every process of this little world sees; nothing else is passed on.
scratch=(env -i HOME="$world" PATH="$PATH" XDG_RUNTIME_DIR="$world" XDG_DATA_HOME="$world/data"
  XDG_DATA_DIRS="$world/none" XDG_CONFIG_HOME="$world/config" XDG_CONFIG_DIRS="$world/none"
  DBUS_SESSION_BUS_ADDRESS="$bus" DBUS_SYSTEM_BUS_ADDRESS="$bus")

"${scratch[@]}" "$bin/bardo-actiond" &
pids+=($!)
ln -s "$bin/bardo-do" "$world/bin/bardo-do"
cat > "$world/env" <<ENV
export DBUS_SESSION_BUS_ADDRESS='$bus'
export PATH='$world/bin':\$PATH
ENV

echo "In a second terminal:  source $world/env"
echo
"${scratch[@]}" "$bin/examples/try_apps"
