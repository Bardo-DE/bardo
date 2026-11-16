#!/usr/bin/env bash
# Take away what an install of the desktop under its earlier names (quire, sill, casement, detent, transom, slipway,
# turnstile, keycap, palmrest, accountd, ...) left on this machine, so installing the bardo names over it is clean:
#
#   install/old-layout.sh [--dry-run] [--prefix DIR] [--destdir DIR] [--sysconfdir DIR]
#
#   - old user and system units are stopped and disabled first (as the person's own manager when this runs as you,
#     through the person who ran sudo when it runs as root), then their files and drop-in folders are removed;
#   - the old programs (sill, detent, casement, casementctl, quire-do, docket-agent, docket-acp, docket-agents, turnstile,
#     keycap, palmrest, slipway, desktop-session, files) are removed from the prefix's bin folder and /usr/local/bin;
#   - the old libexec folders (/usr/libexec/quire, <prefix>/libexec/quire, transom, slipway-apply, slipway-rollback,
#     slipway-health) are removed;
#   - the old bus activation files (org.quire.*.service), polkit actions (org.quire.*.policy), udev rules, module
#     lists, polkit rules, the PAM service sill, the login entry quire.desktop and the portal files quire.portal and
#     quire-portals.conf are removed, and so are the old caller tables the earlier installs wrote in /etc/quire,
#     /etc/porter and /etc/xdg/quire;
#   - <prefix>/share/quire (the earlier data folder) and the emoji font folder fonts/quire are removed.
#
# Nothing named bardo-* or org.bardo.* is touched, and neither is anything in a home folder (old home folders are not
# read). /opt/quire (an earlier release tree) is left alone: the update service
# bootstraps /opt/bardo (see release/README.md). Never runs sudo: a place you cannot write stops it with the command
# to run as root. Safe to run again: what is already gone is not mentioned.
# Exit: 0 done, 1 something failed or a place is not writable, 2 bad usage.
set -euo pipefail
ORIG_ARGS=("$@")
# shellcheck source=lib.sh
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
usage() { sed -n '2,/^set -euo/p' "${BASH_SOURCE[0]}" | sed '$d' | sed 's/^# \{0,1\}//'; }
parse_flags "$@"
[ "$dry" = 0 ] || echo "dry run: nothing below is executed"

# The earlier names of the units (without the .service suffix), and of the two sockets.
old_user_units=(accountd syncd inferd memoryd intentd companiond readerd voiced cuad actions-mcp docket-agent docket-acp
  casement sill transom slipway porter-mountd keycap-focus org.quire.Settings)
old_user_other=(porter-mountd.socket desktop-session.target sill-session.target sill.target)
old_system_units=(turnstile.socket turnstile.service keycap.service palmrest.service)
old_programs=(sill detent casement casementctl quire-do docket-agent docket-acp docket-agents turnstile keycap palmrest slipway
  desktop-session files)

# The places the earlier installs wrote user units into.
user_unit_dirs=("$prefix/lib/systemd/user" "$sysconfdir/systemd/user" /usr/lib/systemd/user /usr/local/lib/systemd/user)

# Everything found, as "KIND PATH" lines: kind file | tree.
found=()
declare -A seen=()
add_found() { if [ -z "${seen[$2]:-}" ]; then seen[$2]=1; found+=("$1 $2"); fi; }
note_found() { if [ -e "$destdir$2" ] || [ -L "$destdir$2" ]; then add_found "$1" "$2"; fi; }

glob_found() { # KIND PATTERN: every existing path matching PATTERN (a glob under the destdir)
  local kind=$1 path
  for path in $destdir$2; do
    [ -e "$path" ] || [ -L "$path" ] || continue
    add_found "$kind" "${path#"$destdir"}"
  done
}

# The units, their drop-in folders and the symlinks the person's own manager made when they were enabled.
for dir in "${user_unit_dirs[@]}"; do
  for unit in "${old_user_units[@]}"; do
    note_found file "$dir/$unit.service"
    note_found tree "$dir/$unit.service.d"
  done
  for other in "${old_user_other[@]}"; do note_found file "$dir/$other"; done
  note_found file "$dir/xdg-desktop-portal.service.d/10-quire.conf"
done
for dir in /usr/lib/systemd/system /etc/systemd/system; do
  for unit in "${old_system_units[@]}"; do
    note_found file "$dir/$unit"
    note_found tree "$dir/$unit.d"
  done
done

# The programs, the libexec folders.
bins=("$prefix/bin")
[ "$prefix" = /usr/local ] || bins+=(/usr/local/bin)
for bin in "${bins[@]}"; do
  for program in "${old_programs[@]}"; do
    # `files` is a common name: under /usr it may be someone else's.
    if [ "$program" = files ] && [ "$bin" = /usr/bin ]; then continue; fi
    note_found file "$bin/$program"
  done
done
for libexec in "$prefix/libexec" /usr/local/libexec /usr/libexec; do
  note_found tree "$libexec/quire"
  note_found file "$libexec/transom"
  for helper in apply rollback health; do note_found file "$libexec/slipway-$helper"; done
done

# Bus activation, polkit actions, login entry, portal files, data folders.
for share in "$prefix/share" /usr/local/share /usr/share; do
  glob_found file "$share/dbus-1/services/org.quire.*.service"
  note_found file "$share/dbus-1/services/org.freedesktop.impl.portal.desktop.quire.service"
  glob_found file "$share/polkit-1/actions/org.quire.*.policy"
  note_found file "$share/xdg-desktop-portal/portals/quire.portal"
  note_found file "$share/xdg-desktop-portal/quire-portals.conf"
  note_found tree "$share/quire"
  note_found tree "$share/fonts/quire"
  note_found tree "$share/slipway"
done
note_found file "$sessions_dir/quire.desktop"
note_found file "$sessions_dir/sill-casement.desktop"

# System files of the input services and the shell, and the old caller tables.
note_found file /etc/udev/rules.d/72-turnstile.rules
note_found file /etc/udev/rules.d/70-palmrest.rules
note_found file /etc/udev/rules.d/70-keycap.rules
note_found file /etc/udev/rules.d/60-sill-ddc.rules
note_found file /etc/modules-load.d/turnstile.conf
note_found file /etc/modules-load.d/keycap.conf
note_found file /etc/modules-load.d/sill-ddc.conf
note_found file /etc/modprobe.d/99-palmrest.conf
note_found file /etc/polkit-1/rules.d/keycap.polkit.rules
note_found file /etc/pam.d/sill
note_found tree /usr/lib/keycap
glob_found file "/etc/quire/*-callers.toml"
glob_found file "/etc/quire/*-callers.toml.dist"
note_found file /etc/quire/docket-floor.toml
note_found tree /etc/xdg/quire
note_found file /etc/porter/callers.toml
note_found file /etc/porter/callers.toml.dist
note_found file /etc/porter/inferd-callers.toml
note_found file /etc/porter/inferd-callers.toml.dist

if [ "${#found[@]}" = 0 ]; then
  step "nothing of the earlier names is installed"
  exit 0
fi

step "the earlier names found (${#found[@]})"
paths=()
for entry in "${found[@]}"; do
  note "${entry#* }"
  paths+=("${entry#* }")
done
need_write "${paths[@]}"

# live: really changing this machine (not a staging root, not a dry run).
live() { [ -z "$destdir" ] && [ "$dry" = 0 ]; }
systemd_running() { [ -d /run/systemd/system ]; }

# user_ctl ARGS...: `systemctl --user ARGS` for the person: as you when you are not root, as the person who ran sudo otherwise.
user_ctl() {
  if [ "$(id -u)" != 0 ]; then
    systemctl --user "$@"
  elif [ -n "${SUDO_USER:-}" ] && [ "$SUDO_USER" != root ]; then
    local uid
    uid=$(id -u "$SUDO_USER")
    [ -S "/run/user/$uid/bus" ] || return 1
    sudo -u "$SUDO_USER" env "XDG_RUNTIME_DIR=/run/user/$uid" "DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/$uid/bus" systemctl --user "$@"
  else
    return 1
  fi
}

if live && systemd_running; then
  step "stopping and disabling the earlier units"
  names=()
  for unit in "${old_user_units[@]}"; do names+=("$unit.service"); done
  names+=(porter-mountd.socket)
  user_ctl disable --now "${names[@]}" >/dev/null 2>&1 || note "(the person's own units could not be reached from here, or none were enabled)"
  if [ "$(id -u)" = 0 ]; then
    systemctl disable --now "${old_system_units[@]}" >/dev/null 2>&1 || true
    # Units enabled for every user (systemctl --global) leave links under /etc/systemd/user.
    systemctl --global disable "${names[@]}" >/dev/null 2>&1 || true
  fi
fi

step "removing"
for entry in "${found[@]}"; do
  kind=${entry%% *}
  path=${entry#* }
  note "remove: $destdir$path"
  [ "$dry" = 1 ] || { if [ "$kind" = tree ]; then rm -rf -- "${destdir:?}$path"; else rm -f -- "$destdir$path"; fi; }
done
# The folders the old caller tables lived in, when nothing else is left in them.
if [ "$dry" = 0 ]; then
  for dir in /etc/quire /etc/porter; do rmdir "$destdir$dir" 2>/dev/null || true; done
fi

if live && systemd_running; then
  step "telling the managers"
  systemctl daemon-reload 2>/dev/null || true
  user_ctl daemon-reload 2>/dev/null || note "log out and in (or run: systemctl --user daemon-reload) so your own manager forgets the old units"
  if [ "$(id -u)" = 0 ] && command -v udevadm >/dev/null 2>&1; then udevadm control --reload-rules 2>/dev/null || true; fi
fi
step "done"
