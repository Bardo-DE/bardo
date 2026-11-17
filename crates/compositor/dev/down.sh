#!/usr/bin/env bash
# Stops what dev/up.sh started, by the pids in the run's env file (never by pattern), and removes
# the run's sockets. With no argument it stops the newest run in this jail; with ENV_FILE, that
# run. Calls require_jail first.
#
#   dev/down.sh [ENV_FILE]
#
# Owner: P0-T-dev. Exit codes: 0 stopped (or nothing running), 2 outside the jail.
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
# shellcheck source=dev/lib/compositor.sh
source "$root/dev/lib/compositor.sh"
require_jail
if [ $# -ge 1 ]; then BARDO_COMPOSITOR_STATE=$(dirname "$1"); export BARDO_COMPOSITOR_STATE; fi
stop_compositor
echo "compositor is down"
