# Sourceable: provides `require_jail` by sourcing the harness's lib/jail.sh and lib/bus.sh from
# packaging/harness/lib of this repository. Missing: prints the fix and exits 3.
_portal_harness=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../../packaging/harness" 2>/dev/null && pwd)
if [ -z "$_portal_harness" ] || [ ! -f "$_portal_harness/lib/jail.sh" ]; then
  echo "dev/lib/jail.sh: the harness (packaging/harness/lib/jail.sh) was not found" >&2
  echo "fix: run this script from a full checkout of the repository" >&2
  exit 3
fi
# shellcheck source=/dev/null
source "$_portal_harness/lib/jail.sh"
# shellcheck source=/dev/null
source "$_portal_harness/lib/bus.sh"
unset _portal_harness
