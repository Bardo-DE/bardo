#!/usr/bin/env bash
# Hand tool: every backticked path in the tracked markdown files must name a file or directory that
# exists. Run from anywhere; it works from the repo root over every tracked .md file.
# A token counts as a path when it ends in .rs/.css/.md/.sh or starts with `crates/`, `docs/`,
# `dev/`, `packaging/`, `scripts/` or `examples/`. It resolves when it exists relative to the repo
# root or to the doc's own directory, or when it is the tail of a tracked file's path (`root/ds.rs`,
# `ui-style/tokens/colour.rs` for `crates/ui/ui-style/src/tokens/colour.rs`). Tokens with a glob, a
# placeholder, a space or a leading `~` (files outside the repo) are skipped, as is anything between
# `<!-- paths: skip -->` and `<!-- paths: end -->` (a table of old locations that no longer exist).
# Prints one `doc: token` line per miss; exits 1 on any. Run by hand after moving or renaming files;
# it is not part of the gate.
set -uo pipefail
cd "$(dirname "$0")/.."

index=$(mktemp)
trap 'rm -f "$index"' EXIT
git ls-files >"$index"

status=0
while IFS= read -r doc; do
  dir=$(dirname "$doc")
  awk '/<!-- paths: skip -->/{skip=1} !skip{print} /<!-- paths: end -->/{skip=0}' "$doc" | grep -o '`[^` ]*`' | tr -d '`' | sort -u | while IFS= read -r token; do
    case "$token" in
      *'*'*|*'<'*|*'{'*|*'$'*|*'('*|*'|'*|*'='*|*','*|http*|*'::'*|'~'*|.*) continue ;;
    esac
    path=${token%%#*}
    path=${path%:[0-9]*}
    case "$path" in
      *.rs|*.css|*.md|*.sh|crates/*|docs/*|dev/*|packaging/*|scripts/*|examples/*) ;;
      *) continue ;;
    esac
    [ -z "$path" ] && continue
    trimmed=${path%/}
    [ -e "$trimmed" ] && continue
    [ -e "$dir/$trimmed" ] && continue
    stripped=${trimmed#./}
    stripped=${stripped#../}
    crate=${stripped%%/*}
    if [ "$crate" != "$stripped" ]; then
      grep -qxF -- "crates/$crate/src/${stripped#*/}" "$index" && continue
    fi
    grep -qF -- "$stripped" "$index" && grep -q -- "\(^\|/\)$(printf '%s' "$stripped" | sed 's/[.[\*^$]/\\&/g')\(/\|$\)" "$index" && continue
    echo "$doc: $token"
  done
done < <(git ls-files '*.md' | grep -v '^examples/.*/target/') | tee "${TMPDIR:-/tmp}/doc-paths.$$" 
n=$(wc -l <"${TMPDIR:-/tmp}/doc-paths.$$"); rm -f "${TMPDIR:-/tmp}/doc-paths.$$"
[ "$n" -eq 0 ] || { echo "$n missing path(s)" >&2; exit 1; }
