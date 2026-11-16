# Sourceable: provides `require_jail` (20-FREEZE-PLAN §6.1, C40) by sourcing the jail harness's harness/lib/jail.sh,
# which is packaging/ of this tree (or $CASEMENT_DESKTOP, a folder holding harness/lib/jail.sh). Missing: prints the fix and exits 3.
_casement_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../casement" && pwd)
_casement_desktop=""
for _d in "${CASEMENT_DESKTOP:-}" "$_casement_root/../../packaging"; do
  if [ -n "$_d" ] && [ -f "$_d/harness/lib/jail.sh" ]; then _casement_desktop=$_d; break; fi
done
if [ -z "$_casement_desktop" ]; then
  echo "dev/lib/jail.sh: the harness (packaging/harness/lib/jail.sh of the bardo tree) was not found" >&2
  echo "fix: run this from a bardo tree, or set CASEMENT_DESKTOP to a folder holding harness/lib/jail.sh" >&2
  exit 3
fi
# shellcheck source=/dev/null
source "$_casement_desktop/harness/lib/jail.sh"
unset _d
