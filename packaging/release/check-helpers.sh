#!/usr/bin/env bash
# Checks that bardo-updater's install is complete on this computer: the three privileged programs
# (bardo-updater-apply installs a release, bardo-updater-rollback goes back, bardo-updater-health keeps the start record) in
# /usr/libexec/bardo, and the one polkit file with their three actions: installing and noting a good start need no
# password for the person at the keyboard, going back asks for the administrator's password.
#
#   release/check-helpers.sh [--root DIR] [--libexec DIR] [--policy FILE]
#
#   --root DIR      look under DIR instead of / (a staging directory); --libexec and --policy are inside it
#   --libexec DIR   where the programs are (default /usr/libexec/bardo); the policy must name exactly these paths
#   --policy FILE   the polkit file (default /usr/share/polkit-1/actions/org.bardo.updater.policy)
#
# Exit: 0 complete, 1 something missing or wrong (each reason on standard error), 2 bad usage or no python3.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
usage() { sed -n '2,/^set -euo/p' "${BASH_SOURCE[0]}" | sed '$d' | sed 's/^# \{0,1\}//'; }
case "${1:-}" in -h|--help) usage; exit 0 ;; esac
command -v python3 >/dev/null || { echo "check-helpers.sh: python3 is not installed" >&2; exit 2; }
PYTHONDONTWRITEBYTECODE=1 exec python3 -I "$HERE/check-helpers.py" "$@"
