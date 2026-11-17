#!/usr/bin/env bash
# The conformance check of cli.md section 6, for an app repository: "every app we ship can be
# driven from a command line" holds by construction. Copy this file to an app repo's scripts/ and
# run it from the app's gate. It fails when
#   - the app ships a .desktop file and no intents manifest,
#   - a manifest does not validate (or an action has no parameter schema),
#   - a menu command or shortcut in <AppName>.ui.toml names no action and is not UI-only.
#
# usage: scripts/check-intents.sh [repository dir]       (default: the repository this file is in)
# environment:
#   BARDO_ACTIONS_EVAL  a built agent-eval binary to run instead of building one
#   BARDO_ACTIONS_DIR   the actions checkout to build it from (default: the bardo tree this repository folder is in)
set -euo pipefail
repo=${1:-$(cd "$(dirname "$0")/.." && pwd)}
if [ -n "${BARDO_ACTIONS_EVAL:-}" ]; then
  exec "$BARDO_ACTIONS_EVAL" --check-app "$repo"
fi
actions=${BARDO_ACTIONS_DIR:-$repo/../..}
if [ ! -f "$actions/Cargo.toml" ]; then
  echo "check-intents: no actions checkout at $actions (set BARDO_ACTIONS_DIR, or BARDO_ACTIONS_EVAL to a built agent-eval)" >&2
  exit 2
fi
exec cargo run --quiet --manifest-path "$actions/Cargo.toml" -p action-eval --bin agent-eval -- --check-app "$repo"
