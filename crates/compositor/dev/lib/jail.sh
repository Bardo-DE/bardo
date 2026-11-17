# Sourceable: provides `require_jail` (20-FREEZE-PLAN §6.1, C40) by sourcing the jail harness's harness/lib/jail.sh,
# which is packaging/ of this tree (or $BARDO_COMPOSITOR_DESKTOP, a folder holding harness/lib/jail.sh). Missing: prints the fix and exits 3.
_compositor_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../compositor" && pwd)
_compositor_desktop=""
for _d in "${BARDO_COMPOSITOR_DESKTOP:-}" "$_compositor_root/../../packaging"; do
  if [ -n "$_d" ] && [ -f "$_d/harness/lib/jail.sh" ]; then _compositor_desktop=$_d; break; fi
done
if [ -z "$_compositor_desktop" ]; then
  echo "dev/lib/jail.sh: the harness (packaging/harness/lib/jail.sh of the bardo tree) was not found" >&2
  echo "fix: run this from a bardo tree, or set BARDO_COMPOSITOR_DESKTOP to a folder holding harness/lib/jail.sh" >&2
  exit 3
fi
# shellcheck source=/dev/null
source "$_compositor_desktop/harness/lib/jail.sh"
unset _d
