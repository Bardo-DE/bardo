#!/usr/bin/env bash
# Regenerates fixtures/schemas from the real programs' own `--write-schema`, into a scratch
# directory first. Builds each program in the bardo workspace (../../Cargo.toml).
#
#   bardo-shell        bardo-shell --write-schema <dir>
#   mouse-gestures  bardo-mouse-gestures --write-schema <dir>
#   Bardo       ui-settings' `write_schema` example (the UI kit has no app binary)
#   inferd      ../inference/dist/inference.settings.toml (written by hand, no --write-schema yet)
#   The actions service      ../actions/dist/settings/actions.settings.toml
#   The memory service     ../memory/memory-settings/schema/memory.settings.toml
#   cua         ../computer-use/dist/settings/computer-use.settings.toml
#
# Usage: scripts/gen-fixtures.sh
set -euo pipefail
cd "$(dirname "$0")/.."

scratch="$(mktemp -d "${TMPDIR:-/tmp}/bardo-settings-fixtures.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT

# A private home: the programs read no real config while writing a schema.
private() { env -i PATH="$PATH" HOME="$scratch/home" XDG_CONFIG_HOME="$scratch/config" XDG_DATA_HOME="$scratch/data" "$@"; }

cargo build --quiet --manifest-path ../../Cargo.toml -p shell-app
cargo build --quiet --manifest-path ../../Cargo.toml -p mouse-gestures-daemon
cargo build --quiet --manifest-path ../../Cargo.toml -p ui-settings --example write_schema

private "$(cargo metadata --format-version 1 --no-deps --manifest-path ../../Cargo.toml | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')/debug/bardo-shell" --write-schema "$scratch/out"
private "$(cargo metadata --format-version 1 --no-deps --manifest-path ../../Cargo.toml | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')/debug/bardo-mouse-gestures" --write-schema "$scratch/out"
private "$(cargo metadata --format-version 1 --no-deps --manifest-path ../../Cargo.toml | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')/debug/examples/write_schema" --write-schema "$scratch/out"

mkdir -p fixtures/schemas
cp "$scratch"/out/*.settings.toml fixtures/schemas/
cp ../inference/dist/inference.settings.toml fixtures/schemas/
cp ../actions/dist/settings/actions.settings.toml ../memory/memory-settings/schema/memory.settings.toml \
  ../computer-use/dist/settings/computer-use.settings.toml fixtures/schemas/
ls -l fixtures/schemas
