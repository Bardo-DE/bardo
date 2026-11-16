#!/usr/bin/env bash
# Screenshot. `dev/shot.sh OUT.png` shoots the nested compositor with grim (or the host with
# spectacle): layer-shell-host's scripts/dev/shot.sh. `dev/shot.sh --surface NAME OUT.png` renders one
# surface offscreen through the daemon (`bardo-shell debug capture`), compositor-independent.
set -euo pipefail
# shellcheck source=dev/lib/compositor.sh
source "$(dirname "${BASH_SOURCE[0]}")/lib/compositor.sh"
if [ "${1:-}" = "--surface" ]; then
  [ "$#" -eq 3 ] || { echo "usage: dev/shot.sh --surface NAME OUT.png" >&2; exit 2; }
  exec "$BARDO_LAYER_SHELL_HOST_DEV/run.sh" "$BARDO_SHELL_BIN" debug capture "$2" "$(realpath -m "$3")"
fi
exec "$BARDO_LAYER_SHELL_HOST_DEV/shot.sh" "$@"
