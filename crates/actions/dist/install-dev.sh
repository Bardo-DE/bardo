#!/usr/bin/env bash
# install-dev.sh: install the agent stack for ONE user, from the bardo tree, with no root.
#
#   dist/install-dev.sh                       build (release, --locked) and install under ~/.local
#   dist/install-dev.sh --prefix DIR          binaries in DIR/bin (default: $PREFIX, else ~/.local)
#   dist/install-dev.sh --cloud               also let bardo-inferd reach the network (hosted models); without it inferd is offline
#   dist/install-dev.sh --dry-run             print every action; write nothing, build nothing
#   dist/install-dev.sh --uninstall           remove exactly what the last install put down
#
# The programs are crates of the tree this is in: the account, inference and memory daemons, the
# model catalog inferd reads, and the actions programs (bardo-actiond, bardo-companiond, bardo-readerd,
# bardo-actions-mcp, bardo-do, agent-eval), all built by one cargo run at the tree root. bardo-voiced (a skeleton), bardo-syncd and bardo-computer-use are not installed.
#
# What goes where (XDG_CONFIG_HOME and XDG_DATA_HOME are honoured):
#   $PREFIX/bin/                              the binaries
#   ~/.config/systemd/user/                   the units, ExecStart pointed at $PREFIX/bin; a drop-in
#                                             for bardo-inferd and bardo-accountd (see "dev-only changes")
#   ~/.local/share/dbus-1/services/           the activation files that start a service on demand, pointed at $PREFIX/bin
#   ~/.local/share/bardo/{intents,skills,settings}/   manifests, skills, settings schemas
#   ~/.local/share/bardo/actions/permissions/   policy presets
#   ~/.local/share/bardo/accounts/providers/  provider files (openrouter, ...): the AI keys and the identities accountd keeps
#   ~/.local/share/bardo/models/catalog/     the model catalogue, for inferd
#   ~/.config/bardo/actions/*.toml            default configs, ONLY where absent (never overwritten)
#   ~/.local/state/bardo/dev/install-dev.manifest   what was installed, for --uninstall
#
# The units' private-view protections are drop-ins (<unit>.service.d/sandbox.conf), installed unless
# this computer refuses them to user services (INSTALL_DEV_SANDBOX=auto, on or off). When they are
# left out, drop-ins an earlier install put down are removed. Dev-only change to them: when the
# prefix is under $HOME, the home folder is made read-only instead of hidden (hidden, the programs
# could not start from there, and bardo-companiond could not read its config and skills). The porter units need nothing: they make their own
# directories. --cloud installs the inference service's dist/inferd-cloud.conf as bardo-inferd.service.d/cloud.conf (network
# on); it is opt-in because local engines share inferd's sandbox, so an on-device-only setup keeps none.
# memoryd has no unit in memory/dist: memory/dbus/bardo-memoryd.service is its unit, started by D-Bus
# activation (org.bardo.Memory1.service names it), and that is what is installed.
#
# Needs root, and is printed but never run: /etc/bardo/accountd-callers.toml (the caller table accountd
# reads, at startup only) and inferd's own. After it, restart both: systemctl --user restart bardo-accountd bardo-inferd.
# AI keys are accountd's: add one with `accountd add <provider> --allow <app>` while accountd is
# stopped.
#
# INSTALL_DEV_BIN_DIR=DIR installs prebuilt binaries from DIR instead of building (the test does).
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOCKET="$(cd "$HERE/.." && pwd)"
TREE="$(cd "$DOCKET/../.." && pwd)"   # the bardo tree
CRATES="$TREE/crates"
PORTER="$CRATES/accounts"
INFERENCE="$CRATES/inference"
MEMORY="$CRATES/memory"
MODELS="$CRATES/models"
CUA="$CRATES/computer-use"

PREFIX="${PREFIX:-$HOME/.local}"
DRY=0
UNINSTALL=0
CLOUD=0

usage() { sed -n '2,/^set -euo/p' "${BASH_SOURCE[0]}" | sed '$d' | sed 's/^# \{0,1\}//'; }

while [ $# -gt 0 ]; do
  case "$1" in
    --prefix) [ $# -ge 2 ] || { echo "--prefix needs a directory" >&2; exit 2; }; PREFIX="$2"; shift ;;
    --prefix=*) PREFIX="${1#--prefix=}" ;;
    --dry-run) DRY=1 ;;
    --uninstall) UNINSTALL=1 ;;
    --cloud) CLOUD=1 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
  shift
done

[ "$(id -u)" -ne 0 ] || { echo "run this as yourself, not root" >&2; exit 1; }
case "$PREFIX" in /*) ;; *) echo "the prefix must be an absolute path: $PREFIX" >&2; exit 2 ;; esac

BIN="$PREFIX/bin"
CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}"
DATA="${XDG_DATA_HOME:-$HOME/.local/share}"
STATE="${XDG_STATE_HOME:-$HOME/.local/state}"
UNITS="$CONFIG/systemd/user"
DBUS="$DATA/dbus-1/services"
BARDO_CONFIG="$CONFIG/bardo/actions"
MANIFEST="$STATE/bardo/dev/install-dev.manifest"

# Binary -> the repo and package that builds it.
BINARIES=(
  "bardo-actiond docket action-daemon"
  "bardo-companiond docket companion-daemon"
  "bardo-readerd docket agent-reader-daemon"
  "bardo-actions-mcp docket action-mcp"
  "bardo-do docket action-cli"
  "agent-eval docket action-eval"
  "bardo-agent docket agent-acp-bin"
  "bardo-agent-net-forward docket agent-acp-bin"
  "bardo-agent-net-enter docket agent-acp-bin"
  "bardo-memoryd the memory service memory-daemon"
  "bardo-inferd the inference service inference-daemon"
  "bardo-accountd porter account-daemon"
)

# The installed set: "kind source dest" per line. Kinds: bin (copied, mode 755), unit and dbus
# (copied with the Exec rewritten), data (copied, mode 644), config (copied only if absent).
PLAN=()
plan() { PLAN+=("$1|$2|$3"); }

# The units' mount-namespace protections are a drop-in beside each unit (<unit>.d/sandbox.conf); they
# go in unless this system refuses them to user units (INSTALL_DEV_SANDBOX=on or off skips the test).
plan_sandbox() {
  [ "$SANDBOX_ON" = 1 ] || return 0
  [ -f "$1.d/sandbox.conf" ] || return 0
  plan sandbox "$1.d/sandbox.conf" "$UNITS/$(basename "$1").d/sandbox.conf"
}
plan_units() {
  local dir="$1" unit
  for unit in "$dir"/*.service; do
    [ -e "$unit" ] || continue
    case "$(basename "$unit")" in bardo-voiced.service|bardo-syncd.service|bardo-computer-use.service) continue ;; esac
    plan unit "$unit" "$UNITS/$(basename "$unit")"
    plan_sandbox "$unit"
  done
}
plan_dbus() {
  local dir="$1" file
  for file in "$dir"/*.service; do
    [ -e "$file" ] || continue
    case "$(basename "$file")" in org.bardo.Voice1.service|org.bardo.Sync1.service) continue ;; esac
    plan dbus "$file" "$DBUS/$(basename "$file")"
  done
}
plan_tree() { # kind, source dir, dest dir: every regular file under it
  local kind="$1" src="$2" dest="$3" file
  [ -d "$src" ] || return 0
  while IFS= read -r file; do plan "$kind" "$src/$file" "$dest/$file"; done \
    < <(cd "$src" && find . -type f | sed 's|^\./||' | sort)
}

build_plan() {
  local entry name repo pkg
  for entry in "${BINARIES[@]}"; do
    read -r name repo pkg <<<"$entry"
    plan bin "$(bin_source "$name" "$repo")" "$BIN/$name"
  done
  plan_units "$DOCKET/dist"; plan_units "$PORTER/dist"; plan_units "$INFERENCE/dist"
  plan unit "$MEMORY/dbus/bardo-memoryd.service" "$UNITS/bardo-memoryd.service"
  plan_sandbox "$MEMORY/dbus/bardo-memoryd.service"
  plan_dbus "$DOCKET/dist/dbus"; plan_dbus "$PORTER/dist/dbus"; plan_dbus "$INFERENCE/dist/dbus"
  plan dbus "$MEMORY/dbus/org.bardo.Memory1.service" "$DBUS/org.bardo.Memory1.service"
  plan_tree data "$DOCKET/dist/skills" "$DATA/bardo/skills"
  plan_tree data "$DOCKET/policy/presets" "$DATA/bardo/actions/permissions"
  local dir
  for dir in "$DOCKET" "$MEMORY" "$PORTER" "$INFERENCE" "$CUA"; do
    plan_tree data "$dir/dist/intents" "$DATA/bardo/intents"
    plan_tree data "$dir/dist/settings" "$DATA/bardo/settings"
  done
  plan data "$INFERENCE/dist/inference.settings.toml" "$DATA/bardo/settings/inference.settings.toml"
  plan data "$MEMORY/memory-settings/schema/memory.settings.toml" "$DATA/bardo/settings/memory.settings.toml"
  plan_tree data "$PORTER/providers" "$DATA/bardo/accounts/providers"
  plan_tree data "$MODELS/catalog" "$DATA/bardo/models/catalog"
  local cfg
  for cfg in "$DOCKET"/dist/*.toml; do
    # The callers files are the system's (sudo_step prints them): a fixed role (the shell's sheet_host)
    # comes from /etc/bardo alone, and the one in the person's config is ignored.
    case "$(basename "$cfg")" in voiced.toml|*-callers.toml) continue ;; esac
    plan config "$cfg" "$BARDO_CONFIG/$(basename "$cfg")"
  done
  if [ "$CLOUD" = 1 ]; then plan dropin "$INFERENCE/dist/inferd-cloud.conf" "$UNITS/bardo-inferd.service.d/cloud.conf"; fi
  plan config "$INFERENCE/dist/inferd.toml" "$CONFIG/bardo/inference/inferd.toml"
  plan config "$MEMORY/dbus/memoryd-callers.toml" "$CONFIG/bardo/memoryd/callers.toml"
}

bin_source() { # name repo -> the file to install
  if [ -n "${INSTALL_DEV_BIN_DIR:-}" ]; then echo "$INSTALL_DEV_BIN_DIR/$1"
  else echo "${CARGO_TARGET_DIR:-$TREE/target}/release/$1"; fi
}

# ---- the manifest: path -> "kind sha", kept sorted ----
declare -A OWNED=()
load_manifest() {
  [ -f "$MANIFEST" ] || return 0
  local kind sha path
  while IFS=$'\t' read -r kind sha path; do
    [ -z "$kind" ] || OWNED["$path"]="$kind $sha"
  done <"$MANIFEST"
}
sha_of() { sha256sum "$1" | cut -d' ' -f1; }
write_manifest() {
  make_dir "$(dirname "$MANIFEST")"
  local path tmp="$MANIFEST.new"
  : >"$tmp"
  for path in "${!OWNED[@]}"; do
    read -r kind sha <<<"${OWNED[$path]}"
    printf '%s\t%s\t%s\n' "$kind" "$sha" "$path"
  done | sort -t$'\t' -k3 >>"$tmp"
  mv "$tmp" "$MANIFEST"
}

say() { printf '%s\n' "$*"; }
act() { say "  $*"; }

# ---- rewriting ----
under_home() { case "$BIN" in "$HOME"|"$HOME"/*) return 0 ;; *) return 1 ;; esac; }
rewrite_exec() { # the packaged /usr/libexec/bardo/<name> is now $BIN/<name>
  sed -E -e "s|^(ExecStart=)/usr/libexec/bardo/|\1$BIN/|" -e "s|^(Exec=)/usr/libexec/bardo/|\1$BIN/|" \
    -e "s|^(ExecStart=)/usr/bin/bardo-|\1$BIN/bardo-|" -e "s|^(Exec=)/usr/bin/bardo-|\1$BIN/bardo-|"
}
# ProtectHome=yes and =tmpfs both hide the home folder, and the binary with it.
hide_home_off() { sed -E -e 's|^ProtectHome=yes$|ProtectHome=read-only|' -e 's|^ProtectHome=tmpfs$|ProtectHome=read-only|'; }
rewrite_unit() {
  local unit="$1"
  if under_home; then rewrite_exec | hide_home_off; else rewrite_exec; fi <"$unit"
}
render() { # kind source -> the text to install, on stdout
  case "$1" in
    unit) rewrite_unit "$2" ;;
    sandbox) if under_home; then hide_home_off <"$2"; else cat "$2"; fi ;;
    dbus) rewrite_exec <"$2" ;;
    *) cat "$2" ;;
  esac
}

# make_dir DIR: mkdir -p, remembering the directories this made (they are removed again when empty).
make_dir() {
  local dir="$1" missing=() d
  d="$dir"
  while [ ! -d "$d" ] && [ "$d" != "/" ]; do missing=("$d" "${missing[@]}"); d="$(dirname "$d")"; done
  [ "${#missing[@]}" -gt 0 ] || return 0
  mkdir -p "$dir"
  for d in "${missing[@]}"; do OWNED["$d"]="dir -"; done
}

install_file() { # kind source dest
  local kind="$1" src="$2" dest="$3" tmp mode=644
  [ "$kind" != bin ] || mode=755
  if [ "$kind" = config ] && [ -e "$dest" ]; then act "kept (yours): $dest"; return 0; fi
  if [ "$DRY" = 1 ]; then act "install -m $mode $src -> $dest"; return 0; fi
  make_dir "$(dirname "$dest")"
  tmp="$(mktemp "$dest.XXXXXX")"
  if [ "$kind" = bin ]; then cat "$src" >"$tmp"; else render "$kind" "$src" >"$tmp"; fi
  chmod "$mode" "$tmp"
  mv "$tmp" "$dest"
  act "installed: $dest"
  OWNED["$dest"]="$kind $(sha_of "$dest")"
}

# An earlier install-dev put its own drop-ins here; the units no longer need them.
drop_stale() {
  local path
  for path in "$UNITS/bardo-inferd.service.d/10-dev.conf" "$UNITS/bardo-accountd.service.d/10-dev.conf"; do
    [ -n "${OWNED[$path]:-}" ] || continue
    if [ "$DRY" = 1 ]; then act "remove stale $path"; else rm -f "$path"; unset 'OWNED[$path]'; act "removed stale: $path"; fi
  done
}

# With the private views off, a sandbox drop-in an earlier install put down would keep its unit
# from starting. Only paths this install owns, and none that the plan writes, are removed.
drop_stale_sandbox() {
  local path row kind src dest planned
  for path in "${!OWNED[@]}"; do
    case "$path" in *.service.d/sandbox.conf) ;; *) continue ;; esac
    planned=0
    for row in "${PLAN[@]}"; do
      IFS='|' read -r kind src dest <<<"$row"
      if [ "$dest" = "$path" ]; then planned=1; fi
    done
    [ "$planned" = 0 ] || continue
    if [ "$DRY" = 1 ]; then act "would remove the private-view drop-in an earlier install left: $path"
    else rm -f "$path"; unset 'OWNED[$path]'; act "removed (an earlier install left it): $path"; fi
  done
}

# The pinned toolchain may not be installed (rustup 1.28 and later does not fetch it by itself).
ensure_toolchain() {
  local toolchain
  toolchain="$(sed -n 's/^channel *= *"\(.*\)"/\1/p' "$TREE/rust-toolchain.toml" 2>/dev/null)" || true
  if [ -z "$toolchain" ] || ! command -v rustup >/dev/null 2>&1; then return 0; fi
  if rustup run "$toolchain" true >/dev/null 2>&1; then return 0; fi
  if [ "$DRY" = 1 ]; then act "would install the Rust version $toolchain with rustup (needs the internet)"; return 0; fi
  say "Installing the Rust version $toolchain with rustup (needs the internet)..."
  rustup toolchain install "$toolchain" --profile minimal
}

# Linked by this system's own linker, whatever a working tree's .cargo/config.toml asks for. Cargo reads that file (and
# rustup the toolchain file) from the directory it runs in, so the build runs from / with the tree's manifest
# named and the tree's pinned toolchain passed on. One run builds every program, into $CARGO_TARGET_DIR or <tree>/target.
release_build() { # cargo-args...
  local toolchain
  ensure_toolchain
  toolchain="$(sed -n 's/^channel *= *"\(.*\)"/\1/p' "$TREE/rust-toolchain.toml" 2>/dev/null)" || true
  (
    cd / &&
      if [ -n "$toolchain" ] && command -v rustup >/dev/null 2>&1; then
        RUSTUP_TOOLCHAIN="$toolchain"
        export RUSTUP_TOOLCHAIN
      fi &&
      CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$TREE/target}" cargo build --release --locked --manifest-path "$TREE/Cargo.toml" \
        --config net.git-fetch-with-cli=true "$@"
  )
}

build_all() {
  [ -z "${INSTALL_DEV_BIN_DIR:-}" ] || { say "using prebuilt binaries from $INSTALL_DEV_BIN_DIR"; return 0; }
  [ "$DRY" = 1 ] || command -v git >/dev/null 2>&1 || { echo "Building needs git (some libraries are downloaded with it). Install it with your system's package manager, then run this again." >&2; exit 1; }
  [ "$DRY" = 1 ] || say "The first build downloads code, so it needs the internet, and it takes a while."
  local packages=() entry name r pkg seen=" "
  for entry in "${BINARIES[@]}"; do
    read -r name r pkg <<<"$entry"
    case "$seen" in *" $pkg "*) continue ;; esac
    seen="$seen$pkg "
    packages+=(-p "$pkg")
  done
  if [ "$DRY" = 1 ]; then
    act "cargo build --release --locked --manifest-path $TREE/Cargo.toml ${packages[*]} (run from /, linked by this system's own linker)"
    ensure_toolchain
  else
    release_build "${packages[@]}"
  fi
}

# Fails before anything is written when a source is missing.
preflight() {
  local row kind src dest missing=0
  for row in "${PLAN[@]}"; do
    IFS='|' read -r kind src dest <<<"$row"
    if [ "$kind" = bin ] && [ "$DRY" = 1 ]; then continue; fi
    if [ ! -f "$src" ]; then echo "missing: $src" >&2; missing=1; fi
  done
  [ "$missing" = 0 ] || { echo "nothing was installed" >&2; exit 1; }
}

sudo_step() {
  cat <<EOF

One step needs root, and is not run here. accountd reads its caller table from
/etc/bardo/accountd-callers.toml (rows of \$XDG_CONFIG_HOME/bardo/accounts/callers.toml win, but the system
file is where bardo-inferd.service is named the AI broker, and where a machine that runs the agent
launcher writes its row, role agent_launcher). inferd reads who is calling from
/etc/bardo/inferd-callers.toml alone: a [callers] table in inferd.toml is ignored. They read them
only when they start:

  sudo install -D -m644 $PORTER/dist/callers.toml /etc/bardo/accountd-callers.toml
  sudo install -D -m644 $INFERENCE/dist/inferd-callers.toml /etc/bardo/inferd-callers.toml

The agent daemons name their callers the same way, by the unit behind a connection, from
/etc/bardo/<daemon>-callers.toml (the system's file alone can say which unit is the shell;
rows of ~/.config/bardo/actions/<daemon>-callers.toml are laid over it, but never give that role). Read
at startup, too:

  sudo install -D -m644 $DOCKET/dist/actiond-callers.toml /etc/bardo/actiond-callers.toml
  sudo install -D -m644 $DOCKET/dist/companiond-callers.toml /etc/bardo/companiond-callers.toml
  sudo install -D -m644 $DOCKET/dist/readerd-callers.toml /etc/bardo/readerd-callers.toml
  sudo install -D -m644 $DOCKET/dist/agent-acp-callers.toml /etc/bardo/agent-acp-callers.toml
  sudo install -D -m644 $MEMORY/dbus/memoryd-callers.toml /etc/bardo/memoryd-callers.toml

Then:  systemctl --user daemon-reload
       systemctl --user restart bardo-accountd bardo-inferd

To add an AI key (accountd holds them), with accountd stopped:
       systemctl --user stop bardo-accountd; bardo-accountd add <provider> --allow <app>; systemctl --user start bardo-accountd
EOF
}

# Do user units get a private view of the system here? Ubuntu 24.04 and later refuse it to programs
# without an AppArmor profile; a unit that asks for it then fails to start. The drop-ins are left
# out then (scripts/install.sh prints the remedy). Without a user manager to ask (over ssh, su) the
# answer is no, and INSTALL_DEV_SANDBOX=on turns it back on.
SANDBOX_ON=1
decide_sandbox() {
  case "${INSTALL_DEV_SANDBOX:-auto}" in
    on) return 0 ;;
    off) SANDBOX_ON=0; return 0 ;;
  esac
  if [ "$DRY" = 1 ]; then say "would test whether this computer allows the services' private views"; return 0; fi
  if ! command -v systemd-run >/dev/null 2>&1 || ! systemctl --user show-environment >/dev/null 2>&1; then
    SANDBOX_ON=0
    say "Private views for the services are off: this session cannot test whether this computer allows them. To turn them on later, run this again from a desktop terminal with INSTALL_DEV_SANDBOX=on."
    return 0
  fi
  systemd-run --user --quiet --collect -p PrivateTmp=yes -p ProtectSystem=strict --wait true >/dev/null 2>&1 || SANDBOX_ON=0
}

do_install() {
  load_manifest
  decide_sandbox
  build_plan
  say "install-dev: prefix $PREFIX, units in $UNITS"
  [ "$DRY" = 0 ] || say "(dry run: nothing is built or written)"
  build_all
  preflight
  local row kind src dest
  for row in "${PLAN[@]}"; do
    IFS='|' read -r kind src dest <<<"$row"
    install_file "$kind" "$src" "$dest"
  done
  drop_stale
  [ "$SANDBOX_ON" = 1 ] || drop_stale_sandbox
  [ "$DRY" = 1 ] || write_manifest
  [ "$DRY" = 1 ] || say "manifest: $MANIFEST"
  if [ "$SANDBOX_ON" = 0 ]; then say "note: this computer does not let user services have a private view of the files, so the private-view protections were left out; scripts/install.sh --help says how to turn them on"; fi
  if under_home; then say "note: bardo-companiond and bardo-readerd see your home folder read-only instead of not at all, because $BIN is under \$HOME"; fi
  sudo_step
}

do_uninstall() {
  [ -f "$MANIFEST" ] || { say "nothing to uninstall: no $MANIFEST"; return 0; }
  local path kind sha pass rows
  rows="$(sort -t$'\t' -k3 -r "$MANIFEST")"
  # Files first, then the manifest, then the directories this install made (deepest first, and
  # only when empty): what is left, an edited config or the person's own files, is theirs now.
  for pass in files dirs; do
    if [ "$pass" = dirs ] && [ "$DRY" = 0 ]; then rm -f "$MANIFEST"; fi
    while IFS=$'\t' read -r kind sha path; do
      [ -n "$kind" ] || continue
      if [ "$pass" = files ] && [ "$kind" = dir ]; then continue; fi
      if [ "$pass" = dirs ] && [ "$kind" != dir ]; then continue; fi
      if [ ! -e "$path" ]; then act "already gone: $path"; continue; fi
      case "$kind" in
        dir)
          if [ "$DRY" = 1 ]; then act "rmdir $path (if empty)"
          elif rmdir "$path" 2>/dev/null; then act "removed: $path/"
          else act "kept (not empty): $path"; fi ;;
        config)
          if [ "$(sha_of "$path")" != "$sha" ]; then act "kept (you edited it): $path"
          elif [ "$DRY" = 1 ]; then act "remove $path"
          else rm -f "$path"; act "removed: $path"; fi ;;
        *)
          if [ "$DRY" = 1 ]; then act "remove $path"
          else rm -f "$path"; act "removed: $path"; fi ;;
      esac
    done <<<"$rows"
  done
  say "Then:  systemctl --user daemon-reload   (and stop what is running: systemctl --user stop bardo-actiond bardo-companiond bardo-readerd bardo-memoryd bardo-inferd bardo-accountd)"
  say "Not removed: /etc/bardo/accountd-callers.toml, /etc/bardo/inferd-callers.toml and /etc/bardo/*-callers.toml (sudo rm them), your accounts and keys, and ~/.local/state/bardo."
}

if [ "$UNINSTALL" = 1 ]; then do_uninstall; else do_install; fi
