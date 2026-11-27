# Sourced by the live scripts (dev/live-smoke.sh, scripts/eval-release.sh): builds the three daemons
# the world starts as the packaged programs, and fills `daemon_env` with the `BARDO_ACCEPT_BIN_*`
# variables `agent-live` and `agent-bringup` read (`accept_kit::bin`). Run a program with them as
# `env "${daemon_env[@]}" <program> ...` (a script that clears its environment passes them on).
#
# The programs and features are those of `[suite.agent]` in the workspace's suites.toml, built into
# the same folders `cargo xtask accept agent` uses (`<target>/accept/<package>-<features>`), so the
# two share their builds. Keep the three lines in step with suites.toml.
# The workspace's target folder, as `cargo xtask` has it.
CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)/target}
daemon_env=()
build_daemon() {
  local package=$1 bin=$2 features=$3
  local dir=$CARGO_TARGET_DIR/accept/$package-${features//,/+}
  cargo build --locked --quiet -p "$package" --bin "$bin" --target-dir "$dir" --features "$features"
  local var
  var=BARDO_ACCEPT_BIN_$(printf '%s' "$bin" | tr 'a-z-' 'A-Z_')
  daemon_env+=("$var=$dir/debug/$bin")
}
build_daemon account-daemon bardo-accountd test-proc-root,test-keys
build_daemon inference-daemon bardo-inferd test-proc-root
build_daemon memory-daemon bardo-memoryd test-keys,test-proc-root
