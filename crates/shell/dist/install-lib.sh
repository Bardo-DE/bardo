# shellcheck shell=bash disable=SC2034
# Shared by dist/install.sh and dist/uninstall.sh (sourced, never run). Where each piece of bardo-shell
# goes, and the helpers that print every step and honour --dry-run.
#
# Nothing here touches Plasma's files, Toshy's units or the key remap daemon's system files: bardo-shell is the files
# below, the binary, and one user file for the key remap daemon (the bardo-shell layer), and nothing else.

DIST="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$DIST/.." && pwd)"
TREE="$(cd "$ROOT/../.." && pwd)"   # the bardo tree

# Where it goes is settled by set_dests once the flags are read (--prefix, --destdir). PREFIX is the
# install prefix; DESTDIR a staging root put in front of every system path (empty: the real system).
# The paths set_dests makes are the logical ones; put and uninstall.sh prepend $DESTDIR.
PREFIX=/usr/local
DESTDIR=
NO_BUILD=0

# The bardo-shell session's key remap daemon layer. bardo-shell forwards this user file to the key remap daemon, which lays it over
# /etc/bardo/key-remap/key-remap.toml (the Plasma overlay, which Plasma never overrides); the file is copied
# when missing and never overwritten.
BARDO_KEY_REMAP_LAYER_SRC="$DIST/key-remap/key-remap.toml"
BARDO_KEY_REMAP_LAYER_DEST="${XDG_CONFIG_HOME:-$HOME/.config}/bardo/key-remap/key-remap.toml"

# DDC/CI for external monitors: a uaccess rule on the i2c buses and the i2c-dev module load. Put in
# place only where no ddcutil rule exists (ddcutil_rule_present).
DDC_RULE_SRC="$DIST/udev/60-bardo-shell-ddc.rules"
DDC_RULE_DEST=/etc/udev/rules.d/60-bardo-shell-ddc.rules
DDC_MODULES_SRC="$DIST/modules-load.d/bardo-shell-ddc.conf"
DDC_MODULES_DEST=/etc/modules-load.d/bardo-shell-ddc.conf

# set_dests: every destination, from $PREFIX.
set_dests() {
  BIN_DEST="$PREFIX/bin/bardo-shell"
  PAM_DEST=/etc/pam.d/bardo-shell
  # What the shell can be asked to do (`org.bardo.Shell`): intentd reads every manifest under
  # $XDG_DATA_DIRS/bardo/intents/, and /usr/local/share is in the default XDG_DATA_DIRS.
  INTENTS_DEST="$PREFIX/share/bardo/intents/org.bardo.Shell.toml"
  # The skills bardo-shell teaches the companion (dist/skills/<id>/): docket's loader reads every directory
  # under $XDG_DATA_DIRS/bardo/skills/.
  SKILLS_DEST="$PREFIX/share/bardo/skills"
  # bardo-shell's settings schema (every domain's keys, app `bardo-shell`): Settings (bardo-settings) reads every schema
  # under $XDG_DATA_DIRS/bardo/settings/. The binary writes it (`bardo-shell --write-schema <dir>`).
  SCHEMA_DEST="$PREFIX/share/bardo/settings/shell.settings.toml"
}
set_dests

DRY=0

# key_remap_rules FILE: each rule of a key remap daemon layer as "[profile] rule", sorted, one per line.
key_remap_rules() {
  awk '/^\[/ { profile = $0 } /^[[:space:]]*\{[[:space:]]*when/ { sub(/^[[:space:]]+/, ""); sub(/,[[:space:]]*$/, ""); print profile " " $0 }' "$1" | sort -u
}

# key_remap_rules_lacking: the rules dist has and the person's layer lacks, one per line.
key_remap_rules_lacking() {
  comm -23 <(key_remap_rules "$BARDO_KEY_REMAP_LAYER_SRC") <(key_remap_rules "$BARDO_KEY_REMAP_LAYER_DEST")
}

# parse_flags "$@": --dry-run, --prefix DIR, --destdir DIR, --no-build (only where the script sets
# HAS_BUILD=1), and the extra flags a script allows ($EXTRA_FLAGS, space-separated).
parse_flags() {
  local arg name
  while [ $# -gt 0 ]; do
    arg="$1"
    shift
    case "$arg" in
      --dry-run) DRY=1 ;;
      --no-build)
        [ "${HAS_BUILD:-0}" = 1 ] || { echo "unknown argument: $arg" >&2; usage >&2; exit 2; }
        NO_BUILD=1 ;;
      --prefix|--destdir)
        [ $# -gt 0 ] || { echo "$arg needs a directory" >&2; exit 2; }
        set_root_flag "$arg" "$1"
        shift ;;
      --prefix=*|--destdir=*) set_root_flag "${arg%%=*}" "${arg#*=}" ;;
      -h|--help) usage; exit 0 ;;
      *) if [[ " ${EXTRA_FLAGS:-} " == *" $arg "* ]]; then name="${arg#--}"; declare -g "FLAG_${name//-/_}=1"; else
           echo "unknown argument: $arg" >&2; usage >&2; exit 2; fi ;;
    esac
  done
  set_dests
}

# set_root_flag FLAG DIR: --prefix or --destdir, an absolute directory (a trailing slash is dropped).
set_root_flag() {
  local dir="${2%/}"
  case "$dir" in
    /*) ;;
    *) echo "$1 needs an absolute directory, got: $2" >&2; exit 2 ;;
  esac
  if [ "$1" = --prefix ]; then PREFIX="$dir"; else DESTDIR="$dir"; fi
}

step() { printf '\n==> %s\n' "$*"; }
note() { printf '    %s\n' "$*"; }

# run CMD...: print the command; run it unless --dry-run.
run() {
  printf '    $ %s\n' "$*"
  [ "$DRY" = 1 ] || "$@"
}

# as_root CMD...: run CMD as root through the helper need_root_helper chose. With a staging root
# ($DESTDIR) nothing escalates: that tree is the caller's. With no helper at all the command is
# printed now and listed again at the end (print_pending), for the person to run themselves.
as_root() {
  if [ -n "$DESTDIR" ]; then
    run "$@"
  elif [ "$ESC_MODE" = print ]; then
    PENDING+=("$(printf '%q ' "$@")")
    printf '    (as root, listed at the end) %s\n' "${PENDING[${#PENDING[@]}-1]}"
  else
    run "${ESC[@]}" "$@"
  fi
}

# put MODE SRC DEST: install SRC at DEST (under $DESTDIR) as root, unless DEST already is a copy of it.
put() {
  local mode="$1" src="$2" dest="$3"
  if [ -f "$DESTDIR$dest" ] && cmp -s "$src" "$DESTDIR$dest"; then
    note "unchanged: $DESTDIR$dest"
    return 0
  fi
  as_root install -D -m "$mode" "$src" "$DESTDIR$dest"
  INSTALLED+=("$dest")
}

# skills: the skill files' paths relative to dist/skills, one per line (evals are not installed).
skills() { (cd "$DIST/skills" && find . -type f -not -path '*/evals/*' | sed 's|^\./||' | sort); }

# check_lock: the build uses --locked, so the tree's Cargo.lock must already fit its manifests. Say so and stop when it does not.
check_lock() {
  if cargo_outside metadata --locked --format-version 1 --manifest-path "$TREE/Cargo.toml" >/dev/null 2>&1; then
    note "Cargo.lock fits the workspace"
    return 0
  fi
  echo "cargo metadata --locked fails: Cargo.lock does not fit the workspace; the exact cargo error: cargo metadata --locked --manifest-path $TREE/Cargo.toml" >&2
  return 1
}

refuse_root() {
  [ "$(id -u)" -ne 0 ] || {
    echo "run this as yourself, not root: it builds as you and escalates only to install" >&2
    exit 1
  }
}

ESC=()
ESC_MODE=run
PENDING=()
INSTALLED=()

# need_root_helper: choose how files get installed as root: $SUDO (a command line, word-split), else
# sudo, doas or pkexec, whichever is on PATH. With none, the commands are printed for the person to
# run (ESC_MODE=print). A staging root ($DESTDIR) needs none. Sets ESC and ESC_MODE.
need_root_helper() {
  [ -z "$DESTDIR" ] || return 0
  local tool
  if [ -n "${SUDO:-}" ]; then
    read -ra ESC <<<"$SUDO"
  else
    for tool in sudo doas pkexec; do
      if command -v "$tool" >/dev/null; then ESC=("$tool"); break; fi
    done
  fi
  if [ "${#ESC[@]}" -eq 0 ]; then
    ESC_MODE=print
    note "no sudo, doas or pkexec: the commands that need root are listed at the end for you to run (or set SUDO)"
    return 0
  fi
  [ "$DRY" = 1 ] && return 0
  command -v "${ESC[0]}" >/dev/null || { echo "${ESC[0]} (from SUDO) was not found" >&2; exit 1; }
  if [ "${ESC[0]}" = sudo ]; then
    note "sudo will ask for your password once"
    "${ESC[@]}" -v
  else
    note "${ESC[0]} will ask for your password when it needs it"
  fi
}

# print_pending: the root commands held back for want of a helper, ready to paste.
print_pending() {
  [ "${#PENDING[@]}" -gt 0 ] || return 0
  printf '\nRun these as root to finish:\n'
  printf '    %s\n' "${PENDING[@]}"
}

# ddcutil_rule_present ROOT: whether a ddcutil package (or the admin) already put a udev rule for the
# i2c buses under ROOT (empty ROOT: the real system). Then bardo-shell's own rule is not installed.
ddcutil_rule_present() {
  local root="$1" pattern
  for pattern in /usr/lib/udev/rules.d/60-ddcutil-i2c.rules /lib/udev/rules.d/60-ddcutil-i2c.rules /etc/udev/rules.d/60-ddcutil*.rules; do
    if compgen -G "$root$pattern" >/dev/null; then return 0; fi
  done
  return 1
}

# cargo_outside SUBCOMMAND ARGS...: run cargo from / with the tree named by --manifest-path in ARGS, so the
# build links with this system's own linker, whatever a working tree's .cargo/config.toml asks for; cargo reads that file (and
# rustup the toolchain file) from the directory it runs in, never from the manifest's. So the pinned
# toolchain is passed on through RUSTUP_TOOLCHAIN, and the one setting an install wants from that file
# (the system git client) is given again. The target directory stays explicit. No RUSTFLAGS.
cargo_outside() {
  local sub=$1 toolchain
  shift
  toolchain="$(sed -n 's/^channel *= *"\(.*\)"/\1/p' "$TREE/rust-toolchain.toml" 2>/dev/null | head -n 1)"
  (
    cd / || exit 1
    if [ -n "$toolchain" ] && command -v rustup >/dev/null; then export RUSTUP_TOOLCHAIN="$toolchain"; fi
    export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$TREE/target}"
    cargo "$sub" --config net.git-fetch-with-cli=true "$@"
  )
}
