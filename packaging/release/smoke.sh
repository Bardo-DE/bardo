#!/usr/bin/env bash
# Install a packed release with bardo-updater's privileged helper on THIS computer and check the result: bardo-updater's
# own install is complete (the three programs in /usr/libexec/bardo and the one polkit file, release/check-helpers.sh),
# every export is a link to a file of the release, every system file is in place, `current` names the release,
# every program's libraries resolve, and installing the same version again is refused. For a throwaway
# machine (a CI container): it writes /etc/bardo, /opt/bardo, /usr/local/bin, /usr/libexec, /usr/share and the
# system folders as root. It refuses to run as anyone else, or without --really.
#
#   release/smoke.sh --release DIR --pubkey FILE --helpers DIR --policy FILE --really [--root DIR]
#
#   --release DIR  the packed release (release.toml, release.toml.minisig, bundle.tar.zst)
#   --pubkey FILE  the key it is signed with
#   --helpers DIR  the built bardo-updater-apply, bardo-updater-rollback and bardo-updater-health
#   --policy FILE  bardo-updater's polkit file (dist/org.bardo.updater.policy), installed as bardo-updater's install does
#   --root DIR     the install root (default /opt/bardo)
#   --really       yes, this machine may be changed
#
# Exit: 0 all checks pass, 1 a check failed, 2 bad usage, not root or a tool missing.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=release/lib.sh
. "$HERE/lib.sh"
usage() { sed -n '2,/^set -euo/p' "${BASH_SOURCE[0]}" | sed '$d' | sed 's/^# \{0,1\}//'; }
release="" pub="" helpers="" policy="" root=/opt/bardo really=0
while [ $# -gt 0 ]; do
  case "$1" in
    --release) release=${2:?}; shift ;;
    --pubkey) pub=${2:?}; shift ;;
    --helpers) helpers=${2:?}; shift ;;
    --policy) policy=${2:?}; shift ;;
    --root) root=${2:?}; shift ;;
    --really) really=1 ;;
    -h|--help) usage; exit 0 ;;
    *) usage >&2; exit 2 ;;
  esac
  shift
done
[ -n "$release" ] && [ -f "$pub" ] && [ -d "$helpers" ] && [ -f "$policy" ] || { usage >&2; exit 2; }
[ "$really" = 1 ] || { echo "smoke.sh: this changes the whole computer; add --really on a throwaway one" >&2; exit 2; }
[ "$(id -u)" = 0 ] || { echo "smoke.sh: run as root (in the container)" >&2; exit 2; }
for tool in python3 zstd minisign; do
  command -v "$tool" >/dev/null || { echo "smoke.sh: $tool is not installed ($(install_hint "$tool"))" >&2; exit 2; }
done
release=$(cd "$release" && pwd)

step() { echo "== $*"; }
step "verify the release"
"$HERE/verify.sh" "$release" --pubkey "$pub"

step "install the key and the settings the helper reads"
install -Dm644 "$pub" /usr/share/bardo/updater/release.pub
install -d -m 755 /etc/bardo
printf 'root = "%s"\npubkey_path = "/usr/share/bardo/updater/release.pub"\n' "$root" >/etc/bardo/updater.toml

step "bardo-updater's own install: three programs and one polkit file"
for program in bardo-updater-apply bardo-updater-rollback bardo-updater-health; do
  [ -x "$helpers/$program" ] || { echo "smoke.sh: $helpers/$program is not there" >&2; exit 2; }
  install -Dm755 "$helpers/$program" "/usr/libexec/bardo/$program"
done
install -Dm644 "$policy" /usr/share/polkit-1/actions/org.bardo.updater.policy
"$HERE/check-helpers.sh"
helper=/usr/libexec/bardo/bardo-updater-apply

step "bardo-updater-apply"
"$helper" "$release"

step "what is in place"
python3 -I "$HERE/release.py" installed "$release" "$root"

step "libraries resolve here"
"$HERE/check-ldd.sh" --stage "$root/current"
"$HERE/check-glibc.sh" --stage "$root/current"

step "the same version again is refused"
set +e
"$helper" "$release" >/dev/null 2>&1
code=$?
set -e
[ "$code" = 3 ] || { echo "smoke.sh: installing the same version again exited $code, not 3 (refused)" >&2; exit 1; }
echo "smoke: pass"
