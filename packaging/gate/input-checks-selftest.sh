#!/usr/bin/env bash
# The input checks against generated trees: a clean one passes, and each fault they exist for fails.
# Reads the real scripts; builds a made-up tree (packaging/ and crates/<domain>/) in a scratch folder; touches nothing else.
#
#   input-checks-selftest.sh
# Exit 0 pass, 1 a check failed.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tmp=$(mktemp -d)
trap 'rm -rf -- "${tmp:?}"' EXIT
fail() { echo "input-checks-selftest: $*" >&2; exit 1; }

# A fresh world: $tmp/w/bardo/packaging (the script, build scripts) beside made-up crate folders.
world() {
  rm -rf -- "${tmp:?}/w"
  local d=$tmp/w/bardo/packaging s=$tmp/w/bardo/crates
  mkdir -p "$d/gate" "$d/dist" "$d/release" "$d/install" "$d/.github/workflows" "$d/harness" \
    "$s/input/input-broker/dist" "$s/input/key-remap/dist" "$s/input/mouse-gestures/dist" "$s/shell/dist"
  cp "$HERE/check-no-uinput-access.sh" "$d/gate/"
  printf '#!/usr/bin/env bash\ncargo_in "$d" input_broker -p input-broker-daemon\nput_system 644 "$t/dist/72-bardo-input-broker.rules" /etc/udev/rules.d/72-bardo-input-broker.rules\nput_system 644 "$t/dist/bardo-input-broker.service" /usr/lib/systemd/system/bardo-input-broker.service\nput_system 644 "$k/dist/bardo-key-remap.service" /usr/lib/systemd/system/bardo-key-remap.service\n' >"$d/release/build.sh"
  printf '#!/usr/bin/env bash\n' >"$d/install.sh"
  printf '#!/usr/bin/env bash\necho "644 $x/dist/72-bardo-input-broker.rules /etc/udev/rules.d/72-bardo-input-broker.rules"\n' >"$d/install/system.sh"
  printf 'KERNEL=="uinput", SUBSYSTEM=="misc", MODE="0600", GROUP="root", TAG-="uaccess", OPTIONS+="static_node=uinput"\n' >"$s/input/input-broker/dist/72-bardo-input-broker.rules"
  printf '[Service]\nExecStart=/usr/local/bin/bardo-input-broker\nDeviceAllow=/dev/uinput rw\n' >"$s/input/input-broker/dist/bardo-input-broker.service"
  printf '[Service]\nExecStart=/usr/local/bin/bardo-key-remap\nDeviceAllow=char-input rw\n' >"$s/input/key-remap/dist/bardo-key-remap.service"
  git -C "$tmp/w/bardo" init -q 2>/dev/null
}
uinput() { "$tmp/w/bardo/packaging/gate/check-no-uinput-access.sh" "$@" >"$tmp/out" 2>&1; }
ok() { "$@"; local rc=$?; [ "$rc" = 0 ] || { cat "$tmp/out"; fail "expected a pass: $* (exit $rc)"; }; }
bad() { local want=$1; shift; "$@"; local rc=$?; [ "$rc" = "$want" ] || { cat "$tmp/out"; fail "expected exit $want: $* (exit $rc)"; }; }
says() { grep -q -- "$1" "$tmp/out" || { cat "$tmp/out"; fail "the message does not say: $1"; }; }

# --- nothing opens /dev/uinput
world
ok uinput; says "3 rule or service"
world; printf 'KERNEL=="uinput", GROUP="input", MODE="0660", OPTIONS+="static_node=uinput"\n' >"$tmp/w/bardo/crates/input/input-broker/dist/72-bardo-input-broker.rules"
bad 1 uinput; says 'GROUP="root"'
world; printf 'KERNEL=="uinput", MODE="0600", GROUP="root", TAG+="uaccess"\n' >"$tmp/w/bardo/crates/input/input-broker/dist/72-bardo-input-broker.rules"
bad 1 uinput; says "uaccess"
world; printf 'KERNEL=="uinput", MODE="0600", GROUP="root", TAG-="uaccess"\n' >"$tmp/w/bardo/crates/input/input-broker/dist/72-bardo-input-broker.rules"
ok uinput
world; printf '[Service]\nDeviceAllow=/dev/uinput rw\n' >>"$tmp/w/bardo/crates/input/key-remap/dist/bardo-key-remap.service"
bad 1 uinput; says "other than the input service"
world; printf 'put_system 644 "$d/dist/70-bardo-key-remap.rules" /etc/udev/rules.d/70-bardo-key-remap.rules\n' >>"$tmp/w/bardo/packaging/release/build.sh"
bad 1 uinput; says "70-bardo-key-remap.rules"
world; printf 'install -Dm644 "$k/dist/bardo-key-remap.uinput.conf" "$UNITDIR/bardo-key-remap.service.d/10-uinput.conf"\n' >>"$tmp/w/bardo/packaging/install/system.sh"
bad 1 uinput; says "bardo-key-remap.uinput.conf"
world; printf 'note "WARNING: /etc/udev/rules.d/70-bardo-key-remap.rules from an earlier install"\n' >>"$tmp/w/bardo/packaging/install/system.sh"
ok uinput
world; mkdir -p "$tmp/stage/usr/lib/systemd/system/bardo-key-remap.service.d" "$tmp/stage/etc/udev/rules.d"
printf '[Service]\nDeviceAllow=/dev/uinput rw\n' >"$tmp/stage/usr/lib/systemd/system/bardo-key-remap.service.d/10-uinput.conf"
bad 1 uinput --stage "$tmp/stage"; says "drop-in"
rm -rf "$tmp/stage"; mkdir -p "$tmp/stage/etc/udev/rules.d"; cp "$tmp/w/bardo/crates/input/input-broker/dist/72-bardo-input-broker.rules" "$tmp/stage/etc/udev/rules.d/"
ok uinput --stage "$tmp/stage"
printf 'KERNEL=="uinput", GROUP="input", MODE="0660"\n' >"$tmp/stage/etc/udev/rules.d/70-bardo-key-remap.rules"
bad 1 uinput --stage "$tmp/stage"

echo "input-checks-selftest: ok"
