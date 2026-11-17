#!/usr/bin/env bash
# The three input and caller checks against generated trees: a clean one passes, and each fault they exist for fails.
# Reads the real scripts; builds a made-up tree (packaging/ and crates/<domain>/) in a scratch folder; touches nothing else.
#
#   input-checks-selftest.sh
# Exit 0 pass, 1 a check failed.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REAL="$(cd "$HERE/.." && pwd)"
tmp=$(mktemp -d)
trap 'rm -rf -- "${tmp:?}"' EXIT
fail() { echo "input-checks-selftest: $*" >&2; exit 1; }

# A fresh world: $tmp/w/bardo/packaging (the three scripts, a table, build scripts) beside made-up crate folders and the caller-identity one.
world() {
  rm -rf -- "${tmp:?}/w"
  local d=$tmp/w/bardo/packaging s=$tmp/w/bardo/crates
  mkdir -p "$d/gate" "$d/dist" "$d/release" "$d/install" "$d/.github/workflows" "$d/harness" \
    "$s/identity/caller-identity-core/src" "$s/input/input-broker/dist" "$s/input/key-remap/dist" "$s/input/mouse-gestures/dist" "$s/shell/dist" "$s/input/input-broker-daemon"
  cp "$HERE/check-callers-table.sh" "$HERE/check-no-fake.sh" "$HERE/check-no-uinput-access.sh" "$d/gate/"
  cp "$REAL/dist/desktop-callers.toml" "$d/dist/"
  printf '#!/usr/bin/env bash\ncargo_in "$d" input_broker -p input-broker-daemon\nput_system 644 "$t/dist/72-bardo-input-broker.rules" /etc/udev/rules.d/72-bardo-input-broker.rules\nput_system 644 "$t/dist/bardo-input-broker.service" /usr/lib/systemd/system/bardo-input-broker.service\nput_system 644 "$k/dist/bardo-key-remap.service" /usr/lib/systemd/system/bardo-key-remap.service\n' >"$d/release/build.sh"
  printf '#!/usr/bin/env bash\n' >"$d/install.sh"
  printf '#!/usr/bin/env bash\necho "644 $x/dist/72-bardo-input-broker.rules /etc/udev/rules.d/72-bardo-input-broker.rules"\n' >"$d/install/system.sh"
  # The caller-identity library's role list, copied from the real one when it is in this tree, else the eighteen written out
  if [ -f "$REAL/../crates/identity/caller-identity-core/src/roles.rs" ]; then cp "$REAL/../crates/identity/caller-identity-core/src/roles.rs" "$s/identity/caller-identity-core/src/"
  else
    {
      for r in SHELL PORTAL PORTAL_FRONTEND COMPOSITOR AGENT_TIER SETTINGS UPDATER UPDATER_CLI PAD_DAEMON KEYMAP_DAEMON ACCOUNTS INFERENCE SYNC ROUTER COMPANION MEMORY SANDBOX_LAUNCHER MANAGER; do
        printf 'pub const %s: RoleName = RoleName::from_static("%s");\n' "$r" "$(echo "$r" | tr A-Z a-z)"
      done
      printf 'pub const ALL: [RoleName; 18] = [SHELL, PORTAL, PORTAL_FRONTEND, COMPOSITOR, AGENT_TIER, SETTINGS, UPDATER, UPDATER_CLI, PAD_DAEMON, KEYMAP_DAEMON, ACCOUNTS, INFERENCE, SYNC, ROUTER, COMPANION, MEMORY, SANDBOX_LAUNCHER, MANAGER];\n'
    } >"$s/identity/caller-identity-core/src/roles.rs"
  fi
  printf 'KERNEL=="uinput", SUBSYSTEM=="misc", MODE="0600", GROUP="root", TAG-="uaccess", OPTIONS+="static_node=uinput"\n' >"$s/input/input-broker/dist/72-bardo-input-broker.rules"
  printf '[Service]\nExecStart=/usr/local/bin/bardo-input-broker\nDeviceAllow=/dev/uinput rw\n' >"$s/input/input-broker/dist/bardo-input-broker.service"
  printf '[Service]\nExecStart=/usr/local/bin/bardo-key-remap\nDeviceAllow=char-input rw\n' >"$s/input/key-remap/dist/bardo-key-remap.service"
  printf '[package]\nname = "x"\n[features]\nfake = []\ndefault = []\n[dev-dependencies]\ncaller_identity = { version = "1", features = ["fake"] }\n' >"$s/input/input-broker-daemon/Cargo.toml"
  git -C "$tmp/w/bardo" init -q 2>/dev/null
}
callers() { "$tmp/w/bardo/packaging/gate/check-callers-table.sh" "$@" >"$tmp/out" 2>&1; }
nofake() { "$tmp/w/bardo/packaging/gate/check-no-fake.sh" "$@" >"$tmp/out" 2>&1; }
uinput() { "$tmp/w/bardo/packaging/gate/check-no-uinput-access.sh" "$@" >"$tmp/out" 2>&1; }
ok() { "$@"; local rc=$?; [ "$rc" = 0 ] || { cat "$tmp/out"; fail "expected a pass: $* (exit $rc)"; }; }
bad() { local want=$1; shift; "$@"; local rc=$?; [ "$rc" = "$want" ] || { cat "$tmp/out"; fail "expected exit $want: $* (exit $rc)"; }; }
says() { grep -q -- "$1" "$tmp/out" || { cat "$tmp/out"; fail "the message does not say: $1"; }; }
t() { echo "$tmp/w/bardo/packaging/dist/desktop-callers.toml"; }

# --- the table
world
ok callers
mutate() { # PYTHON-EXPRESSION on `text`: rewrites the table
  python3 - "$(t)" "$1" <<'PY'
import sys
path, code = sys.argv[1], sys.argv[2]
text = open(path).read()
scope = {"text": text}
exec(code, scope)
open(path, "w").write(scope["text"])
PY
}
world; mutate 'text = text.replace("role = \"memory\"", "role = \"memry\"")'
bad 1 callers; says "memry"; says "no row for the role memory"
world; mutate 'text = text.replace("unit = \"bardo-shell.service\"\n", "")'
bad 1 callers; says "no unit, and 'shell' is a service"
world; mutate 'text = text.replace("\"/usr/bin/bardo-shell\"", "\"usr/bin/shell\"")'
bad 1 callers; says "is not an absolute path"
world; mutate 'text = text.replace("\"/usr/bin/bardo-shell\"", "\"/usr/bin/../shell\"")'
bad 1 callers; says "empty, . or .. segment"
world; mutate 'text = text.replace("\"/usr/bin/bardo-shell\"", "\"/usr/*/*/shell\"")'
ok callers
world; mutate 'text = text.replace("\"/usr/bin/bardo-shell\"", "\"/usr/*x*/shell\"")'
bad 1 callers; says "more than one"
world; mutate 'text = text.replace("unit = \"bardo-compositor.service\"", "unit = \"bardo-shell.service\"")'
bad 1 callers; says "also in row"
world; mutate 'i = text.index("# ...and also by a launcher"); j = text.index("# The software-update service."); text = text[:i] + text[j:]'
bad 1 callers; says "settings has no row without a unit"
world; mutate 'text = text.replace("[[peer]]\napp  = \"org.bardo.Shell\"", "[[peer]]\nstray = 1\napp  = \"org.bardo.Shell\"")'
bad 1 callers; says "unknown key"
world; printf '[Service]\nExecStart=/srv/somewhere/shell run\n' >"$tmp/w/bardo/crates/shell/dist/bardo-shell.service"
bad 1 callers; says "ExecStart /srv/somewhere/shell"
world; printf '[Service]\nExecStart=/usr/bin/bardo-shell run\n' >"$tmp/w/bardo/crates/shell/dist/bardo-shell.service"
ok callers; says "2 unit(s) compared"
world; rm -rf "$tmp/w/bardo/crates/identity"
bad 2 callers --caller-identity "$tmp/w/none"; says "no caller-identity crate"

# --- no stand-in
world
ok nofake
world; printf 'cargo_in "$d" x --features fake\n' >>"$tmp/w/bardo/packaging/release/build.sh"
bad 1 nofake; says "build.sh:6"
world; printf 'cargo build --all-features\n' >>"$tmp/w/bardo/packaging/install.sh"
bad 1 nofake; says "all features"
world; printf '# cargo build --features fake (a comment)\n' >>"$tmp/w/bardo/packaging/install.sh"
ok nofake
world; printf '      - run: cargo build --release -p input-broker-daemon --features input_broker/fake\n' >"$tmp/w/bardo/packaging/.github/workflows/release.yml"
bad 1 nofake
world; printf '[features]\nfake = []\ndefault = ["extra"]\nextra = ["dep/fake"]\n' >"$tmp/w/bardo/crates/input/input-broker-daemon/Cargo.toml"
bad 1 nofake; says "default features"
world; printf '[features]\nfake = []\ndefault = ["fake"]\n' >"$tmp/w/bardo/crates/input/input-broker-daemon/Cargo.toml"
bad 1 nofake; says "default features"
world; printf '[dependencies]\ncaller_identity = { version = "1", features = ["fake"] }\n' >"$tmp/w/bardo/crates/input/input-broker-daemon/Cargo.toml"
bad 1 nofake; says "caller_identity turns"
world; printf '[workspace.dependencies]\ncaller_identity = { version = "1", features = ["fake"] }\n' >"$tmp/w/bardo/crates/input/input-broker-daemon/Cargo.toml"
bad 1 nofake; says "workspace.dependencies"
world; printf '[features]\nfake = []\n[dev-dependencies]\ncaller_identity = { version = "1", features = ["fake"] }\n' >"$tmp/w/bardo/crates/input/input-broker-daemon/Cargo.toml"
ok nofake
world; printf '[dependencies]\ncaller_identity = { version = "1", features = ["dev-rows"] }\n' >"$tmp/w/bardo/crates/input/input-broker-daemon/Cargo.toml"
bad 1 nofake; says "caller_identity turns"
world; printf 'cargo_in "$d" x --features caller_identity/dev-rows\n' >>"$tmp/w/bardo/packaging/release/build.sh"
bad 1 nofake
world; printf '[features]\n[dev-dependencies]\ncaller_identity = { version = "1", features = ["dev-rows"] }\n' >"$tmp/w/bardo/crates/input/input-broker-daemon/Cargo.toml"
ok nofake

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

# The real tree is clean.
ok "$HERE/check-no-fake.sh"
echo "input-checks-selftest: ok"
