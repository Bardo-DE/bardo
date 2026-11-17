#!/usr/bin/env bash
# New global, new test file: every protocol module
# compositor-wayland/src/<p>.rs has compositor-testkit/tests/it/protocol/<p>.rs and a
# `mod <p>;` line in that directory's mod.rs.
set -euo pipefail
shopt -s nullglob
cd "$(dirname "$0")/.."
src=compositor-wayland/src
tests=compositor-testkit/tests/it/protocol
fail=0

if [ ! -d "$src" ]; then
  echo "holds: compositor-wayland is not a member yet, nothing to cover"
  exit 0
fi

# covered <module>: 0 when the module has its test file and its mod line.
covered() {
  [ -f "$tests/$1.rs" ] && [ -f "$tests/mod.rs" ] && grep -Eq "^[[:space:]]*(pub[[:space:]]+)?mod[[:space:]]+$1[[:space:]]*;" "$tests/mod.rs"
}

for file in "$src"/*.rs; do
  p=$(basename "$file" .rs)
  case "$p" in lib|states|serial|convert|client) continue ;; esac
  if covered "$p"; then echo "covered: $p"; else echo "MISSING: $p (needs $tests/$p.rs and 'mod $p;' in $tests/mod.rs)"; fail=1; fi
done

exit "$fail"
