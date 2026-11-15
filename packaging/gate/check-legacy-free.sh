#!/usr/bin/env bash
# No trace of the removed compositor stack in the tree (owner decision 2026-10-14). For the git tree at ROOT:
#   - `git grep -i` for the three words below finds nothing, and
#   - no Cargo.lock lists a package named after the first word or a git source from its organisation.
# Every offending match is printed.
#
#   check-legacy-free.sh [ROOT]     ROOT defaults to the bardo tree this packaging folder is in
# Exit 0 clean, 1 found, 2 usage or nothing to check.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${1:-$(cd "$HERE/../.." && pwd)}"
# The words are split so that this check finds nothing in itself.
w1="c""osmic"; w2="sys""tem76"; w3="c""ctk"
PATTERN="$w1|$w2|$w3"
LOCK_PATTERN="^name = \"$w1-|pop-os/$w1"
[ -d "$ROOT_DIR" ] || { echo "no such directory: $ROOT_DIR" >&2; exit 2; }

found=0 scanned=0

scan_text() { # REPO DIR
  local r=$1 d=$2 out rc=0 line
  out=$(git -C "$d" grep --untracked -n -I -i -E "$PATTERN" -- . ':!Cargo.lock' ':!**/Cargo.lock') || rc=$?
  [ "$rc" != 128 ] || { echo "$r: git grep failed in $d" >&2; found=$((found + 1)); return; }
  [ -n "$out" ] || return 0
  while IFS= read -r line; do echo "$r:$line"; found=$((found + 1)); done <<<"$out"
}

scan_locks() { # REPO DIR
  local r=$1 d=$2 lock hits l
  while IFS= read -r lock; do
    [ -n "$lock" ] || continue
    hits=$(grep -n -E "$LOCK_PATTERN" "$d/$lock") || true
    [ -z "$hits" ] || { while IFS= read -r l; do echo "$r:$lock:$l"; found=$((found + 1)); done <<<"$hits"; }
  done < <(git -C "$d" ls-files --cached --others --exclude-standard -- 'Cargo.lock' '**/Cargo.lock')
}

if git -C "$ROOT_DIR" rev-parse --git-dir >/dev/null 2>&1; then
  scanned=1
  scan_text tree "$ROOT_DIR"
  scan_locks tree "$ROOT_DIR"
fi

[ "$scanned" -gt 0 ] || { echo "no git tree at $ROOT_DIR" >&2; exit 2; }
if [ "$found" = 0 ]; then echo "$(echo "$w1" | tr a-z A-Z)-FREE"; exit 0; fi
echo "$(echo "$w1" | tr a-z A-Z) FOUND ($found)"
exit 1
