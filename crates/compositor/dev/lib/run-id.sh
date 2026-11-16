#!/usr/bin/env bash
# Prints a sortable run id and creates dev/out/<run>/ under the repo root.
set -euo pipefail
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../casement" && pwd)
run="$(date +%Y%m%dT%H%M%S)-$$"
mkdir -p "$root/dev/out/$run"
echo "$run"
