#!/usr/bin/env bash
# The three input and caller checks against generated trees: a clean one passes, and each fault they exist for fails.
# Reads the real scripts; builds a made-up tree (packaging/ and crates/<name>/) in a scratch folder; touches nothing else.
#
#   input-checks-selftest.sh
# Exit 0 pass, 1 a check failed.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REAL="$(cd "$HERE/.." && pwd)"
tmp=$(mktemp -d)
trap 'rm -rf -- "${tmp:?}"' EXIT
fail() { echo "input-checks-selftest: $*" >&2; exit 1; }

# A fresh world: $tmp/w/bardo/packaging (the three scripts, a table, build scripts) beside made-up crates and a peephole.
world() {
  rm -rf -- "${tmp:?}/w"
  local d=$tmp/w/bardo/packaging s=$tmp/w/bardo/crates
  mkdir -p "$d/gate" "$d/dist" "$d/release" "$d/install" "$d/.github/workflows" "$d/harness" \
    "$s/peephole/peephole-core/src" "$s/turnstile/dist" "$s/keycap/dist" "$s/palmrest/dist" "$s/sill/dist"
  cp "$HERE/check-callers-table.sh" "$HERE/check-no-fake.sh" "$HERE/check-no-uinput-access.sh" "$d/gate/"
  cp "$REAL/dist/desktop-callers.toml" "$d/dist/"
  printf '#!/usr/bin/env bash\ncargo_in "$d" turnstile -p input-broker-daemon\nput_system 644 "$t/dist/72-turnstile.rules" /etc/udev/rules.d/72-turnstile.rules\nput_system 644 "$t/dist/turnstile.service" /usr/lib/systemd/system/turnstile.service\nput_system 644 "$k/dist/keycap.service" /usr/lib/systemd/system/keycap.service\n' >"$d/release/build.sh"
  printf '#!/usr/bin/env bash\n' >"$d/install.sh"
  printf '#!/usr/bin/env bash\necho "644 $x/dist/72-turnstile.rules /etc/udev/rules.d/72-turnstile.rules"\n' >"$d/install/system.sh"
  # peephole's role list, copied from the real one when it is in this tree, else the eighteen written out
  if [ -f "$REAL/../crates/peephole/peephole-core/src/roles.rs" ]; then cp "$REAL/../crates/peephole/peephole-core/src/roles.rs" "$s/peephole/peephole-core/src/"
  else
    {
      for r in SHELL PORTAL PORTAL_FRONTEND COMPOSITOR AGENT_TIER SETTINGS UPDATER UPDATER_CLI PAD_DAEMON KEYMAP_DAEMON ACCOUNTS INFERENCE SYNC ROUTER COMPANION MEMORY SANDBOX_LAUNCHER MANAGER; do
        printf 'pub const %s: RoleName = RoleName::from_static("%s");\n' "$r" "$(echo "$r" | tr A-Z a-z)"
      done
      printf 'pub const ALL: [RoleName; 18] = [SHELL, PORTAL, PORTAL_FRONTEND, COMPOSITOR, AGENT_TIER, SETTINGS, UPDATER, UPDATER_CLI, PAD_DAEMON, KEYMAP_DAEMON, ACCOUNTS, INFERENCE, SYNC, ROUTER, COMPANION, MEMORY, SANDBOX_LAUNCHER, MANAGER];\n'
    } >"$s/peephole/peephole-core/src/roles.rs"
  fi
  printf 'KERNEL=="uinput", SUBSYSTEM=="misc", MODE="0600", GROUP="root", TAG-="uaccess", OPTIONS+="static_node=uinput"\n' >"$s/turnstile/dist/72-turnstile.rules"
  printf '[Service]\nExecStart=/usr/local/bin/turnstile\nDeviceAllow=/dev/uinput rw\n' >"$s/turnstile/dist/turnstile.service"
  printf '[Service]\nExecStart=/usr/local/bin/keycap\nDeviceAllow=char-input rw\n' >"$s/keycap/dist/keycap.service"
  printf '[package]\nname = "x"\n[features]\nfake = []\ndefault = []\n[dev-dependencies]\npeephole = { version = "1", features = ["fake"] }\n' >"$s/turnstile/Cargo.toml"
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
world; mutate 'text = text.replace("unit = \"sill.service\"\n", "")'
bad 1 callers; says "no unit, and 'shell' is a service"
world; mutate 'text = text.replace("\"/usr/bin/sill\"", "\"usr/bin/sill\"")'
bad 1 callers; says "is not an absolute path"
world; mutate 'text = text.replace("\"/usr/bin/sill\"", "\"/usr/bin/../sill\"")'
bad 1 callers; says "empty, . or .. segment"
world; mutate 'text = text.replace("\"/usr/bin/sill\"", "\"/usr/*/*/sill\"")'
ok callers
world; mutate 'text = text.replace("\"/usr/bin/sill\"", "\"/usr/*x*/sill\"")'
bad 1 callers; says "more than one"
world; mutate 'text = text.replace("unit = \"casement.service\"", "unit = \"sill.service\"")'
bad 1 callers; says "also in row"
world; mutate 'i = text.index("# ...and also by a launcher"); j = text.index("# The software-update service."); text = text[:i] + text[j:]'
bad 1 callers; says "settings has no row without a unit"
world; mutate 'text = text.replace("[[peer]]\napp  = \"org.quire.Shell\"", "[[peer]]\nstray = 1\napp  = \"org.quire.Shell\"")'
bad 1 callers; says "unknown key"
world; printf '[Service]\nExecStart=/srv/somewhere/sill run\n' >"$tmp/w/bardo/crates/sill/dist/sill.service"
bad 1 callers; says "ExecStart /srv/somewhere/sill"
world; printf '[Service]\nExecStart=/usr/bin/sill run\n' >"$tmp/w/bardo/crates/sill/dist/sill.service"
ok callers; says "2 unit(s) compared"
world; rm -rf "$tmp/w/bardo/crates/peephole"
bad 2 callers --peephole "$tmp/w/none"; says "no peephole crate"

# --- no stand-in
world
ok nofake
world; printf 'cargo_in "$d" x --features fake\n' >>"$tmp/w/bardo/packaging/release/build.sh"
bad 1 nofake; says "build.sh:6"
world; printf 'cargo build --all-features\n' >>"$tmp/w/bardo/packaging/install.sh"
bad 1 nofake; says "all features"
world; printf '# cargo build --features fake (a comment)\n' >>"$tmp/w/bardo/packaging/install.sh"
ok nofake
world; printf '      - run: cargo build --release -p input-broker-daemon --features turnstile/fake\n' >"$tmp/w/bardo/packaging/.github/workflows/release.yml"
bad 1 nofake
world; printf '[features]\nfake = []\ndefault = ["extra"]\nextra = ["dep/fake"]\n' >"$tmp/w/bardo/crates/turnstile/Cargo.toml"
bad 1 nofake; says "default features"
world; printf '[features]\nfake = []\ndefault = ["fake"]\n' >"$tmp/w/bardo/crates/turnstile/Cargo.toml"
bad 1 nofake; says "default features"
world; printf '[dependencies]\npeephole = { version = "1", features = ["fake"] }\n' >"$tmp/w/bardo/crates/turnstile/Cargo.toml"
bad 1 nofake; says "peephole turns"
world; printf '[workspace.dependencies]\npeephole = { version = "1", features = ["fake"] }\n' >"$tmp/w/bardo/crates/turnstile/Cargo.toml"
bad 1 nofake; says "workspace.dependencies"
world; printf '[features]\nfake = []\n[dev-dependencies]\npeephole = { version = "1", features = ["fake"] }\n' >"$tmp/w/bardo/crates/turnstile/Cargo.toml"
ok nofake
world; printf '[dependencies]\npeephole = { version = "1", features = ["dev-rows"] }\n' >"$tmp/w/bardo/crates/turnstile/Cargo.toml"
bad 1 nofake; says "peephole turns"
world; printf 'cargo_in "$d" x --features peephole/dev-rows\n' >>"$tmp/w/bardo/packaging/release/build.sh"
bad 1 nofake
world; printf '[features]\n[dev-dependencies]\npeephole = { version = "1", features = ["dev-rows"] }\n' >"$tmp/w/bardo/crates/turnstile/Cargo.toml"
ok nofake

# --- nothing opens /dev/uinput
world
ok uinput; says "3 rule or service"
world; printf 'KERNEL=="uinput", GROUP="input", MODE="0660", OPTIONS+="static_node=uinput"\n' >"$tmp/w/bardo/crates/turnstile/dist/72-turnstile.rules"
bad 1 uinput; says 'GROUP="root"'
world; printf 'KERNEL=="uinput", MODE="0600", GROUP="root", TAG+="uaccess"\n' >"$tmp/w/bardo/crates/turnstile/dist/72-turnstile.rules"
bad 1 uinput; says "uaccess"
world; printf 'KERNEL=="uinput", MODE="0600", GROUP="root", TAG-="uaccess"\n' >"$tmp/w/bardo/crates/turnstile/dist/72-turnstile.rules"
ok uinput
world; printf '[Service]\nDeviceAllow=/dev/uinput rw\n' >>"$tmp/w/bardo/crates/keycap/dist/keycap.service"
bad 1 uinput; says "other than the input service"
world; printf 'put_system 644 "$d/dist/70-keycap.rules" /etc/udev/rules.d/70-keycap.rules\n' >>"$tmp/w/bardo/packaging/release/build.sh"
bad 1 uinput; says "70-keycap.rules"
world; printf 'install -Dm644 "$k/dist/keycap.uinput.conf" "$UNITDIR/keycap.service.d/10-uinput.conf"\n' >>"$tmp/w/bardo/packaging/install/system.sh"
bad 1 uinput; says "keycap.uinput.conf"
world; printf 'note "WARNING: /etc/udev/rules.d/70-keycap.rules from an earlier install"\n' >>"$tmp/w/bardo/packaging/install/system.sh"
ok uinput
world; mkdir -p "$tmp/stage/usr/lib/systemd/system/keycap.service.d" "$tmp/stage/etc/udev/rules.d"
printf '[Service]\nDeviceAllow=/dev/uinput rw\n' >"$tmp/stage/usr/lib/systemd/system/keycap.service.d/10-uinput.conf"
bad 1 uinput --stage "$tmp/stage"; says "drop-in"
rm -rf "$tmp/stage"; mkdir -p "$tmp/stage/etc/udev/rules.d"; cp "$tmp/w/bardo/crates/turnstile/dist/72-turnstile.rules" "$tmp/stage/etc/udev/rules.d/"
ok uinput --stage "$tmp/stage"
printf 'KERNEL=="uinput", GROUP="input", MODE="0660"\n' >"$tmp/stage/etc/udev/rules.d/70-keycap.rules"
bad 1 uinput --stage "$tmp/stage"

# The real tree is clean.
ok "$HERE/check-no-fake.sh"
echo "input-checks-selftest: ok"
