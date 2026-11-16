#!/usr/bin/env bash
# Run bardo-shell inside the nested compositor. With no arguments: `bardo-shell run` from this checkout's
# debug build (built first); with arguments: that command, e.g. `dev/run.sh foot`.
#
# Usage: dev/run.sh                 dev/run.sh target/release/bardo-shell run
set -euo pipefail
# shellcheck source=dev/lib/compositor.sh
source "$(dirname "${BASH_SOURCE[0]}")/lib/compositor.sh"
if [ "$#" -eq 0 ]; then
  (cd "$BARDO_SHELL_REPO" && cargo build "${BARDO_SHELL_CARGO_PROFILE[@]}" -p shell-app --features debug)
  exec "$BARDO_LAYER_SHELL_HOST_DEV/run.sh" "$BARDO_SHELL_BIN" run
fi
exec "$BARDO_LAYER_SHELL_HOST_DEV/run.sh" "$@"
