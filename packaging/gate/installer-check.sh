#!/usr/bin/env bash
# The top-level installer against stub component installers in a scratch tree (crates/<domain>/...): the dry run prints the plan in
# order and runs nothing, a real run passes the prefix and the staging root through, a failing component stops the run
# with a message naming it, and nothing writes outside the scratch directory (HOME is the scratch one) or calls sudo.
#
#   installer-check.sh
# Exit 0 pass, 1 a check failed.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL="$HERE/../install.sh"
tmp=$(mktemp -d)
trap 'rm -rf -- "${tmp:?}"' EXIT
sib="$tmp/tree" dest="$tmp/dest" log="$tmp/calls.log" home="$tmp/home"
mkdir -p "$home" "$tmp/bin"
: >"$log"

# Stubs: each installer records its name, arguments and the variables it is given.
stub() { # REPO-RELATIVE-SCRIPT FAIL-NAME
  local f="$sib/crates/$1"
  mkdir -p "$(dirname "$f")"
  cat >"$f" <<STUB
#!/usr/bin/env bash
echo "$1 \$* PREFIX=\${PREFIX:-} DESTDIR=\${DESTDIR:-} CTD=\${CARGO_TARGET_DIR:-} BARDO_FILES_PREFIX=\${BARDO_FILES_PREFIX:-} BARDO_FILES_DESTDIR=\${BARDO_FILES_DESTDIR:-} FLAGS=\${RUSTFLAGS-unset}" >>"$log"
[ "\${STUB_FAIL:-}" != "$2" ] || exit 7
STUB
  chmod +x "$f"
}
stub accounts/scripts/install.sh accounts
stub inference/scripts/install.sh inference
stub sync/scripts/install.sh sync
stub models/scripts/install.sh models
stub memory/scripts/install.sh memory
stub actions/scripts/install.sh actions
stub computer-use/scripts/install.sh computer-use
stub settings-app/scripts/install.sh settings
stub compositor/scripts/install.sh compositor
stub shell/dist/install.sh shell
stub portal/scripts/install.sh portal
stub input/key-remap/install.sh key-remap
stub files/dist/install.sh files
stub input/mouse-gestures/install.sh mouse-gestures
# The two units the session installer puts in /etc/systemd/user, as the compositor and shell checkouts ship them.
mkdir -p "$sib/crates/compositor/dist" "$sib/crates/shell/dist"
# the stub units carry the loader line the real ones do (peephole's list), so the loader step below checks them as it would a release
unset_line='UnsetEnvironment=LD_PRELOAD LD_LIBRARY_PATH LD_AUDIT GCONV_PATH GIO_EXTRA_MODULES GTK_MODULES GTK_PATH GST_PLUGIN_PATH GST_PLUGIN_SYSTEM_PATH LIBGL_DRIVERS_PATH MESA_LOADER_DRIVER_OVERRIDE __EGL_VENDOR_LIBRARY_DIRS __EGL_VENDOR_LIBRARY_FILENAMES GBM_BACKENDS_PATH LIBVA_DRIVERS_PATH VK_ICD_FILENAMES VK_DRIVER_FILES VK_ADD_DRIVER_FILES VK_LAYER_PATH VK_ADD_LAYER_PATH VK_INSTANCE_LAYERS VK_LOADER_LAYERS_ENABLE'
printf '[Service]\nExecStart=/usr/bin/bardo-compositor $BARDO_COMPOSITOR_SESSION_ARGS\n%s\n' "$unset_line" >"$sib/crates/compositor/dist/bardo-compositor.service"
printf '[Service]\nExecStart=/usr/bin/bardo-shell run\n%s\n' "$unset_line" >"$sib/crates/shell/dist/bardo-shell.service"
mkdir -p "$sib/target/release"; : >"$sib/target/release/bardo-settings"; chmod +x "$sib/target/release/bardo-settings"
# The input service, the keyboard map and the touchpad service: the files install/system.sh takes from their crates (the unit as
# each crate ships it), and their programs, already built (so no compiler runs here). key-remap's and mouse-gestures's own installers,
# stubbed above, must never run.
mkdir -p "$sib/crates/input/input-broker/dist" "$sib/crates/input/key-remap/dist" "$sib/crates/input/mouse-gestures/dist"
printf '[Socket]\nListenSequentialPacket=/run/bardo/input-broker/input.sock\nSocketMode=0666\n' >"$sib/crates/input/input-broker/dist/bardo-input-broker.socket"
printf '[Service]\nExecStart=/usr/local/bin/bardo-input-broker\nUser=root\n%s\n' "$unset_line" >"$sib/crates/input/input-broker/dist/bardo-input-broker.service"
printf 'KERNEL=="uinput", SUBSYSTEM=="misc", MODE="0600", GROUP="root", TAG-="uaccess", OPTIONS+="static_node=uinput"\n' >"$sib/crates/input/input-broker/dist/72-bardo-input-broker.rules"
printf 'uinput\n' >"$sib/crates/input/input-broker/dist/bardo-input-broker-modules.conf"
printf '[Service]\nExecStart=/usr/local/bin/bardo-key-remap\nDynamicUser=yes\n%s\n' "$unset_line" >"$sib/crates/input/key-remap/dist/bardo-key-remap.service"
printf '// polkit\n' >"$sib/crates/input/key-remap/dist/bardo-key-remap.polkit.rules"
printf 'KERNEL=="uinput", GROUP="input", MODE="0660"\n' >"$sib/crates/input/key-remap/dist/70-bardo-key-remap.rules"
printf '[Service]\nDeviceAllow=/dev/uinput rw\n' >"$sib/crates/input/key-remap/dist/bardo-key-remap.uinput.conf"
printf '[Service]\nExecStart=/usr/local/bin/bardo-mouse-gestures\nDynamicUser=yes\n%s\n' "$unset_line" >"$sib/crates/input/mouse-gestures/dist/bardo-mouse-gestures.service"
printf 'SUBSYSTEM=="hidraw", KERNELS=="0005:004C:0269.*", GROUP="input", MODE="0660"\n' >"$sib/crates/input/mouse-gestures/dist/70-bardo-mouse-gestures.rules"
printf 'options hid_magicmouse emulate_scroll_wheel=0 emulate_3button=1\n' >"$sib/crates/input/mouse-gestures/dist/99-bardo-mouse-gestures.conf"
for b in bardo-input-broker bardo-key-remap bardo-mouse-gestures; do printf '#!/bin/sh\n' >"$sib/target/release/$b"; chmod +x "$sib/target/release/$b"; done
# A sudo that fails the check when called.
printf '#!/bin/sh\necho sudo >>"%s"\nexit 1\n' "$log" >"$tmp/bin/sudo"; chmod +x "$tmp/bin/sudo"
export PATH="$tmp/bin:$PATH" HOME="$home" XDG_CONFIG_HOME="$home/.config"

fail() { echo "installer-check: $*" >&2; exit 1; }
# inst never builds (the programs are in place above); inst_build does, with a stub cargo.
inst() { env -u PREFIX -u DESTDIR -u SYSCONFDIR -u RUSTFLAGS -u CARGO_TARGET_DIR -u BARDO_TREE SUDO=asroot "$INSTALL" --tree "$sib" --no-build "$@"; }
inst_build() { env -u PREFIX -u DESTDIR -u SYSCONFDIR -u RUSTFLAGS -u CARGO_TARGET_DIR -u BARDO_TREE SUDO=asroot "$INSTALL" --tree "$sib" "$@"; }

# 1. dry run: the plan, in order, nothing run.
plan=$(inst --dry-run --destdir "$dest") || fail "dry run failed"
[ ! -s "$log" ] || fail "the dry run ran an installer: $(cat "$log")"
order=$(printf '%s\n' "$plan" | grep -E '^[0-9]+\.' | awk '{print $2}' | tr '\n' ' ')
[ "$order" = "callers accounts inference sync models memory actions computer-use settings compositor shell portal input-broker key-remap files mouse-gestures session " ] || fail "plan order is: $order"
printf '%s\n' "$plan" | grep -q 'callers .*runs .*install/system.sh callers --destdir' || fail "the callers table is not placed first, into the staging root: $plan"
printf '%s\n' "$plan" | grep -q 'input-broker .*runs .*install/system.sh input-broker --prefix /usr/local --destdir' || fail "the input service does not run into a staging root: $plan"
printf '%s\n' "$plan" | grep -q 'key-remap .*runs .*install/system.sh key-remap --prefix /usr/local --destdir' || fail "the keyboard map is not placed by the desktop's own script: $plan"
printf '%s\n' "$plan" | grep -q 'mouse-gestures .*runs .*install/system.sh mouse-gestures --prefix /usr/local --destdir' || fail "the touchpad service is not placed by the desktop's own script, into the staging root: $plan"
! printf '%s\n' "$plan" | grep -i 'mouse-gestures' | grep -q 'your home' || fail "mouse-gestures is still said to be installed under the home: $plan"
printf '%s\n' "$plan" | grep -q 'shell .*yourself' || fail "shell is not left to the person"
printf '%s\n' "$plan" | grep -q 'accounts .*runs .*scripts/install.sh --prefix /usr/local --sysconfdir /etc --no-build --destdir' || fail "accounts does not get the desktop prefix and /etc: $plan"
printf '%s\n' "$plan" | grep -q 'models .*runs .*scripts/install.sh --prefix /usr/local --sysconfdir /etc --no-build --destdir' || fail "models does not get the desktop prefix, /etc and the destdir: $plan"
! printf '%s\n' "$plan" | grep -q 'note: .* is not on the session' || fail "the default prefix got the search-path note: $plan"
printf '%s\n' "$plan" | grep -q 'memory .*runs .*scripts/install.sh --prefix /usr/local --no-build --destdir' || fail "memory does not run into a staging root: $plan"
printf '%s\n' "$plan" | grep -q 'actions .*runs .*scripts/install.sh --prefix /usr/local --no-build --destdir' || fail "actions does not run into a staging root: $plan"
printf '%s\n' "$plan" | grep -q 'computer-use .*runs .*scripts/install.sh --prefix /usr/local --no-build --destdir' || fail "computer-use does not run into a staging root: $plan"
! printf '%s\n' "$plan" | grep -E 'memory|actions|computer-use ' | grep -q -- '--sysconfdir' || fail "memory, actions or computer-use was given --sysconfdir (their tables are read from /etc/bardo only)"
printf '%s\n' "$plan" | grep -q 'session .*runs' || fail "the session step does not run into a staging root"

# 2. a system prefix without root is left to the person, not attempted.
plan=$(inst --dry-run --prefix /usr) || fail "system prefix dry run failed"
printf '%s\n' "$plan" | grep -q 'accounts .*yourself (needs root' || fail "a system prefix was not reported as needing root"
printf '%s\n' "$plan" | grep -q 'session .*yourself (needs root' || fail "the session step was not left to the person without root: $plan"
printf '%s\n' "$plan" | grep -q '&& asroot .*install/install.sh --prefix /usr --sysconfdir /etc' || fail "the session root command does not go through the escalation program: $plan"
printf '%s\n' "$plan" | grep -q 'cd .*accounts && asroot .*scripts/install.sh --prefix /usr' || fail "the root command does not go through the escalation program: $plan"
printf '%s\n' "$plan" | grep -q 'callers .*yourself (needs root: /etc/bardo' || fail "the callers table was not left to the person without root: $plan"
printf '%s\n' "$plan" | grep -q '&& asroot .*install/system.sh callers *$' || fail "the callers root command does not go through the escalation program: $plan"
printf '%s\n' "$plan" | grep -q '&& asroot .*install/system.sh input-broker --prefix /usr --tree' || fail "the input service root command is missing: $plan"
printf '%s\n' "$plan" | grep -q '&& asroot .*install/system.sh mouse-gestures --prefix /usr --tree' || fail "the touchpad service root command is missing: $plan"

# 2b. without a staging root memory, actions and computer-use are left to the person, as themselves: their installers refuse to
# run as root and ask for sudo for /etc/bardo themselves, so no root command is printed for them.
plan=$(inst --dry-run --prefix "$tmp/user-prefix") || fail "user prefix dry run failed"
declare -A domain=([memory]=memory [actions]=actions [computer-use]=computer-use [models]=models)
for c in memory actions computer-use; do
  printf '%s\n' "$plan" | grep -q "$c .*yourself (its caller table" || fail "$c was not left to the person: $plan"
  printf '%s\n' "$plan" | grep -q "cd .*${domain[$c]} && .*scripts/install.sh --prefix $tmp/user-prefix --no-build *\$" || fail "$c command is missing: $plan"
  ! printf '%s\n' "$plan" | grep -q "asroot .*${domain[$c]}/scripts/install.sh" || fail "$c was offered as a root command: $plan"
done

# 2c. without a staging root models is left to the person, as themselves (its installer asks for sudo itself).
plan=$(inst --dry-run --prefix "$tmp/user-prefix") || fail "user prefix dry run failed"
printf '%s\n' "$plan" | grep -q 'models .*yourself (its installer asks for sudo' || fail "models was not left to the person: $plan"
! printf '%s\n' "$plan" | grep -q "asroot .*${domain[models]}/scripts/install.sh" || fail "models was offered as a root command: $plan"

# 2d. a prefix off the session's search paths prints, never writes, how to tell the session: the data dirs line and the unit links.
for p in /opt/d "$tmp/user-prefix"; do
  plan=$(inst --dry-run --prefix "$p") || fail "dry run with prefix $p failed"
  printf '%s\n' "$plan" | grep -q "XDG_DATA_DIRS=$p/share:/usr/local/share:/usr/share\$" || fail "no XDG_DATA_DIRS line for $p: $plan"
  printf '%s\n' "$plan" | grep -q "environment.d" || fail "the note does not name environment.d for $p: $plan"
  printf '%s\n' "$plan" | grep -q "ln -sf $p/lib/systemd/user/\* /usr/local/lib/systemd/user/" || fail "no unit link line for $p: $plan"
done
for p in /usr /usr/local; do
  ! inst --dry-run --prefix "$p" | grep -q 'XDG_DATA_DIRS=' || fail "prefix $p got the search-path note"
done
[ ! -e "$home/.config/environment.d" ] || fail "the note was written, not printed"

# 2e. a program root runs never lives where the person can replace it. A user-owned prefix (--prefix ~/.local) is refused by
# install/system.sh for the input service, the keyboard map and the touchpad service, and install.sh puts those three under /usr/local instead and says so;
# a prefix root holds is kept. (Without a staging root: the check judges the real folders.)
mkdir -p "$tmp/user-prefix/bin"
for part in input-broker key-remap mouse-gestures; do
  if "$HERE/../install/system.sh" "$part" --dry-run --prefix "$tmp/user-prefix" --tree "$sib" >"$tmp/out-unheld.log" 2>&1; then
    fail "install/system.sh $part accepted a prefix the person owns: $(cat "$tmp/out-unheld.log")"
  fi
  grep -q "is not a safe place for a program that root runs" "$tmp/out-unheld.log" || fail "the refusal for $part does not say why: $(cat "$tmp/out-unheld.log")"
  "$HERE/../install/system.sh" "$part" --dry-run --prefix /usr --tree "$sib" >"$tmp/out-held.log" 2>&1 || { cat "$tmp/out-held.log" >&2; fail "install/system.sh $part refused /usr, which root holds"; }
  plan=$(inst --dry-run --prefix "$tmp/user-prefix" --only "$part") || fail "dry run with a user prefix failed for $part"
  printf '%s\n' "$plan" | grep -q "install/system.sh $part --prefix /usr/local" || fail "$part was not moved to /usr/local for a user prefix: $plan"
  ! printf '%s\n' "$plan" | grep -q "install/system.sh $part --prefix $tmp/user-prefix" || fail "$part was planned under the user's prefix: $plan"
  printf '%s\n' "$plan" | grep -q "note: $part runs as root, so its program goes under /usr/local" || fail "the move of $part was not said: $plan"
  plan=$(inst --dry-run --prefix /usr --only "$part") || fail "dry run with /usr failed for $part"
  printf '%s\n' "$plan" | grep -q "install/system.sh $part --prefix /usr " || fail "$part left /usr, which root holds: $plan"
done
[ -z "$(find "$tmp/user-prefix" -type f)" ] || fail "a refused install left files under the person's prefix"
# a staging root is not judged: the folder it becomes is the packager's
plan=$(inst --dry-run --destdir "$dest" --prefix "$tmp/user-prefix" --only input-broker) || fail "staged dry run with a user prefix failed"
printf '%s\n' "$plan" | grep -q "install/system.sh input-broker --prefix $tmp/user-prefix --destdir" || fail "a staged prefix was moved: $plan"

# 3. a real run into a staging root passes PREFIX and DESTDIR through, runs the stageable ones, never sudo.
inst --destdir "$dest" --prefix /opt/d >"$tmp/out.log" 2>&1 || { cat "$tmp/out.log" >&2; fail "staged run failed"; }
# accounts goes under the same prefix as everything else (its daemons find what they ship under XDG_DATA_DIRS),
# with the caller tables in --sysconfdir.
grep -q "^accounts/scripts/install.sh --prefix /opt/d --sysconfdir /etc --no-build --destdir $dest" "$log" || fail "accounts did not get the prefix, /etc and the destdir: $(cat "$log")"
grep -q "^models/scripts/install.sh --prefix /opt/d --sysconfdir /etc --no-build --destdir $dest" "$log" || fail "models did not get the prefix, /etc and the destdir: $(cat "$log")"
grep -q "^compositor/scripts/install.sh --prefix /opt/d --locked --no-build --destdir $dest" "$log" || fail "compositor did not get the prefix and destdir: $(cat "$log")"
! grep -q 'note: accounts goes to' "$tmp/out.log" || fail "a accounts prefix note is still printed: $(cat "$tmp/out.log")"
# the session step is this repo's own installer: really run, into the staging root
[ -x "$dest/opt/d/bin/bardo-session" ] || fail "bardo-session was not staged under the prefix"
[ -f "$dest/etc/systemd/user/bardo-session.target" ] || fail "the user target was not staged"
# the portal frontend's own unit gets a root-held drop-in that unsets the loader variables (NEW-PI-1): staged next to the units, the
# same bytes as the repo's, and complete against peephole's list (the check skips, exit 2, when no peephole checkout is beside this one)
dropin="$dest/etc/systemd/user/xdg-desktop-portal.service.d/10-bardo.conf"
[ -f "$dropin" ] || fail "the portal frontend's drop-in was not staged"
cmp -s "$HERE/../session/xdg-desktop-portal.service.d/10-bardo.conf" "$dropin" || fail "the staged portal drop-in is not the repo's"
[ "$(stat -c %a "$dropin")" = 644 ] || fail "the portal drop-in is not mode 644"
rc=0; "$HERE/check-loader-unset.sh" --crates "$sib/crates" --dropin "$dropin" >"$tmp/loader.log" 2>&1 || rc=$?
[ "$rc" = 0 ] || [ "$rc" = 2 ] || fail "the staged portal drop-in does not unset every loader variable: $(cat "$tmp/loader.log")"
grep -q '^Wants=bardo-updater.service$' "$dest/etc/systemd/user/bardo-session.target" || fail "the user target does not want the update service"
[ -f "$dest/usr/share/wayland-sessions/bardo.desktop" ] || fail "the login entry was not staged"
grep -q '^ExecStart=/opt/d/bin/bardo-compositor \$BARDO_COMPOSITOR_SESSION_ARGS$' "$dest/etc/systemd/user/bardo-compositor.service" || fail "bardo-compositor.service was not staged with the prefix and the session arguments: $(cat "$dest/etc/systemd/user/bardo-compositor.service" 2>&1)"
grep -q '^ExecStart=/opt/d/bin/bardo-shell run$' "$dest/etc/systemd/user/bardo-shell.service" || fail "bardo-shell.service was not staged with the prefix: $(cat "$dest/etc/systemd/user/bardo-shell.service" 2>&1)"
[ "$(stat -c %a "$dest/etc/systemd/user/bardo-compositor.service")" = 644 ] || fail "bardo-compositor.service is not mode 644"
grep -q "^memory/scripts/install.sh --prefix /opt/d --no-build --destdir $dest" "$log" || fail "memory did not get the prefix and destdir: $(cat "$log")"
grep -q "^actions/scripts/install.sh --prefix /opt/d --no-build --destdir $dest" "$log" || fail "actions did not get the prefix and destdir: $(cat "$log")"
grep -q "^computer-use/scripts/install.sh --prefix /opt/d --no-build --destdir $dest" "$log" || fail "computer-use did not get the prefix and destdir: $(cat "$log")"
grep -q "^settings-app/scripts/install.sh  PREFIX=/opt/d DESTDIR=$dest" "$log" || fail "settings did not get PREFIX and DESTDIR"
grep -q "^portal/scripts/install.sh --prefix /opt/d --no-build --destdir $dest" "$log" || fail "portal did not get the prefix and destdir"
grep -q "^accounts/scripts/install.sh .*CTD=$sib/target " "$log" || fail "accounts was not told where the one target directory is: $(cat "$log")"
grep -q "BARDO_FILES_PREFIX=/opt/d BARDO_FILES_DESTDIR=$dest" "$log" || fail "files did not get its prefix and destdir"
! grep -q "^input/key-remap/install.sh" "$log" || fail "key-remap's own installer ran: it gives the input group /dev/uinput and the service a drop-in to open it"
! grep -q "^input/mouse-gestures/install.sh" "$log" || fail "mouse-gestures's own installer ran: the desktop places the touchpad service with install/system.sh"
grep -q "^portal/scripts/install.sh" "$log" || fail "no component ran"
# the desktop's own system files: the callers table (an exact copy, 644), the input service and the keyboard map
cmp -s "$HERE/../dist/desktop-callers.toml" "$dest/etc/bardo/desktop-callers.toml" || fail "the callers table was not staged as it is in dist/"
[ "$(stat -c %a "$dest/etc/bardo/desktop-callers.toml")" = 644 ] || fail "the callers table is not mode 644"
for f in usr/lib/systemd/system/bardo-input-broker.socket usr/lib/systemd/system/bardo-input-broker.service etc/udev/rules.d/72-bardo-input-broker.rules etc/modules-load.d/bardo-input-broker.conf \
  opt/d/bin/bardo-input-broker usr/lib/systemd/system/bardo-key-remap.service etc/polkit-1/rules.d/bardo-key-remap.polkit.rules opt/d/bin/bardo-key-remap usr/lib/bardo/key-remap/key-remap.toml; do
  [ -f "$dest/$f" ] || fail "$f was not staged"
done
grep -q '^ExecStart=/opt/d/bin/bardo-input-broker$' "$dest/usr/lib/systemd/system/bardo-input-broker.service" || fail "bardo-input-broker.service does not start the installed program: $(cat "$dest/usr/lib/systemd/system/bardo-input-broker.service")"
grep -q '^ExecStart=/opt/d/bin/bardo-key-remap$' "$dest/usr/lib/systemd/system/bardo-key-remap.service" || fail "bardo-key-remap.service does not start the installed program"
grep -q 'rules = \[ { when = "capslock", then = "ctrl+space" } \]\|when = "capslock", then = "ctrl+space"' "$dest/usr/lib/bardo/key-remap/key-remap.toml" || fail "the vendor key-remap.toml has no Caps Lock rule"
grep -q 'when = "shift+capslock", then = "capslock"' "$dest/usr/lib/bardo/key-remap/key-remap.toml" || fail "the vendor key-remap.toml has no Shift+Caps Lock rule (key-remap refuses a Shift trigger in a user layer)"
cmp -s "$HERE/../dist/key-remap.toml" "$dest/usr/lib/bardo/key-remap/key-remap.toml" || fail "the vendor key-remap.toml was not staged as it is in dist/"
[ "$(stat -c %a "$dest/usr/lib/bardo/key-remap/key-remap.toml")" = 644 ] || fail "the vendor key-remap.toml is not mode 644"
# the touchpad service is a system service like key-remap: the program, the unit (a plain file, started by the installed program), the rule
# that gives the mouse's raw node to the input group and no person, and the module options; nothing for /dev/uinput, and no user unit
for f in opt/d/bin/bardo-mouse-gestures usr/lib/systemd/system/bardo-mouse-gestures.service etc/udev/rules.d/70-bardo-mouse-gestures.rules etc/modprobe.d/99-bardo-mouse-gestures.conf; do
  [ -f "$dest/$f" ] || fail "$f was not staged"
done
grep -q '^ExecStart=/opt/d/bin/bardo-mouse-gestures$' "$dest/usr/lib/systemd/system/bardo-mouse-gestures.service" || fail "bardo-mouse-gestures.service does not start the installed program: $(cat "$dest/usr/lib/systemd/system/bardo-mouse-gestures.service")"
[ ! -e "$dest/etc/systemd/user/bardo-mouse-gestures.service" ] || fail "a user unit for mouse-gestures was staged"
grep -q 'GROUP="input"' "$dest/etc/udev/rules.d/70-bardo-mouse-gestures.rules" || fail "the hidraw rule does not give the node to the input group"
! grep -q 'uaccess' "$dest/etc/udev/rules.d/70-bardo-mouse-gestures.rules" || fail "the hidraw rule gives the node to the person"
[ -z "$(find "$dest" -name 'bardo-mouse-gestures-modules.conf' -o -name 'bardo-mouse-gestures.conf')" ] || fail "mouse-gestures's own module list was staged (the input service loads uinput)"
# bardo-key-remap.service is the same file from install.sh and from a release (bardo-updater replaces a system file only when it is absent, equal or its own
# earlier one): the path is the one the updater's list names, and at the default prefix the installed unit is the checkout's unit byte for byte,
# which is what build.sh stages (no retarget; /usr/local/bin/bardo-key-remap is the link exports.toml makes to bin/bardo-key-remap)
grep -qE '^/usr/lib/systemd/system +exact +bardo-key-remap\.service$' "$HERE/../release/system-files.list" || fail "the updater's list (release/system-files.list) does not name /usr/lib/systemd/system/bardo-key-remap.service"
inst --destdir "$tmp/dest-default" --prefix /usr/local --only key-remap,mouse-gestures >"$tmp/out-default.log" 2>&1 || { cat "$tmp/out-default.log" >&2; fail "the keyboard map or touchpad step failed at the default prefix"; }
cmp -s "$sib/crates/input/mouse-gestures/dist/bardo-mouse-gestures.service" "$tmp/dest-default/usr/lib/systemd/system/bardo-mouse-gestures.service" || fail "bardo-mouse-gestures.service from install.sh at the default prefix is not the unit a release stages: $(cat "$tmp/dest-default/usr/lib/systemd/system/bardo-mouse-gestures.service" 2>&1)"
grep -qE '^/usr/lib/systemd/system +exact +bardo-mouse-gestures\.service$' "$HERE/../release/system-files.list" || fail "the updater's list (release/system-files.list) does not name /usr/lib/systemd/system/bardo-mouse-gestures.service"
grep -qF 'put_system 644 "$d/dist/bardo-mouse-gestures.service" /usr/lib/systemd/system/bardo-mouse-gestures.service' "$HERE/../release/build.sh" || fail "release/build.sh does not stage bardo-mouse-gestures.service at /usr/lib/systemd/system"
! grep -q 'retarget.*bardo-mouse-gestures\.service' "$HERE/../release/build.sh" || fail "release/build.sh rewrites bardo-mouse-gestures.service's ExecStart: install.sh's unit and the release's would differ"
cmp -s "$sib/crates/input/key-remap/dist/bardo-key-remap.service" "$tmp/dest-default/usr/lib/systemd/system/bardo-key-remap.service" || fail "bardo-key-remap.service from install.sh at the default prefix is not the unit a release stages: $(cat "$tmp/dest-default/usr/lib/systemd/system/bardo-key-remap.service" 2>&1)"
grep -qF 'put_system 644 "$d/dist/bardo-key-remap.service" /usr/lib/systemd/system/bardo-key-remap.service' "$HERE/../release/build.sh" || fail "release/build.sh does not stage bardo-key-remap.service at /usr/lib/systemd/system"
! grep -q 'retarget.*bardo-key-remap\.service' "$HERE/../release/build.sh" || fail "release/build.sh rewrites bardo-key-remap.service's ExecStart: install.sh's unit and the release's would differ"
grep -A1 -x 'link = "/usr/local/bin/bardo-key-remap"' "$HERE/../release/exports.toml" | grep -qx 'target = "bin/bardo-key-remap"' || fail "release/exports.toml has no /usr/local/bin/bardo-key-remap link to bin/bardo-key-remap"
# /etc/bardo/key-remap is the administrator's: the install writes nothing there
[ ! -e "$dest/etc/bardo/key-remap" ] || fail "the install wrote into /etc/bardo/key-remap: $(ls -A "$dest/etc/bardo/key-remap")"
# nothing gives the person, or a group, the device file; the old stand-alone pieces are not staged
[ ! -e "$dest/etc/udev/rules.d/70-bardo-key-remap.rules" ] || fail "key-remap's group rule for /dev/uinput was staged"
[ -z "$(find "$dest" -name '*uinput.conf' -o -name '10-uinput*')" ] || fail "a uinput drop-in was staged: $(find "$dest" -name '*uinput.conf' -o -name '10-uinput*')"
if grep -rh 'uaccess' "$dest/etc/udev/rules.d" 2>/dev/null | grep -v 'TAG-="uaccess"' | grep -q .; then fail "a staged rule gives the person a device file"; fi
# a second run replaces a vendor key-remap.toml that was changed, and leaves the administrator's file alone
printf '[profile.default]\nrules = []\n' >"$dest/usr/lib/bardo/key-remap/key-remap.toml"
mkdir -p "$dest/etc/bardo/key-remap"; printf '[profile.default]\nrules = []\n' >"$dest/etc/bardo/key-remap/key-remap.toml"
inst --destdir "$dest" --prefix /opt/d --only key-remap >"$tmp/out2.log" 2>&1 || { cat "$tmp/out2.log" >&2; fail "the keyboard map step failed the second time"; }
cmp -s "$HERE/../dist/key-remap.toml" "$dest/usr/lib/bardo/key-remap/key-remap.toml" || fail "a changed vendor key-remap.toml was not replaced"
[ "$(cat "$dest/etc/bardo/key-remap/key-remap.toml")" = "$(printf '[profile.default]\nrules = []')" ] || fail "the administrator's key-remap.toml was changed"
# the removal takes away what is unchanged (the vendor file) and never looks at /etc/bardo/key-remap
"$HERE/../install/system.sh" --remove key-remap --destdir "$dest" --prefix /opt/d --tree "$sib" >"$tmp/out3.log" 2>&1 || { cat "$tmp/out3.log" >&2; fail "removing the keyboard map failed"; }
[ ! -e "$dest/opt/d/bin/bardo-key-remap" ] || fail "the keyboard map program was kept by the removal"
[ ! -e "$dest/usr/lib/bardo/key-remap/key-remap.toml" ] || fail "the vendor key-remap.toml was kept by the removal"
[ -f "$dest/etc/bardo/key-remap/key-remap.toml" ] || fail "the administrator's key-remap.toml was removed"
"$HERE/../install/system.sh" --remove input-broker --destdir "$dest" --prefix /opt/d --tree "$sib" >"$tmp/out3.log" 2>&1 || fail "removing the input service failed"
[ ! -e "$dest/etc/udev/rules.d/72-bardo-input-broker.rules" ] || fail "the device rule was kept by the removal"
"$HERE/../install/system.sh" --remove mouse-gestures --destdir "$dest" --prefix /opt/d --tree "$sib" >"$tmp/out3.log" 2>&1 || { cat "$tmp/out3.log" >&2; fail "removing the touchpad service failed"; }
[ ! -e "$dest/opt/d/bin/bardo-mouse-gestures" ] || fail "the touchpad program was kept by the removal"
[ ! -e "$dest/usr/lib/systemd/system/bardo-mouse-gestures.service" ] || fail "the touchpad unit was kept by the removal"
[ ! -e "$dest/etc/udev/rules.d/70-bardo-mouse-gestures.rules" ] || fail "the hidraw rule was kept by the removal"
inst --destdir "$dest" --prefix /opt/d --only key-remap,input-broker,mouse-gestures >/dev/null 2>&1 || fail "the input steps did not run again"
! grep -q sudo "$log" || fail "sudo was called"
[ ! -e "$home/.config" ] || fail "something wrote the scratch ~/.config"
grep -q 'left to you.*shell' "$tmp/out.log" || fail "the summary does not name what is left"

# 3b. the build: one cargo run for everything the selected components install, with the system's linker. cargo runs from /
# with --manifest-path of the tree (so the tree's .cargo/config.toml is not read), the tree's pinned toolchain goes in
# RUSTUP_TOOLCHAIN, and neither RUSTFLAGS nor a linker variable is set. cargo here is a stub.
grep -q 'FLAGS=unset' "$log" || fail "an installer was given RUSTFLAGS: $(cat "$log")"
! grep -q 'LINKER' "$log" || fail "an installer was given a linker: $(cat "$log")"
rm -f -- "$sib/target/release/bardo-settings"
printf '[toolchain]\nchannel = "9.9.9"\n' >"$sib/rust-toolchain.toml"
printf '#!/bin/sh\necho "cargo cwd=$PWD TC=${RUSTUP_TOOLCHAIN:-} FLAGS=${RUSTFLAGS-unset} LINKER=${CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER:-} $*" >>"%s"\nmkdir -p "$CARGO_TARGET_DIR/release" && : >"$CARGO_TARGET_DIR/release/bardo-settings" && chmod +x "$CARGO_TARGET_DIR/release/bardo-settings"\n' "$log" >"$tmp/bin/cargo"
printf '#!/bin/sh\nexit 0\n' >"$tmp/bin/rustup"
chmod +x "$tmp/bin/cargo" "$tmp/bin/rustup"
: >"$log"
inst_build --destdir "$dest" --only settings >"$tmp/out.log" 2>&1 || { cat "$tmp/out.log" >&2; fail "the settings build run failed"; }
grep -q "^cargo cwd=/ TC=9.9.9 FLAGS=unset LINKER= build --release --locked --manifest-path $sib/Cargo.toml --config net.git-fetch-with-cli=true -p settings-app\$" "$log" || fail "settings was not built from / with --manifest-path: $(cat "$log")"
[ "$(grep -c '^cargo ' "$log")" = 1 ] || fail "the build was more than one cargo run: $(cat "$log")"
# several components: still one cargo run, each package once
: >"$log"
inst_build --destdir "$dest" --only compositor,shell,accounts >"$tmp/out.log" 2>&1 || { cat "$tmp/out.log" >&2; fail "the build run for three components failed"; }
grep -q "^cargo cwd=/ .* -p account-daemon -p compositor-app -p compositor-ctl -p shell-app\$" "$log" || fail "three components were not one cargo run: $(cat "$log")"
[ "$(grep -c '^cargo ' "$log")" = 1 ] || fail "the build was more than one cargo run: $(cat "$log")"
rm -f -- "$tmp/bin/cargo" "$tmp/bin/rustup"
: >"$log"

# 3c. the emoji font: staged under the prefix with its licence; not when --only leaves it out; not when fontconfig
# already has a COLRv1 Noto Color Emoji; fc-cache runs on a real (unstaged) install.
[ -f "$dest/opt/d/share/fonts/bardo/Noto-COLRv1.ttf" ] || fail "the emoji font was not staged"
[ -f "$dest/opt/d/share/fonts/bardo/OFL-Noto-COLRv1.txt" ] || fail "the font licence was not staged"
rm -rf -- "$tmp/dest2"
inst --destdir "$tmp/dest2" --prefix /opt/d --only accounts >/dev/null 2>&1 || fail "--only accounts failed"
[ ! -e "$tmp/dest2/opt/d/share/fonts" ] || fail "--only accounts installed the font"
mkdir -p "$tmp/fcbin" "$tmp/user"
printf '#!/bin/sh\necho "fc-cache $*" >>"%s"\n' "$log" >"$tmp/fcbin/fc-cache"
printf '#!/bin/sh\necho "/usr/share/fonts/Noto-COLRv1.ttf: Noto Color Emoji:style=Regular"\n' >"$tmp/fcbin/fc-list-present"
printf '#!/bin/sh\necho "/usr/share/fonts/NotoColorEmoji.ttf: Noto Color Emoji:style=Regular"\n' >"$tmp/fcbin/fc-list"
chmod +x "$tmp/fcbin"/*
: >"$log"
PATH="$tmp/fcbin:$PATH" inst --prefix "$tmp/user" --only fonts >"$tmp/out.log" 2>&1 || { cat "$tmp/out.log" >&2; fail "the font step failed"; }
[ -f "$tmp/user/share/fonts/bardo/Noto-COLRv1.ttf" ] || fail "the font was not installed under the prefix"
grep -q "fc-cache -f $tmp/user/share/fonts/bardo" "$log" || fail "fc-cache was not run: $(cat "$log")"
rm -rf -- "$tmp/user"; : >"$log"
mv "$tmp/fcbin/fc-list-present" "$tmp/fcbin/fc-list"
PATH="$tmp/fcbin:$PATH" inst --prefix "$tmp/user" --only fonts >"$tmp/out.log" 2>&1 || fail "the font step failed with a system font"
[ ! -e "$tmp/user/share/fonts" ] || fail "the font was installed although fontconfig has a COLRv1 one"
grep -q 'fontconfig already has' "$tmp/out.log" || fail "the skip was not said"
: >"$log"

# 3d. the loader-variable check itself, on generated trees: a unit that lacks one name of the list is a fault, a complete one passes, a
# unit on the debt list that is complete is a fault, and the drop-in is held to the same list.
lt="$tmp/loader"; mkdir -p "$lt/identity/caller-identity-core/src" "$lt/sibs/x/dist" "$lt/sibs/y/dist" "$lt/sibs/desktop"
printf 'pub const LOADER_VARIABLES: [&str; 3] = [\n    "LD_PRELOAD",\n    "LD_LIBRARY_PATH",\n    "GTK_PATH",\n];\n' >"$lt/identity/caller-identity-core/src/facts.rs"
printf '[[peer]]\napp = "a"\nunit = "a.service"\nexe = ["/usr/bin/a"]\nrole = "r"\n[[peer]]\napp = "b"\nunit = "b.service"\nexe = ["/usr/bin/b"]\nrole = "r"\n' >"$lt/table.toml"
printf '[Service]\nUnsetEnvironment=LD_PRELOAD LD_LIBRARY_PATH\nUnsetEnvironment=GTK_PATH\n' >"$lt/sibs/x/dist/a.service"
printf '[Service]\nUnsetEnvironment=LD_PRELOAD LD_LIBRARY_PATH\n' >"$lt/sibs/y/dist/b.service"
printf '[Service]\nUnsetEnvironment=LD_PRELOAD LD_LIBRARY_PATH GTK_PATH\n' >"$lt/dropin.conf"
: >"$lt/pending.txt"
loader() { "$HERE/check-loader-unset.sh" --peephole "$lt/identity" --crates "$lt/sibs" --table "$lt/table.toml" --pending "$lt/pending.txt" "$@" >"$lt/out.log" 2>&1; }
loader && fail "a unit that lacks GTK_PATH passed the loader check: $(cat "$lt/out.log")"
grep -q 'b.service .*does not unset: GTK_PATH' "$lt/out.log" || fail "the loader check does not name the unit and the variable: $(cat "$lt/out.log")"
printf 'b.service y\n' >"$lt/pending.txt"
loader || fail "a pending unit that is still missing a name broke the check: $(cat "$lt/out.log")"
printf 'UnsetEnvironment=GTK_PATH\n' >>"$lt/sibs/y/dist/b.service"
loader && fail "a complete unit still on the debt list was accepted"
: >"$lt/pending.txt"
loader || fail "complete units were refused: $(cat "$lt/out.log")"
printf '[Service]\nUnsetEnvironment=LD_PRELOAD\n' >"$lt/short.conf"
loader --dropin "$lt/short.conf" && fail "an incomplete drop-in was accepted"
loader --dropin "$lt/dropin.conf" || fail "a complete drop-in was refused: $(cat "$lt/out.log")"

# 3e. the session's settings file is read as data (ROOT-14): a line that would run a program is ignored, not run; the three
# settings count, with or without quotes; a number setting needs a number. The function is taken out of the script and run alone.
se="$tmp/session-env"; mkdir -p "$se"
log() { printf '%s\n' "$*" >>"$se/log"; }
eval "$(sed -n '/^read_settings() {$/,/^}$/p' "$HERE/../session/bardo-session")"
printf '# a comment\nBARDO_SESSION_QUICK_SECS=7\nBARDO_SESSION_BACKEND_ARGS="--backend headless"\nBARDO_SESSION_FIRST_RUN_DELAY=soon\nPATH=/evil\ntouch %s/ran\n$(touch %s/ran2)\n' "$se" "$se" >"$se/session.env"
( unset BARDO_SESSION_QUICK_SECS BARDO_SESSION_BACKEND_ARGS BARDO_SESSION_FIRST_RUN_DELAY
  read_settings "$se/session.env"
  [ "$BARDO_SESSION_QUICK_SECS" = 7 ] || exit 1
  [ "$BARDO_SESSION_BACKEND_ARGS" = "--backend headless" ] || exit 2
  [ -z "${BARDO_SESSION_FIRST_RUN_DELAY:-}" ] || exit 3
  [ "$PATH" != /evil ] || exit 4 ) || fail "session.env was not read as the three settings (exit $?)"
[ ! -e "$se/ran" ] && [ ! -e "$se/ran2" ] || fail "session.env ran a command"
grep -q 'ignored PATH' "$se/log" || fail "the ignored name was not logged"
unset -f log read_settings

# 4. a failing component stops the run and is named.
: >"$log"
if STUB_FAIL=settings inst --destdir "$dest" >"$tmp/out.log" 2>&1; then fail "a failing component did not fail the run"; fi
grep -q 'settings failed (exit 7)' "$tmp/out.log" || fail "the failure does not name the component: $(cat "$tmp/out.log")"
! grep -q "^portal/" "$log" || fail "a component after the failure ran"

# 4c. the session uninstall removes the two units only while they are still what the installer wrote.
units="$dest/etc/systemd/user"
"$HERE/../install/uninstall.sh" --destdir "$dest" --prefix /opt/d --tree "$sib" >"$tmp/out.log" 2>&1 || { cat "$tmp/out.log" >&2; fail "uninstall failed"; }
[ ! -e "$units/bardo-compositor.service" ] || fail "an unchanged bardo-compositor.service was kept"
[ ! -e "$units/bardo-shell.service" ] || fail "an unchanged bardo-shell.service was kept"
[ ! -e "$dest/opt/d/bin/bardo-session" ] || fail "the uninstall left bardo-session"
inst --destdir "$dest" --prefix /opt/d --only session >/dev/null 2>&1 || fail "the session step did not run again"
printf '[Service]\nExecStart=/opt/mine/bardo-compositor\n' >"$units/bardo-compositor.service"
"$HERE/../install/uninstall.sh" --destdir "$dest" --prefix /opt/d --tree "$sib" >"$tmp/out.log" 2>&1 || fail "uninstall failed (changed unit)"
[ -f "$units/bardo-compositor.service" ] || fail "a changed bardo-compositor.service was removed"
grep -q 'kept (not what the installer wrote)' "$tmp/out.log" || fail "the kept unit was not reported"
[ ! -e "$units/bardo-shell.service" ] || fail "an unchanged bardo-shell.service was kept (second run)"

# 4d. a unit whose source is missing is warned about and skipped, not an error.
rm -f -- "$sib/crates/shell/dist/bardo-shell.service"
out=$("$HERE/../install/install.sh" --dry-run --destdir "$dest" --prefix /opt/d --tree "$sib" 2>&1) || fail "a missing unit source failed the session installer"
printf '%s\n' "$out" | grep -q 'WARNING: .*bardo-shell.service is missing' || fail "the missing bardo-shell.service was not warned about: $out"

# 5. a missing component folder is exit 2 with its path.
rm -rf -- "${sib:?}/crates/files"
inst --dry-run >"$tmp/out.log" 2>&1 && fail "a missing component did not fail"
grep -q 'files: no installer' "$tmp/out.log" || fail "the missing component is not named"
echo "installer-check: ok"
