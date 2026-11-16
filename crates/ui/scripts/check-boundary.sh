#!/usr/bin/env bash
# Crate boundaries, mechanically enforced.
#
# `cargo tree -i <dep>` exits 101 when the dependency is absent, which is precisely the state
# we want. Checking the exit status would therefore fail whenever the boundary holds, so we
# check for OUTPUT instead: any line naming the dependency is a leak.
#
# Each rule is "<crate>: <forbidden deps...>". Forbidden names are exact package names; a
# family such as blitz-* is spelled out member by member.
set -uo pipefail
cd "$(dirname "$0")/.."

# ui-core is plain data and maths, so sill's pure crates can use it: not even dioxus. ui-style and
# ui-motion name dioxus only through their `dioxus` feature (ui-style: scope, scale, task, busy,
# `Glyph`; ui-motion: the hooks, timers and event conversions), which neither enables by default,
# so a compositor or a crate that only reads settings links no Dioxus; ui-settings has the same
# feature for the environment that provides them. ui-behaviour holds the input machines a
# compositor runs too, so like ui-core it reaches no Dioxus and no renderer. ds is renderer-free
# and effect-free; ui-settings does I/O but never renders, and takes a Spawner instead of naming a
# runtime. ui-helpers (missing helpers: probe PATH, ask PackageKit) is I/O too: zbus only, no
# renderer, no runtime, and no quire crate but ui-peer (the deadline on each call to a peer).
# chordkit (the keymap core) is pure and ui-core and ds take it; its I/O sources (keymap-kde, keymap-gnome
# read a settings file) are named by ui-blitz alone, which injects them as the launch's default KeySource.
# ui-desktop (the capability probe) names zbus only through its opt-in `dbus` feature and dioxus
# only through `dioxus`, so a build without them links neither; the rule below checks the default
# build. Its all-features edges are none of ours.
# ui-settings-page draws a schema (ui-settings' types) with ds's rows and controls; neither of those
# may name the other, so the page is a crate of its own, and no D-Bus client or runtime comes with it.
# ui-blitz reaches D-Bus only through its opt-in `print` and `menus` features, so an app that
# never prints or exports a menu builds no D-Bus client for it, and pdfrum only through `pdf`; ui-harness inherits both rules
# (it turns on `pdf` only for its own `pdf` feature). pdfrum-anyrender (a git dependency
# from the pdfrum repo) is not an in-tree edge, so it is not listed there. wgpu is a renderer
# dependency like anyrender: only ui-blitz (the texture layer's device) and ui-harness (the
# hybrid painter) name it; every other crate stays device-free.
# ui-extensions (Extensions: the folder scan, the checks, on and off, the data capabilities) is a library
# with no UI, no runtime and no bus: it reads settings through ui-settings, files through ui-helpers'
# capped reader, and manifests through bayonet (a git dependency, not an in-tree edge). ui-settings must
# never name it back (the Provided key kind knows nothing of what is installed).
# ui-emoji finds the colour emoji face in a fontique collection for ui-blitz and for temor's terminal,
# which has no toolkit: only fontique, so no renderer, no toolkit, no D-Bus and not even parley.
RULES=(
  "ui-core: keymap-kde keymap-gnome zbus notify tokio winit dioxus blitz blitz-dom blitz-paint blitz-traits blitz-html blitz-net blitz-shell stylo_taffy dioxus-native dioxus-native-dom anyrender anyrender_vello anyrender_vello_cpu anyrender_vello_hybrid anyrender_skia anyrender_svg pdfrum-anyrender pdfrum-edit wgpu wgpu_context"
  "ui-behaviour: zbus notify tokio winit dioxus blitz blitz-dom blitz-paint blitz-traits blitz-html blitz-net blitz-shell stylo_taffy dioxus-native dioxus-native-dom anyrender anyrender_vello anyrender_vello_cpu anyrender_vello_hybrid anyrender_skia anyrender_svg pdfrum-anyrender pdfrum-edit wgpu wgpu_context"
  "ui-intents: zbus notify tokio winit dioxus blitz blitz-dom blitz-paint blitz-traits blitz-html blitz-net blitz-shell stylo_taffy dioxus-native dioxus-native-dom anyrender anyrender_vello anyrender_vello_cpu anyrender_vello_hybrid anyrender_skia anyrender_svg pdfrum-anyrender pdfrum-edit wgpu wgpu_context"
  "ui-style: dioxus zbus notify tokio winit blitz blitz-dom blitz-paint blitz-traits blitz-html blitz-net blitz-shell stylo_taffy dioxus-native dioxus-native-dom anyrender anyrender_vello anyrender_vello_cpu anyrender_vello_hybrid anyrender_skia anyrender_svg pdfrum-anyrender pdfrum-edit wgpu wgpu_context"
  "ui-motion: dioxus zbus notify tokio winit blitz blitz-dom blitz-paint blitz-traits blitz-html blitz-net blitz-shell stylo_taffy dioxus-native dioxus-native-dom anyrender anyrender_vello anyrender_vello_cpu anyrender_vello_hybrid anyrender_skia anyrender_svg pdfrum-anyrender pdfrum-edit wgpu wgpu_context"
  "ui-lint: zbus notify tokio winit dioxus blitz blitz-dom blitz-paint blitz-traits blitz-html blitz-net blitz-shell stylo_taffy dioxus-native dioxus-native-dom anyrender anyrender_vello anyrender_vello_cpu anyrender_vello_hybrid anyrender_skia anyrender_svg pdfrum-anyrender pdfrum-edit wgpu wgpu_context"
  "ui-shell: zbus notify tokio winit blitz blitz-dom blitz-paint blitz-traits blitz-html blitz-net blitz-shell stylo_taffy dioxus-native dioxus-native-dom anyrender anyrender_vello anyrender_vello_cpu anyrender_vello_hybrid anyrender_skia anyrender_svg pdfrum-anyrender pdfrum-edit wgpu wgpu_context"
  "ds: keymap-kde keymap-gnome zbus notify tokio winit blitz blitz-dom blitz-paint blitz-traits blitz-html blitz-net blitz-shell stylo_taffy dioxus-native dioxus-native-dom anyrender anyrender_vello anyrender_vello_cpu anyrender_vello_hybrid anyrender_skia anyrender_svg pdfrum-anyrender pdfrum-edit wgpu wgpu_context"
  "ui-settings: dioxus tokio blitz blitz-dom blitz-paint blitz-traits blitz-html blitz-net blitz-shell stylo_taffy dioxus-native dioxus-native-dom anyrender anyrender_vello anyrender_vello_cpu anyrender_vello_hybrid anyrender_skia anyrender_svg pdfrum-anyrender pdfrum-edit wgpu wgpu_context"
  "ui-settings-page: zbus tokio winit blitz blitz-dom blitz-paint blitz-traits blitz-html blitz-net blitz-shell stylo_taffy dioxus-native dioxus-native-dom anyrender anyrender_vello anyrender_vello_cpu anyrender_vello_hybrid anyrender_skia anyrender_svg pdfrum-anyrender pdfrum-edit wgpu wgpu_context"
  "ui-extensions: dioxus zbus tokio winit blitz blitz-dom blitz-paint blitz-traits blitz-html blitz-net blitz-shell stylo_taffy dioxus-native dioxus-native-dom anyrender anyrender_vello anyrender_vello_cpu anyrender_vello_hybrid anyrender_skia anyrender_svg pdfrum-anyrender pdfrum-edit wgpu wgpu_context"
  "ui-helpers: dioxus tokio blitz blitz-dom blitz-paint blitz-traits blitz-html blitz-net blitz-shell stylo_taffy dioxus-native dioxus-native-dom anyrender anyrender_vello anyrender_vello_cpu anyrender_vello_hybrid anyrender_skia anyrender_svg pdfrum-anyrender pdfrum-edit wgpu wgpu_context"
  "ui-desktop: zbus dioxus tokio blitz blitz-dom blitz-paint blitz-traits blitz-html blitz-net blitz-shell stylo_taffy dioxus-native dioxus-native-dom anyrender anyrender_vello anyrender_vello_cpu anyrender_vello_hybrid anyrender_skia anyrender_svg pdfrum-anyrender pdfrum-edit wgpu wgpu_context"
  "ui-emoji: zbus notify tokio winit dioxus blitz blitz-dom blitz-paint blitz-traits blitz-html blitz-net blitz-shell stylo_taffy dioxus-native dioxus-native-dom anyrender anyrender_vello anyrender_vello_cpu anyrender_vello_hybrid anyrender_skia anyrender_svg pdfrum-anyrender pdfrum-edit wgpu wgpu_context parley"
  "ui-blitz: zbus memfd pdfrum-anyrender pdfrum pdfrum-edit"
  "ui-harness: zbus memfd pdfrum-anyrender pdfrum pdfrum-edit"
  "ui-peer: zbus notify tokio winit dioxus blitz blitz-dom blitz-paint blitz-traits blitz-html blitz-net blitz-shell stylo_taffy dioxus-native dioxus-native-dom anyrender anyrender_vello anyrender_vello_cpu anyrender_vello_hybrid anyrender_skia anyrender_svg pdfrum-anyrender pdfrum-edit wgpu wgpu_context"
  "ui-core-derive: dioxus zbus tokio"
  "ui-settings-derive: dioxus zbus tokio"
)
fail=0

for rule in "${RULES[@]}"; do
  crate="${rule%%:*}"
  read -r -a forbidden <<<"${rule#*:}"
  # A crate that cargo cannot find would make every check below pass vacuously.
  if ! cargo tree -p "$crate" --depth 0 >/dev/null 2>&1; then
    echo "ERROR: cargo tree cannot resolve $crate; the boundary was not checked"
    fail=1
    continue
  fi
  leaked=0
  for dep in "${forbidden[@]}"; do
    if [ -n "$(cargo tree -p "$crate" -i "$dep" -e normal,build 2>/dev/null)" ]; then
      echo "LEAK: $crate depends on $dep"
      cargo tree -p "$crate" -i "$dep" -e normal,build 2>/dev/null | head -20
      leaked=1
      fail=1
    fi
  done
  if [ "$leaked" -eq 0 ]; then
    echo "boundary holds: $crate reaches none of ${forbidden[*]}"
  fi
done

# The allowed edges between our own crates (ARCHITECTURE.md section 1): each crate's direct normal
# and build dependencies that live in this workspace, and nothing else. A dependency on a crate
# not listed here is a leak; so is one the crate no longer has, so the table stays exact.
EDGES=(
  "ui-core-derive:"
  "ui-settings-derive:"
  "ui-core: ui-core-derive"
  "ui-style: ui-core ui-core-derive"
  "ui-motion: ui-core ui-style"
  "ui-lint: ui-core ui-style"
  "ui-intents: ui-core"
  "ui-behaviour: ui-core"
  "ds: ui-core ui-intents ui-motion ui-style"
  "ui-shell: ds ui-core ui-motion ui-style"
  "ui-settings: ui-core ui-helpers ui-peer ui-style ui-settings-derive"
  "ui-settings-page: ds ui-settings"
  "ui-extensions: ui-core ui-helpers ui-settings ui-style"
  "ui-peer:"
  "ui-helpers: ui-peer"
  "ui-desktop: ui-peer"
  "ui-blitz: blitz-embed ds ui-desktop ui-helpers ui-peer ui-emoji"
  "ui-harness: blitz-embed ds ui-blitz ui-core"
  "ds-gallery: ds ui-core ui-harness ui-lint ui-blitz ui-settings ui-shell"
  "ui-conformance:"
  "icons: ds ui-style ui-settings"
)
for edge in "${EDGES[@]}"; do
  crate="${edge%%:*}"
  read -r -a allowed <<<"${edge#*:}"
  found=$(cargo tree -p "$crate" --depth 1 -e normal,build --prefix none --all-features 2>/dev/null \
    | grep '(/' | awk '{print $1}' | grep -vx "$crate" | sort -u | tr '\n' ' ')
  want=$(printf '%s\n' "${allowed[@]}" | grep . | sort -u | tr '\n' ' ')
  if [ "$found" != "$want" ]; then
    echo "EDGE: $crate depends on [${found% }], the table allows [${want% }]"
    fail=1
  else
    echo "edges hold: $crate depends on [${found% }]"
  fi
done

# ui-conformance is test-only: no normal or build edge at all (the EDGES row above), and its
# dev-dependencies are the crates it tests through, never the lower crates directly.
DEV_EDGES=(
  "ui-conformance: ds ui-shell ui-lint ui-settings ui-blitz ui-harness"
)
for edge in "${DEV_EDGES[@]}"; do
  crate="${edge%%:*}"
  read -r -a allowed <<<"${edge#*:}"
  found=$(cargo tree -p "$crate" --depth 1 -e dev --prefix none --all-features 2>/dev/null \
    | grep '(/' | awk '{print $1}' | grep -vx "$crate" | sort -u)
  outside=$(comm -23 <(printf '%s\n' "$found") <(printf '%s\n' "${allowed[@]}" | sort -u))
  if [ -n "$outside" ]; then
    echo "DEV EDGE: $crate dev-depends on [$(echo $outside)], the table allows [${allowed[*]}]"
    fail=1
  else
    echo "dev edges hold: $crate dev-depends on [$(echo $found)]"
  fi
done

# The layers inside ds, the one crate that still has any: the host seams, the hooks over them, the
# overlay stack, the root, the components and assembly on top. A file in a layer may name
# (`crate::<module>`) only its own layer and the ones below it; mail's own components
# (components::app) are named by nothing but themselves and assembly. Doc links count. The layers
# that became crates (ui-core, ui-style, ui-motion, ui-lint, ui-shell) are kept by EDGES above and
# by cargo itself: a `pub(crate)` item cannot be named across a crate boundary.
DS=crates/ds/src
LAYERS=(
  "host: focus edit file_drop keys machine spell window stack root components assembly"
  "focus edit file_drop keys machine spell window: stack root components assembly"
  "stack: root components assembly"
  "root: components assembly"
  "components: assembly components::app"
)
layered=0
for rule in "${LAYERS[@]}"; do
  read -r -a dirs <<<"${rule%%:*}"
  read -r -a above <<<"${rule#*:}"
  pattern="crate::($(IFS='|'; echo "${above[*]}"))\\b"
  for dir in "${dirs[@]}"; do
    [ -d "$DS/$dir" ] || { echo "ERROR: $DS/$dir is missing; the layer was not checked"; fail=1; continue; }
    if [ "$dir" = components ]; then
      hits=$(grep -rnE "$pattern" "$DS/$dir" --exclude-dir=app || true)
    else
      hits=$(grep -rnE "$pattern" "$DS/$dir" | grep -v "^$DS/components/app/" || true)
    fi
    if [ -n "$hits" ]; then
      echo "LAYER: $dir names a layer above it:"
      echo "$hits" | head -20
      layered=1
      fail=1
    fi
  done
done

# The component groups, lowest first: a group names only itself and the groups below it.
COMPONENT_GROUPS=(content controls overlays lists fields menus editor chrome companion app)
for i in "${!COMPONENT_GROUPS[@]}"; do
  group="${COMPONENT_GROUPS[$i]}"
  above=("${COMPONENT_GROUPS[@]:$((i + 1))}")
  [ "${#above[@]}" -gt 0 ] || continue
  [ -d "$DS/components/$group" ] || { echo "ERROR: $DS/components/$group is missing; the group was not checked"; fail=1; continue; }
  pattern="crate::components::($(IFS='|'; echo "${above[*]}"))\\b"
  hits=$(grep -rnE "$pattern" "$DS/components/$group" || true)
  if [ -n "$hits" ]; then
    echo "GROUP: components::$group names a group above it:"
    echo "$hits" | head -20
    layered=1
    fail=1
  fi
done
if [ "$layered" -eq 0 ]; then
  echo "layers hold: host < hooks < stack < root < components < assembly; component groups in order"
fi

# The crate roots of ds and ui-shell hold declarations, `prelude` and the curated roots (the
# stylesheet assembly) only: every other name has one path, its home module or the prelude. A new
# `pub use` at either root introduces a name outside the allowed set below and fails here.
# The three facades (`pub use ds_core as base;`, `ds_style as style`, `ds_motion as motion`) are the
# only crate re-exports.
ROOT_ALLOWED_DS="pub use crate assembly selectors kit KIT kits stylesheet component_sheets ds_core base ds_style style ds_motion motion as"
ROOT_ALLOWED_DS_SHELL="pub use crate kit KIT kits stylesheet component_sheets"
for root in "ds:$ROOT_ALLOWED_DS" "ui-shell:$ROOT_ALLOWED_DS_SHELL"; do
  crate="${root%%:*}"
  read -r -a allowed <<<"${root#*:}"
  lib="crates/$crate/src/lib.rs"
  # Every `pub use` statement, comments removed, reduced to its identifier tokens.
  tokens=$(sed 's://.*$::' "$lib" | tr '\n' ' ' | grep -o 'pub use[^;]*;' | grep -o '[A-Za-z_][A-Za-z_0-9]*\|\*' | sort -u)
  outside=$(comm -23 <(printf '%s\n' "$tokens") <(printf '%s\n' "${allowed[@]}" | sort -u))
  if [ -n "$outside" ]; then
    echo "ROOT: $lib re-exports [$(echo $outside)] at the crate root; put the name in the prelude or reach it by its home path"
    fail=1
  else
    echo "root holds: $lib re-exports only the curated roots"
  fi
done

# The fork [patch] block consumers copy (docs/fork-patches.toml) is quire's own, and the checker
# still tells a differing copy from a matching one.
if ./scripts/fork-patches.sh selftest >/dev/null; then
  echo "fork patches: docs/fork-patches.toml matches the root manifest"
else
  echo "FORK PATCHES: docs/fork-patches.toml differs from the root Cargo.toml; run scripts/fork-patches.sh generate"
  fail=1
fi

# The pinned block consumers copy (docs/workspace-deps.toml) names every git dependency at the
# root manifest's revision or tag (it once lagged a toolchain bump, and later the chordkit rows lagged
# a chordkit move, and a consumer noticed both).
for crate in $(grep -E '^[A-Za-z0-9_-]+ *= *\{.*git = ' docs/workspace-deps.toml | cut -d= -f1 | tr -d ' '); do
  root=$(grep -E "^$crate *=" Cargo.toml | grep -oE '(rev|tag) = "[^"]+"' | head -1)
  doc=$(grep -E "^$crate *=" docs/workspace-deps.toml | grep -oE '(rev|tag) = "[^"]+"' | head -1)
  if [ -n "$root" ] && [ "$root" = "$doc" ]; then
    echo "pins hold: docs/workspace-deps.toml names $crate at the root manifest's $root"
  else
    echo "PINS DRIFT: docs/workspace-deps.toml has $crate ${doc:-missing}, Cargo.toml has ${root:-missing}"
    fail=1
  fi
done

exit "$fail"
