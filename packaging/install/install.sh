#!/usr/bin/env bash
# Install the "sill (casement)" session: the session script, the portal choice for the quire
# desktop, the user target and the login-screen entry. Run as yourself; it never runs sudo (a
# destination you cannot write stops it with the command to run as root). Safe to run again.
#
#   install/install.sh --dry-run    print every step, change nothing
#   install/install.sh              do it
#   (flags: see install/lib.sh)
#
# Also installed, root-held, in <sysconfdir>/systemd/user (/etc/systemd/user): bardo-compositor.service and bardo-shell.service, from the
# casement and sill crates of this tree (--tree), so the account service can trust the compositor and the
# shell as fixed roles (it accepts units only from root-held places, never ~/.config).
# And, in the same place, xdg-desktop-portal.service.d/10-bardo.conf: a drop-in for the portal frontend's own unit that unsets the
# loader variables, so the portal backend (transom) can prove the frontend is the real one (NEW-PI-1).
# Not installed here: the casement, sill and transom (the portal backend) programs themselves (cargo xtask install, then cargo xtask install transom, and
# packages), other sessions (left untouched), and
# anything under $HOME. install/uninstall.sh removes exactly the files listed below.
set -euo pipefail
ORIG_ARGS=("$@")
# Folders made here are readable by everybody whatever root's umask is (a 0700 share folder hides the files from the person).
umask 022
# shellcheck source=lib.sh
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
usage() { sed -n '2,/^set -euo/p' "${BASH_SOURCE[0]}" | sed '$d' | sed 's/^# \{0,1\}//'; }
parse_flags "$@"
[ "$dry" = 0 ] || echo "dry run: nothing below is executed"

step "1/3 checks"
for tool in casement sill; do
  command -v "$tool" >/dev/null || note "WARNING: $tool is not on PATH; the session cannot start without it"
done
for pair in compositor:casement shell:sill; do
  name=${pair#*:}
  [ -f "$(unit_source "${pair%%:*}" "$name")" ] || note "WARNING: $(unit_source "${pair%%:*}" "$name") is missing; the session needs $name.service and cannot start $name without it (pass --tree if this is not the bardo tree's packaging folder)"
  # A copy in the person's own folder wins over the installed one and is not trusted by the account service.
  if [ -z "$destdir" ] && [ -f "${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user/$name.service" ]; then
    note "WARNING: ${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user/$name.service hides the installed unit and is not trusted by the account service; remove it"
  fi
done
[ -x "$prefix/libexec/transom" ] || [ -x /usr/libexec/bardo/bardo-portal ] || note "WARNING: transom is not installed (the repo's scripts/install.sh, or the top-level ./install.sh --only transom); screen sharing and screenshots need it"
portal_found=0
for portal in /usr/libexec/xdg-desktop-portal /usr/lib/xdg-desktop-portal /usr/lib/*/xdg-desktop-portal; do
  [ ! -x "$portal" ] || portal_found=1
done
[ "$portal_found" = 1 ] || note "WARNING: xdg-desktop-portal is missing (screenshots and screen sharing need it): $(install_hint portal | tr '\n' ' ')"

step "2/3 the files"
dests=()
while read -r _ source dest; do if [ -f "$source" ]; then dests+=("$dest"); fi; done < <(files)
need_write "${dests[@]}"
while read -r mode source dest; do
  [ -f "$source" ] || { note "skipped: $destdir$dest (no $source)"; continue; }
  note "$destdir$dest ($mode)"
  [ "$dry" = 1 ] || {
    install -d -m 755 "$(dirname "$destdir$dest")"
    content "$source" | install -m "$mode" /dev/stdin "$destdir$dest"
  }
done < <(files)

step "3/3 the user manager"
if [ -n "$destdir" ]; then
  note "staged under $destdir: nothing to reload"
elif [ "$dry" = 1 ]; then
  note "\$ systemctl --user daemon-reload"
else
  systemctl --user daemon-reload || note "could not reload the user manager; log out and in"
fi

step "done"
note "Pick \"sill (casement)\" at the login screen. First-login checks (not testable without a login):"
note "  the guard's real tty path, sill and transom against the real casement, the target, bardo-compositor.service and bardo-shell.service"
note "  the activation environment, a screen share through transom (a browser, OBS), logout through sill's power menu."
