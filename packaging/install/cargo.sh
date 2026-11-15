# Building the bardo tree for an install or a release (sourced, never run). Always with this system's own
# linker: we never ask the person for one, and we never install one.
#
# The tree is one cargo workspace with one target directory. Cargo reads .cargo/config.toml, and rustup reads
# rust-toolchain.toml, from the directory cargo runs in, never from the manifest's. So a build runs from / with
# the tree named by --manifest-path (the tree's own cargo config, which is for working on it, is never read), the
# pinned toolchain goes on in RUSTUP_TOOLCHAIN, and the one setting an install wants from that config is given
# again. No RUSTFLAGS. The target directory is explicit.
#
#   tree_root                  the bardo tree: $BARDO_TREE, else the parent of packaging/
#   build_tree CARGO-ARGS...   cargo build --release --locked of the tree's workspace, run from /;
#                              CARGO_TARGET_DIR if set, else <tree>/target

tree_root() {
  if [ -n "${BARDO_TREE:-}" ]; then printf '%s' "${BARDO_TREE%/}"; else (cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd); fi
}

# tree_toolchain DIR: the channel DIR/rust-toolchain.toml pins (empty when it pins none).
tree_toolchain() {
  sed -n 's/^channel *= *"\(.*\)"/\1/p' "$1/rust-toolchain.toml" 2>/dev/null | head -n 1
}

build_tree() {
  local dir toolchain
  dir=$(tree_root)
  toolchain=$(tree_toolchain "$dir")
  (
    cd / || exit 1
    if [ -n "$toolchain" ] && command -v rustup >/dev/null 2>&1; then export RUSTUP_TOOLCHAIN=$toolchain; fi
    export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$dir/target}"
    cargo build --release --locked --manifest-path "$dir/Cargo.toml" --config net.git-fetch-with-cli=true "$@"
  )
}

# part_packages PART: the cargo packages whose programs PART installs (none: nothing to build).
part_packages() {
  case $1 in
    porter) echo accountd ;;
    ferry) echo syncd ferry-mountd ;;
    steward) echo inferd ;;
    stoker) echo engine-install ;;
    almanac) echo memoryd ;;
    docket) echo intentd companiond readerd actions-mcp docket-cli docket-acp-bin ;;
    cua) echo cuad ;;
    detent) echo detent ;;
    casement) echo casement casementctl ;;
    sill) echo sill ;;
    transom) echo transom ;;
    turnstile) echo turnstile ;;
    keycap) echo keycap ;;
    files) echo files ;;
    palmrest) echo palmrest ;;
  esac
}


# build_parts PART...: one cargo run (one target directory) for every package of the parts.
build_parts() {
  local args=() part pkg
  for part in "$@"; do
    for pkg in $(part_packages "$part"); do args+=(-p "$pkg"); done
  done
  [ ${#args[@]} -eq 0 ] || build_tree "${args[@]}"
}
