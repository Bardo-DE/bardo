#!/usr/bin/env bash
# Rewrites the generated key tables of design/22-SETTINGS.md from the schemas.
#
#   scripts/gen-doc-keys.sh [--schema <app>.settings.toml]...
#
# A table sits between `<!-- keys:begin <domain> -->` and `<!-- keys:end <domain> -->`; the UI kit's own
# schema (appearance, icons, region) is always loaded, a program's is added with --schema (write it
# with its `--write-schema <dir>`). A hand tool, not part of the gate: the `doc_tables` test in
# ui-settings fails when the doc differs from the schema, and this script is the fix.
set -euo pipefail
cd "$(dirname "$0")/.."
exec cargo run --quiet -p appearance-settings --example doc_tables -- "$@" design/22-SETTINGS.md
