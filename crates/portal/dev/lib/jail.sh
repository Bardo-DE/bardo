# Sourceable: provides `require_jail` by sourcing the desktop harness's harness/lib/jail.sh and
# bus.sh, found as a sibling checkout: $BARDO_PORTAL_DESKTOP, then <repo>/../desktop, then
# $HOME/desktop. Missing: prints the fix and exits 3.
_portal_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../portal" && pwd)
_portal_desktop=""
for _d in "${BARDO_PORTAL_DESKTOP:-}" "$_portal_root/../desktop" "$HOME/desktop"; do
  if [ -n "$_d" ] && [ -f "$_d/harness/lib/jail.sh" ]; then _portal_desktop=$_d; break; fi
done
if [ -z "$_portal_desktop" ]; then
  echo "dev/lib/jail.sh: the desktop harness (harness/lib/jail.sh) was not found" >&2
  echo "fix: check out the desktop repository beside bardo-portal, or set BARDO_PORTAL_DESKTOP" >&2
  exit 3
fi
# shellcheck source=/dev/null
source "$_portal_desktop/harness/lib/jail.sh"
# shellcheck source=/dev/null
source "$_portal_desktop/harness/lib/bus.sh"
unset _d
